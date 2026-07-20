//! The 10 diagnostic checks `doctor` runs, moved here unchanged from the
//! former `src/doctor.rs` (module split, no logic or output-text changes).

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::prompt;
use crate::config::write::{self, RepairOutcome, RootBreadth};
use crate::config::{self, Config};
use crate::herdr::socket::{resolve_socket_path, SocketHerdr};
use crate::herdr::Herdr;

/// One diagnostic line.
pub struct Check {
    pub ok: bool,
    pub critical: bool,
    pub label: String,
    pub detail: String,
    pub fix: Option<String>,
    /// True when the check could not run because a prerequisite failed (e.g.
    /// socket resolution). Structurally distinct from ok/fail: it never counts
    /// as a pass, and it never blocks the exit code on its own (D14).
    pub skipped: bool,
}

impl Check {
    fn ok(label: &str, detail: impl Into<String>) -> Self {
        Check {
            ok: true,
            critical: false,
            label: label.into(),
            detail: detail.into(),
            fix: None,
            skipped: false,
        }
    }
    fn fail(label: &str, detail: impl Into<String>, fix: &str, critical: bool) -> Self {
        Check {
            ok: false,
            critical,
            label: label.into(),
            detail: detail.into(),
            fix: Some(fix.into()),
            skipped: false,
        }
    }
    fn info(label: &str, detail: impl Into<String>) -> Self {
        Check {
            ok: true,
            critical: false,
            label: label.into(),
            detail: detail.into(),
            fix: None,
            skipped: false,
        }
    }
    /// A check that could not be computed because a prerequisite failed. It
    /// keeps `critical: true` (these are critical checks) but is exempted from
    /// the exit-code predicate explicitly via `skipped`, so it is never a pass
    /// on its own critical flag.
    fn skipped(label: &str, reason: impl Into<String>) -> Self {
        Check {
            ok: false,
            critical: true,
            label: label.into(),
            detail: reason.into(),
            fix: None,
            skipped: true,
        }
    }
}

/// Run every check in order and return them. A socket-resolution failure no
/// longer aborts the run (D14): it records a failing "herdr endpoint" line,
/// then the two checks that depend on the resolved socket (socket exists,
/// herdr reachable) are reported as *skipped* while every other check still
/// runs. The conditionally-absent checks (7/9 when config is None, 10 when no
/// dev service) are unchanged — only the abort is removed.
pub async fn build_checks() -> Vec<Check> {
    let mut checks: Vec<Check> = Vec::new();

    // 1. herdr binary present.
    match herdr_version() {
        Some(v) => checks.push(Check::ok("herdr binary", v)),
        None => checks.push(Check::fail(
            "herdr binary",
            "not found on PATH",
            "install herdr and ensure it is on your PATH",
            false,
        )),
    }

    // 2/3. Config: load the default if present, else note it auto-creates.
    let config_path = config::default_config_path();
    let config: Option<Config> = if config_path.exists() {
        match Config::load_file(&config_path) {
            Ok(c) => {
                checks.push(Check::ok("config", "present and valid"));
                Some(c)
            }
            Err(_) => {
                checks.push(Check::fail(
                    "config",
                    "present but invalid",
                    "fix or delete the file so a fresh default is created",
                    true,
                ));
                None
            }
        }
    } else {
        checks.push(Check::info(
            "config",
            "none yet — auto-created on first run",
        ));
        None
    };

    // Socket path from config or default. A resolution failure records a
    // failing endpoint line but no longer aborts the run (D14); the two
    // socket-dependent checks below report as skipped instead.
    let socket = config
        .as_ref()
        .map(|c| resolve_socket_path(&c.herdr_socket, &c.herdr_session))
        .unwrap_or_else(|| resolve_socket_path("", "default"));
    let socket = match socket {
        Ok(socket) => Some(socket),
        Err(e) => {
            checks.push(Check::fail(
                "herdr endpoint",
                e.to_string(),
                "fix herdr_socket or herdr_session in the config",
                true,
            ));
            None
        }
    };

    // 4. Socket file exists.
    match &socket {
        Some(socket) if socket.exists() => {
            checks.push(Check::ok("herdr endpoint", "configured endpoint exists"))
        }
        Some(_) => checks.push(Check::fail(
            "herdr endpoint",
            "configured endpoint does not exist",
            "start herdr (e.g. `herdr --session default server`)",
            true,
        )),
        None => checks.push(Check::skipped(
            "herdr endpoint",
            "skipped — socket path could not be resolved",
        )),
    }

    // 5. herdr reachable + protocol compatible.
    match &socket {
        Some(socket) => {
            let client = SocketHerdr::new(socket.clone());
            match client.ping().await {
                Ok(info) => checks.push(Check::ok(
                    "herdr reachable",
                    format!("protocol {} (v{})", info.protocol, info.server_version),
                )),
                Err(crate::herdr::HerdrError::ProtocolMismatch { expected, actual }) => checks
                    .push(Check::fail(
                        "herdr reachable",
                        format!(
                            "protocol mismatch: gateway pins {expected}, herdr reports {actual}"
                        ),
                        "upgrade herdr-go (or herdr) so the wire protocol numbers match",
                        true,
                    )),
                Err(e) => checks.push(Check::fail(
                    "herdr reachable",
                    e.to_string(),
                    "start herdr, or check the socket path in the config",
                    true,
                )),
            }
        }
        None => checks.push(Check::skipped(
            "herdr reachable",
            "skipped — socket path could not be resolved",
        )),
    }

    // 6. Web token available.
    match ensure_web_secret_readonly_impl() {
        Some(_) => checks.push(Check::ok("web token", "present; protection valid")),
        None => checks.push(Check::fail(
            "web token",
            "no HERDR_GO_WEB_SECRET and none saved",
            "set HERDR_GO_WEB_SECRET, or just run herdr-go once to auto-generate one",
            false,
        )),
    }

    // 7. allowed_roots exist.
    if let Some(c) = &config {
        let missing = c.allowed_roots.iter().filter(|p| !p.is_dir()).count();
        if missing == 0 {
            checks.push(Check::ok(
                "allowed roots",
                format!("{} root(s) exist", c.allowed_roots.len()),
            ));
        } else {
            checks.push(Check::fail(
                "allowed roots",
                format!("{missing} configured root(s) are missing"),
                "create the directory or fix allowed_roots in the config",
                false,
            ));
        }
    }

    // 8. Web UI. Always available — the binary carries an embedded copy
    // (D b300856d). An on-disk build under static_dir overrides it; its
    // absence is not a failure, just the embedded fallback in effect.
    let static_dir = config
        .as_ref()
        .map(|c| c.static_dir.clone())
        .unwrap_or_else(|| std::path::PathBuf::from("static"));
    if static_dir.join("index.html").exists() {
        checks.push(Check::ok(
            "web UI",
            format!(
                "{} (on-disk build overrides embedded)",
                static_dir.display()
            ),
        ));
    } else {
        checks.push(Check::ok("web UI", "embedded in binary"));
    }

    // 9. Bind reachability advice.
    if let Some(c) = &config {
        if c.bind_addr.ip().is_loopback() {
            checks.push(Check::info(
                "bind address",
                format!("{} — local only", c.bind_addr),
            ));
        } else {
            checks.push(Check::info(
                "bind address",
                format!("{} — reachable from other devices", c.bind_addr),
            ));
        }
    }

    // 10. Dev service state (info).
    if let Some(state) = systemd_state("herdr-go-dev.service") {
        checks.push(Check::info("dev service", state));
    }

    checks
}

fn herdr_version() -> Option<String> {
    let out = std::process::Command::new("herdr")
        .arg("--version")
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

fn systemd_state(unit: &str) -> Option<String> {
    let out = std::process::Command::new("systemctl")
        .args(["--user", "is-active", unit])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() || s == "unknown" {
        None
    } else {
        Some(s)
    }
}

/// Read-only variant of the web-token resolution used by doctor: reports whether
/// a token is available without generating one.
pub fn ensure_web_secret_readonly_impl() -> Option<String> {
    if let Ok(t) = std::env::var("HERDR_GO_WEB_SECRET") {
        if !t.trim().is_empty() {
            return Some(t);
        }
    }
    let env_path = config::config_dir().join("herdr-go.env");
    config::validate_token_protection(&env_path).ok()?;
    let text = std::fs::read_to_string(&env_path).ok()?;
    for line in text.lines() {
        if let Some(v) = line.trim().strip_prefix("HERDR_GO_WEB_SECRET=") {
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

/// Phase 2 of the interactive run (D14): walk the diagnosed checks and offer a
/// guided fix for the two check identities this cell owns — `config` and
/// `allowed roots` — matched by label, never by a bare `!ok` test, since a
/// fixable problem can present as info, fail, or (for allowed_roots) as an
/// entirely absent check. Every other check keeps printing advice only. Caller
/// gates this on [`prompt::is_interactive`]; nothing here runs otherwise.
pub(super) fn offer_fixes(
    checks: &[Check],
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    home: &Path,
    config_path: &Path,
) -> io::Result<()> {
    for check in checks {
        match check.label.as_str() {
            "config" => {
                offer_config_fix(reader, writer, home, config_path)?;
            }
            "allowed roots" => {
                offer_allowed_roots_fix(reader, writer, home, config_path)?;
            }
            "web token" => {
                offer_web_token_fix(reader, writer)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Guided fix for the `config` check identity. State is re-derived from the
/// file itself rather than trusting the phase-1 marker, which cleanly covers
/// both failure-shaped states: missing (the info line) and present-but-invalid
/// (the fail line). An empty `allowed_roots` — which fails the whole load, so
/// check 7 never appears — is repaired here, as part of the config flow.
/// Returns whether a write was applied. Every persisted path goes through the
/// validated writers in [`write`]; an invalid candidate is never written (D6).
pub(super) fn offer_config_fix(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    home: &Path,
    config_path: &Path,
) -> io::Result<bool> {
    if !config_path.exists() {
        if prompt::confirm(
            reader,
            writer,
            "config is missing — create a default now?",
            true,
        )? {
            return match config::ensure_config(config_path) {
                Ok(_) => {
                    writeln!(writer, "  created {}", config_path.display())?;
                    Ok(true)
                }
                Err(e) => {
                    writeln!(writer, "  could not create config: {e}")?;
                    Ok(false)
                }
            };
        }
        return Ok(false);
    }

    if Config::load_file(config_path).is_ok() {
        return Ok(false);
    }

    let raw = std::fs::read_to_string(config_path).unwrap_or_default();
    match write::diagnose_fields(&raw) {
        RepairOutcome::Unparseable => {
            if prompt::confirm(
                reader,
                writer,
                "config is not valid JSON — back it up and recreate a default?",
                false,
            )? {
                return match write::backup_and_recreate(config_path, &default_config_json(home)) {
                    Ok(backup) => {
                        writeln!(
                            writer,
                            "  backed up to {} and recreated a default",
                            backup.display()
                        )?;
                        Ok(true)
                    }
                    Err(e) => {
                        writeln!(writer, "  recreate failed: {e}")?;
                        Ok(false)
                    }
                };
            }
            Ok(false)
        }
        RepairOutcome::StillInvalid { invalid_fields } => {
            apply_field_repairs(reader, writer, home, config_path, &raw, &invalid_fields)
        }
        RepairOutcome::Repaired { json } => {
            // Known fields are all valid, yet the real load failed — the file
            // carries an unrecognized field (deny_unknown_fields). repair_fields
            // already dropped it; offer to save the cleaned document.
            if prompt::confirm(
                reader,
                writer,
                "config has an unrecognized field — remove it and save the cleaned config?",
                true,
            )? {
                return persist_and_report(writer, config_path, &json, "saved cleaned config");
            }
            Ok(false)
        }
    }
}

/// Prompt for each invalid field in turn and persist the field-by-field repair
/// (D7). `allowed_roots` routes through [`prompt_new_allowed_root`] so the
/// breadth guard (D9) applies; other fields take a typed line. A field left
/// blank keeps no replacement, so the repair simply stays invalid and nothing
/// is written.
fn apply_field_repairs(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    home: &Path,
    config_path: &Path,
    raw: &str,
    invalid_fields: &[String],
) -> io::Result<bool> {
    let mut replacements: HashMap<String, Value> = HashMap::new();
    for field in invalid_fields {
        if field == "allowed_roots" {
            writeln!(writer, "  allowed_roots is empty or invalid — add a root:")?;
            if let Some(root) = prompt_new_allowed_root(reader, writer, home)? {
                replacements.insert(field.clone(), json!([root.to_string_lossy()]));
            }
        } else {
            let val =
                prompt::prompt_line(reader, writer, &format!("  new value for {field}:"), None)?;
            if !val.trim().is_empty() {
                replacements.insert(field.clone(), field_json_value(field, val.trim()));
            }
        }
    }
    match write::repair_fields(raw, &replacements) {
        RepairOutcome::Repaired { json } => {
            persist_and_report(writer, config_path, &json, "saved repaired config")
        }
        _ => {
            writeln!(writer, "  config still invalid — no changes written")?;
            Ok(false)
        }
    }
}

/// Guided fix for the `allowed roots` check (a configured root missing on
/// disk). Offers to create the missing directories or to add a new root; the
/// new-root path is subject to the same breadth guard (D9). Config state is
/// re-derived from the file so this never acts on a stale marker.
pub(super) fn offer_allowed_roots_fix(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    home: &Path,
    config_path: &Path,
) -> io::Result<bool> {
    let Ok(cfg) = Config::load_file(config_path) else {
        return Ok(false);
    };
    let missing: Vec<PathBuf> = cfg
        .allowed_roots
        .iter()
        .filter(|p| !p.is_dir())
        .cloned()
        .collect();
    if missing.is_empty() {
        return Ok(false);
    }

    let choice = prompt::choose(
        reader,
        writer,
        "some allowed roots are missing on disk:",
        &[
            "create the missing directories",
            "add a new allowed root",
            "skip",
        ],
        0,
    )?;
    match choice {
        0 => {
            let mut made = 0;
            for dir in &missing {
                if std::fs::create_dir_all(dir).is_ok() {
                    writeln!(writer, "  created {}", dir.display())?;
                    made += 1;
                } else {
                    writeln!(writer, "  could not create {}", dir.display())?;
                }
            }
            Ok(made > 0)
        }
        1 => {
            let Some(root) = prompt_new_allowed_root(reader, writer, home)? else {
                return Ok(false);
            };
            let raw = std::fs::read_to_string(config_path).unwrap_or_default();
            let mut roots: Vec<String> = cfg
                .allowed_roots
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            roots.push(root.to_string_lossy().into_owned());
            let mut replacements: HashMap<String, Value> = HashMap::new();
            replacements.insert("allowed_roots".to_string(), json!(roots));
            match write::repair_fields(&raw, &replacements) {
                RepairOutcome::Repaired { json } => persist_and_report(
                    writer,
                    config_path,
                    &json,
                    &format!("added {}", root.display()),
                ),
                _ => {
                    writeln!(writer, "  could not update config — no changes written")?;
                    Ok(false)
                }
            }
        }
        _ => Ok(false),
    }
}

/// Guided fix for the `web token` check: when no web session secret is
/// available, offer to generate and persist a protected one through the real
/// creation/repair path [`config::ensure_web_secret`] (mod.rs) — never the
/// read-only [`ensure_web_secret_readonly_impl`], which only diagnoses and
/// creates nothing. Availability is re-derived from the token state itself, not
/// the phase-1 marker, so the offer never appears when a token already resolves.
pub(super) fn offer_web_token_fix(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> io::Result<bool> {
    offer_web_token_fix_with(
        reader,
        writer,
        ensure_web_secret_readonly_impl().is_some(),
        config::ensure_web_secret,
    )
}

/// Testable core of [`offer_web_token_fix`]: `already_available` is the
/// re-derived token state and `create` is the real creation path (injected so
/// the flow is unit-testable without touching the process environment or the
/// real config directory). The generated secret is returned by `create` but is
/// never echoed, logged, or otherwise rendered — only the created/existed
/// outcome is reported.
fn offer_web_token_fix_with<F>(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    already_available: bool,
    create: F,
) -> io::Result<bool>
where
    F: FnOnce() -> io::Result<(String, bool)>,
{
    if already_available {
        return Ok(false);
    }
    if !prompt::confirm(
        reader,
        writer,
        "no web login token is set — generate a protected one now?",
        true,
    )? {
        return Ok(false);
    }
    match create() {
        // The returned secret is deliberately discarded: only the outcome is
        // reported, never the value (D6/D13).
        Ok((_secret, generated)) => {
            let msg = if generated {
                "generated a protected web login token"
            } else {
                "a web login token is already available"
            };
            writeln!(writer, "  {msg}")?;
            Ok(generated)
        }
        Err(e) => {
            writeln!(writer, "  could not create web token: {e}")?;
            Ok(false)
        }
    }
}

/// Prompt for a new `allowed_roots` entry and return it only once the breadth
/// guard (D9) is satisfied: a filesystem root, the home directory, or a symlink
/// each demands an explicit typed confirmation that names the breadth kind; a
/// narrow directory is accepted directly. Empty input, or a declined
/// confirmation, returns `None`. `home` is resolved by the caller from the
/// native per-user profile logic — never reimplemented here. Reused by the
/// settings editor in the next cell, hence `pub(crate)`.
pub(crate) fn prompt_new_allowed_root(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    home: &Path,
) -> io::Result<Option<PathBuf>> {
    let input = prompt::prompt_line(reader, writer, "  absolute path to allow:", None)?;
    let input = input.trim();
    if input.is_empty() {
        return Ok(None);
    }
    let candidate = PathBuf::from(input);
    match write::classify_root_breadth(&candidate, home) {
        RootBreadth::Narrow => Ok(Some(candidate)),
        breadth => {
            let kind = match breadth {
                RootBreadth::FilesystemRoot => "a filesystem root — grants the ENTIRE filesystem",
                RootBreadth::HomeDirectory => "your home directory — grants every file you own",
                RootBreadth::Symlink => {
                    "a symlink — its real target may be far broader than it looks"
                }
                RootBreadth::Narrow => unreachable!("narrow handled above"),
            };
            writeln!(writer, "  ⚠ {} is {kind}.", candidate.display())?;
            if prompt::typed_confirm(
                reader,
                writer,
                "  this widens the file-access boundary",
                "widen",
            )? {
                Ok(Some(candidate))
            } else {
                writeln!(writer, "  not added.")?;
                Ok(None)
            }
        }
    }
}

pub(super) fn persist_and_report(
    writer: &mut impl Write,
    config_path: &Path,
    json: &str,
    ok_msg: &str,
) -> io::Result<bool> {
    match write::persist_validated(config_path, json) {
        Ok(()) => {
            writeln!(writer, "  {ok_msg}")?;
            Ok(true)
        }
        Err(e) => {
            writeln!(writer, "  could not persist: {e}")?;
            Ok(false)
        }
    }
}

/// Convert a typed line into the JSON value shape the field validator expects:
/// the two numeric fields parse as numbers (a non-number stays a string and is
/// rejected by validation, so nothing invalid is ever written), everything else
/// is a string. `allowed_roots` never reaches here — it is handled separately.
pub(super) fn field_json_value(field: &str, input: &str) -> Value {
    match field {
        "poll_interval_ms" | "herdr_protocol" => match input.parse::<u64>() {
            Ok(n) => json!(n),
            Err(_) => json!(input),
        },
        _ => json!(input),
    }
}

/// A fresh default config document for the unparseable-recreate path (D7),
/// mirroring [`config::ensure_config`]'s defaults but as a string for
/// [`write::backup_and_recreate`]. `home` supplies the same `~/projects`-or-`~`
/// allowed root ensure_config would pick.
fn default_config_json(home: &Path) -> String {
    let projects = home.join("projects");
    let root = if projects.is_dir() {
        projects
    } else {
        home.to_path_buf()
    };
    format!(
        "{{\n  \"bind_addr\": \"0.0.0.0:8787\",\n  \"herdr_session\": \"default\",\n  \
         \"allowed_roots\": [{:?}],\n  \"poll_interval_ms\": 500,\n  \
         \"herdr_protocol\": 16,\n  \"static_dir\": \"static\"\n}}\n",
        root.to_string_lossy()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_constructors_carry_intent() {
        let ok = Check::ok("x", "fine");
        assert!(ok.ok && ok.fix.is_none());
        let bad = Check::fail("y", "broken", "do z", true);
        assert!(!bad.ok && bad.critical && bad.fix.as_deref() == Some("do z"));
    }

    #[test]
    fn web_token_readonly_prefers_env() {
        std::env::set_var("HERDR_GO_WEB_SECRET", "tok-abc");
        assert_eq!(
            ensure_web_secret_readonly_impl().as_deref(),
            Some("tok-abc")
        );
        std::env::remove_var("HERDR_GO_WEB_SECRET");
    }

    #[test]
    fn skipped_check_is_distinct_from_ok_and_fail() {
        let s = Check::skipped("herdr endpoint", "socket path could not be resolved");
        assert!(s.skipped, "skipped flag set");
        assert!(!s.ok, "a skipped check is not a pass");
        assert!(s.fix.is_none());
    }

    fn reader(input: &str) -> std::io::Cursor<Vec<u8>> {
        std::io::Cursor::new(input.as_bytes().to_vec())
    }

    #[test]
    fn new_allowed_root_narrow_needs_no_confirmation() {
        let mut r = reader("/opt/data\n");
        let mut w = Vec::new();
        let home = Path::new("/home/tester");
        let got = prompt_new_allowed_root(&mut r, &mut w, home).unwrap();
        assert_eq!(got, Some(PathBuf::from("/opt/data")));
    }

    #[test]
    fn new_allowed_root_home_requires_typed_confirmation() {
        // Home directory candidate: accepted only when the user types "widen".
        let mut r = reader("/home/tester\nwiden\n");
        let mut w = Vec::new();
        let home = Path::new("/home/tester");
        let got = prompt_new_allowed_root(&mut r, &mut w, home).unwrap();
        assert_eq!(got, Some(PathBuf::from("/home/tester")));

        // Same candidate, wrong confirmation phrase -> rejected.
        let mut r = reader("/home/tester\nno\n");
        let mut w = Vec::new();
        let got = prompt_new_allowed_root(&mut r, &mut w, home).unwrap();
        assert_eq!(got, None);
    }

    #[test]
    fn new_allowed_root_filesystem_root_requires_typed_confirmation() {
        let mut r = reader("/\nno\n");
        let mut w = Vec::new();
        let home = Path::new("/home/tester");
        let got = prompt_new_allowed_root(&mut r, &mut w, home).unwrap();
        assert_eq!(got, None, "'/' must not be added without confirmation");
    }

    #[test]
    fn web_token_fix_skips_when_a_token_already_resolves() {
        let mut r = reader("y\n");
        let mut w = Vec::new();
        let applied = offer_web_token_fix_with(&mut r, &mut w, true, || {
            panic!("create must never run when a token is already available")
        })
        .unwrap();
        assert!(!applied, "no fix applied when a token already resolves");
        assert!(String::from_utf8(w).unwrap().is_empty(), "no prompt shown");
    }

    #[test]
    fn web_token_fix_generates_via_the_real_creation_path_without_leaking_the_value() {
        let mut r = reader("y\n");
        let mut w = Vec::new();
        let applied = offer_web_token_fix_with(&mut r, &mut w, false, || {
            Ok(("super-secret-token-value".to_string(), true))
        })
        .unwrap();
        assert!(applied, "a token was generated");
        let out = String::from_utf8(w).unwrap();
        assert!(out.contains("generated a protected web login token"));
        assert!(
            !out.contains("super-secret-token-value"),
            "the secret value must never appear in output"
        );
    }

    #[test]
    fn web_token_fix_declined_writes_nothing() {
        let mut r = reader("n\n");
        let mut w = Vec::new();
        let applied = offer_web_token_fix_with(&mut r, &mut w, false, || {
            panic!("create must never run when the user declines")
        })
        .unwrap();
        assert!(!applied, "declining applies no fix");
    }

    #[test]
    fn web_token_fix_reports_a_creation_error() {
        let mut r = reader("y\n");
        let mut w = Vec::new();
        let applied = offer_web_token_fix_with(&mut r, &mut w, false, || {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "no dir"))
        })
        .unwrap();
        assert!(!applied, "a failed creation is not an applied fix");
        assert!(String::from_utf8(w)
            .unwrap()
            .contains("could not create web token"));
    }

    #[test]
    fn config_fix_creates_missing_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("config.json");
        let mut r = reader("y\n");
        let mut w = Vec::new();
        let home = Path::new("/home/tester");
        let applied = offer_config_fix(&mut r, &mut w, home, &path).unwrap();
        assert!(applied, "a create was applied");
        assert!(path.exists());
        assert!(Config::load_file(&path).is_ok(), "created config is valid");
    }

    #[test]
    fn config_fix_repairs_empty_allowed_roots_via_added_root() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"herdr_session":"g","allowed_roots":[]}"#).unwrap();
        assert!(Config::load_file(&path).is_err(), "starts invalid");

        // Field-by-field repair prompts for the one invalid field (allowed_roots);
        // a narrow path needs no extra confirmation.
        let mut r = reader("/opt/data\n");
        let mut w = Vec::new();
        let home = Path::new("/home/tester");
        let applied = offer_config_fix(&mut r, &mut w, home, &path).unwrap();
        assert!(applied, "a repair was applied");
        let cfg = Config::load_file(&path).expect("config now valid");
        assert_eq!(cfg.allowed_roots, vec![PathBuf::from("/opt/data")]);
        assert_eq!(cfg.herdr_session, "g", "valid field preserved");
    }
}

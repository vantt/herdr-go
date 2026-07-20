//! End-of-run settings editor (D2, D6, D16, D17).
//!
//! Reached through a single end-of-run "edit a setting?" prompt (default no) —
//! never a flag or a subcommand (D17). The operator picks one of the 8
//! `config.json` fields or one of the 3 env-only secrets from a menu, edits it,
//! and returns to the menu until they choose to stop. Config edits are always
//! routed through [`write::persist_validated`] so an invalid candidate is
//! rejected and the existing file is left untouched (D6); secret edits go
//! through the masked prompt ([`prompt::prompt_secret`]) and the owner-only
//! writer ([`crate::config::secrets::write_secret`]) — never a plaintext prompt
//! or a hand-rolled env writer (D2/D10/D13). Nothing here is reachable without a
//! TTY: the caller gates the whole flow on [`prompt::is_interactive`] and skips
//! it entirely under `--check` or a pipe (D5/D15).

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::net::SocketAddr;
use std::path::Path;

use serde_json::{json, Value};

use super::checks;
use super::prompt::{self, SecretEntry};
use crate::config::write::{self, RepairOutcome};
use crate::config::{self, Config};

/// The single end-of-run entry point (D17): ask once whether to edit a setting,
/// defaulting to no, and only then open the editor menu. A "no" (the default)
/// returns without touching anything.
pub(super) fn maybe_edit(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    home: &Path,
    config_path: &Path,
) -> io::Result<()> {
    if !prompt::confirm(reader, writer, "edit a setting?", false)? {
        return Ok(());
    }
    run_editor(reader, writer, home, config_path)
}

/// Menu loop: list every editable setting (all 8 config fields plus the 3
/// secrets) plus a final "stop", and dispatch the chosen one until the operator
/// stops.
fn run_editor(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    home: &Path,
    config_path: &Path,
) -> io::Result<()> {
    // Build the menu from CONFIG_FIELDS so it never drifts from the schema, then
    // the 3 env-only secrets and a final "stop".
    let mut options: Vec<&str> = write::CONFIG_FIELDS.to_vec();
    options.push("web login token (secret)");
    options.push("github token (secret)");
    options.push("telegram bot token (secret)");
    options.push("stop editing");
    let field_count = write::CONFIG_FIELDS.len();
    let done = options.len() - 1;
    loop {
        let choice = prompt::choose(reader, writer, "edit which setting?", &options, done)?;
        if choice < field_count {
            edit_config_field(
                reader,
                writer,
                home,
                config_path,
                write::CONFIG_FIELDS[choice],
            )?;
        } else if choice == field_count {
            edit_secret(writer, "HERDR_GO_WEB_SECRET", "web login token")?;
        } else if choice == field_count + 1 {
            edit_secret(writer, "HERDR_GO_GITHUB_TOKEN", "github token")?;
        } else if choice == field_count + 2 {
            edit_secret(writer, "HERDR_GO_TELEGRAM_TOKEN", "telegram bot token")?;
        } else {
            return Ok(());
        }
    }
}

/// Edit one `config.json` field. `allowed_roots` routes through the shared
/// breadth guard; `bind_addr` fires the non-loopback warning at edit time
/// (D16); every write goes through [`persist_field`] and thus
/// [`write::persist_validated`].
fn edit_config_field(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    home: &Path,
    config_path: &Path,
    field: &str,
) -> io::Result<()> {
    if field == "allowed_roots" {
        return edit_allowed_roots(reader, writer, home, config_path);
    }
    let raw = std::fs::read_to_string(config_path).unwrap_or_default();
    let current = current_config_value(&raw, field);
    let input = prompt::prompt_line(
        reader,
        writer,
        &format!("  new value for {field} (current: {current}; empty to keep):"),
        None,
    )?;
    let input = input.trim();
    if input.is_empty() {
        writeln!(writer, "  {field} unchanged")?;
        return Ok(());
    }
    // D16: changing bind_addr to a non-loopback address widens exposure — warn
    // at edit time, before the write, with the same notice the startup path
    // shows.
    if field == "bind_addr" {
        if let Ok(addr) = input.parse::<SocketAddr>() {
            if !addr.ip().is_loopback() {
                write!(writer, "{}", super::non_loopback_bind_warning(&addr))?;
            }
        }
    }
    let mut replacements: HashMap<String, Value> = HashMap::new();
    replacements.insert(field.to_string(), checks::field_json_value(field, input));
    persist_field(writer, config_path, &raw, &replacements, field)
}

/// Add a new `allowed_roots` entry, reusing doctor-config-surface-4's shared
/// breadth-classification + typed-confirmation prompt
/// ([`checks::prompt_new_allowed_root`]) rather than a second implementation.
fn edit_allowed_roots(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    home: &Path,
    config_path: &Path,
) -> io::Result<()> {
    let Ok(cfg) = Config::load_file(config_path) else {
        writeln!(
            writer,
            "  config.json must be valid before editing allowed_roots — nothing changed"
        )?;
        return Ok(());
    };
    writeln!(
        writer,
        "  {} allowed root(s) configured — adding one:",
        cfg.allowed_roots.len()
    )?;
    let Some(root) = checks::prompt_new_allowed_root(reader, writer, home)? else {
        writeln!(writer, "  allowed_roots unchanged")?;
        return Ok(());
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
    persist_field(writer, config_path, &raw, &replacements, "allowed_roots")
}

/// Set one env-only secret with a masked prompt, writing it through the
/// owner-only env writer. The value is never echoed, logged, or persisted in
/// plaintext output; only the non-revealing [`SecretEntry::display`] is shown.
fn edit_secret(writer: &mut impl Write, key: &str, label: &str) -> io::Result<()> {
    let entry = prompt::prompt_secret(writer, &format!("  new {label}:"))?;
    match &entry {
        SecretEntry::Skipped => {
            writeln!(writer, "  {label} unchanged")?;
        }
        SecretEntry::Value(value) => match config::secrets::write_secret(key, value) {
            // Report the non-revealing display only — never the value (D6/D13).
            Ok(()) => writeln!(writer, "  saved {label} ({})", entry.display())?,
            Err(e) => writeln!(writer, "  could not save {label}: {e}")?,
        },
    }
    Ok(())
}

/// Merge `replacements` into the existing document field-by-field and persist
/// through [`write::persist_validated`]. An invalid result leaves the file
/// untouched (D6).
fn persist_field(
    writer: &mut impl Write,
    config_path: &Path,
    raw: &str,
    replacements: &HashMap<String, Value>,
    label: &str,
) -> io::Result<()> {
    match write::repair_fields(raw, replacements) {
        RepairOutcome::Repaired { json } => {
            checks::persist_and_report(writer, config_path, &json, &format!("saved {label}"))?;
        }
        RepairOutcome::StillInvalid { .. } => {
            writeln!(writer, "  {label} value is invalid — nothing was written")?;
        }
        RepairOutcome::Unparseable => {
            writeln!(
                writer,
                "  config.json is not valid JSON — repair it first; nothing was written"
            )?;
        }
    }
    Ok(())
}

/// The current on-disk value of `field` for display, or `unset` when absent.
/// Never called for secrets, so it never risks revealing one.
fn current_config_value(raw: &str, field: &str) -> String {
    serde_json::from_str::<Value>(raw)
        .ok()
        .and_then(|v| v.get(field).cloned())
        .map(|v| match v {
            Value::String(s) => s,
            other => other.to_string(),
        })
        .unwrap_or_else(|| "unset".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn reader(input: &str) -> io::Cursor<Vec<u8>> {
        io::Cursor::new(input.as_bytes().to_vec())
    }

    #[cfg(windows)]
    const HOME: &str = r"C:\Users\tester";
    #[cfg(not(windows))]
    const HOME: &str = "/home/tester";

    #[cfg(windows)]
    const ROOT_A: &str = r"C:\data";
    #[cfg(not(windows))]
    const ROOT_A: &str = "/opt/data";

    #[cfg(windows)]
    const ROOT_B: &str = r"C:\media";
    #[cfg(not(windows))]
    const ROOT_B: &str = "/srv/media";

    fn valid_config(dir: &Path) -> PathBuf {
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            format!(
                r#"{{"bind_addr":"127.0.0.1:8787","herdr_session":"orig","allowed_roots":[{ROOT_A:?}],"poll_interval_ms":500,"herdr_protocol":16,"static_dir":"static"}}"#,
            ),
        )
        .unwrap();
        path
    }

    #[test]
    fn declining_the_edit_prompt_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = valid_config(dir.path());
        let before = std::fs::read_to_string(&path).unwrap();
        // The single end-of-run prompt defaults to no; an empty answer is "no".
        let mut r = reader("\n");
        let mut w = Vec::new();
        maybe_edit(&mut r, &mut w, Path::new(HOME), &path).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    }

    #[test]
    fn editing_a_plain_field_persists_and_preserves_the_others() {
        let dir = tempfile::tempdir().unwrap();
        let path = valid_config(dir.path());
        // choose herdr_session (menu #2), set it, then choose "stop".
        let mut r = reader("2\nrenamed\n12\n");
        let mut w = Vec::new();
        run_editor(&mut r, &mut w, Path::new(HOME), &path).unwrap();
        let cfg = Config::load_file(&path).expect("config still valid");
        assert_eq!(cfg.herdr_session, "renamed");
        assert_eq!(
            cfg.allowed_roots,
            vec![PathBuf::from(ROOT_A)],
            "unrelated field preserved"
        );
    }

    #[test]
    fn an_invalid_field_value_leaves_the_file_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = valid_config(dir.path());
        let before = std::fs::read_to_string(&path).unwrap();
        // choose bind_addr (menu #1), give a non-address, then stop.
        let mut r = reader("1\nnot-an-addr\n12\n");
        let mut w = Vec::new();
        run_editor(&mut r, &mut w, Path::new(HOME), &path).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            before,
            "a rejected candidate must never touch the file"
        );
    }

    #[test]
    fn changing_bind_addr_to_non_loopback_warns_at_edit_time() {
        let dir = tempfile::tempdir().unwrap();
        let path = valid_config(dir.path());
        let mut r = reader("1\n0.0.0.0:9000\n12\n");
        let mut w = Vec::new();
        run_editor(&mut r, &mut w, Path::new(HOME), &path).unwrap();
        let out = String::from_utf8(w).unwrap();
        assert!(
            out.contains("reachable beyond this machine"),
            "the non-loopback warning must fire at edit time"
        );
        let cfg = Config::load_file(&path).unwrap();
        assert_eq!(cfg.bind_addr.to_string(), "0.0.0.0:9000");
    }

    #[test]
    fn changing_bind_addr_to_loopback_does_not_warn() {
        let dir = tempfile::tempdir().unwrap();
        let path = valid_config(dir.path());
        let mut r = reader("1\n127.0.0.1:9001\n12\n");
        let mut w = Vec::new();
        run_editor(&mut r, &mut w, Path::new(HOME), &path).unwrap();
        let out = String::from_utf8(w).unwrap();
        assert!(
            !out.contains("reachable beyond this machine"),
            "a loopback bind must not warn"
        );
    }

    #[test]
    fn adding_a_narrow_allowed_root_appends_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = valid_config(dir.path());
        // choose allowed_roots (menu #3), add a narrow path, then stop.
        let mut r = reader(&format!("3\n{ROOT_B}\n12\n"));
        let mut w = Vec::new();
        run_editor(&mut r, &mut w, Path::new(HOME), &path).unwrap();
        let cfg = Config::load_file(&path).unwrap();
        assert_eq!(
            cfg.allowed_roots,
            vec![PathBuf::from(ROOT_A), PathBuf::from(ROOT_B)]
        );
    }

    #[test]
    fn adding_an_over_broad_allowed_root_without_confirmation_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = valid_config(dir.path());
        // choose allowed_roots (menu #3), offer "/", decline the typed confirm.
        let mut r = reader("3\n/\nno\n12\n");
        let mut w = Vec::new();
        run_editor(&mut r, &mut w, Path::new(HOME), &path).unwrap();
        let cfg = Config::load_file(&path).unwrap();
        assert_eq!(
            cfg.allowed_roots,
            vec![PathBuf::from(ROOT_A)],
            "'/' must not be added without the typed confirmation"
        );
    }

    #[test]
    fn the_menu_exposes_all_eight_fields_and_three_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let path = valid_config(dir.path());
        // choose "stop" immediately; the menu is still printed once.
        let mut r = reader("12\n");
        let mut w = Vec::new();
        run_editor(&mut r, &mut w, Path::new(HOME), &path).unwrap();
        let out = String::from_utf8(w).unwrap();
        for field in write::CONFIG_FIELDS {
            assert!(out.contains(field), "menu lists {field}");
        }
        assert!(out.contains("web login token"), "menu lists the web secret");
        assert!(out.contains("github token"), "menu lists the github secret");
        assert!(
            out.contains("telegram bot token"),
            "menu lists the telegram secret"
        );
    }
}

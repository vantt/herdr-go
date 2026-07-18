//! `herdctl doctor` — a read-only environment check that diagnoses setup
//! problems and prints a one-line fix for each. It never mutates anything.

use crate::config::{self, Config};
use crate::herdr::socket::{default_socket_path, SocketHerdr};
use crate::herdr::Herdr;

/// One diagnostic line.
pub struct Check {
    pub ok: bool,
    pub critical: bool,
    pub label: String,
    pub detail: String,
    pub fix: Option<String>,
}

impl Check {
    fn ok(label: &str, detail: impl Into<String>) -> Self {
        Check {
            ok: true,
            critical: false,
            label: label.into(),
            detail: detail.into(),
            fix: None,
        }
    }
    fn fail(label: &str, detail: impl Into<String>, fix: &str, critical: bool) -> Self {
        Check {
            ok: false,
            critical,
            label: label.into(),
            detail: detail.into(),
            fix: Some(fix.into()),
        }
    }
    fn info(label: &str, detail: impl Into<String>) -> Self {
        Check {
            ok: true,
            critical: false,
            label: label.into(),
            detail: detail.into(),
            fix: None,
        }
    }
}

/// Run all checks, print the report, and return `true` if every **critical**
/// check passed.
pub async fn run() -> bool {
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
                checks.push(Check::ok("config", config_path.display().to_string()));
                Some(c)
            }
            Err(e) => {
                checks.push(Check::fail(
                    "config",
                    format!("{} is invalid: {e}", config_path.display()),
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

    // Socket path from config or default.
    let socket = config
        .as_ref()
        .filter(|c| !c.herdr_socket.is_empty())
        .map(|c| std::path::PathBuf::from(&c.herdr_socket))
        .unwrap_or_else(default_socket_path);

    // 4. Socket file exists.
    if socket.exists() {
        checks.push(Check::ok("herdr socket", socket.display().to_string()));
    } else {
        checks.push(Check::fail(
            "herdr socket",
            format!("{} does not exist", socket.display()),
            "start herdr (e.g. `herdr --session default server`)",
            true,
        ));
    }

    // 5. herdr reachable + protocol compatible.
    let client = SocketHerdr::new(socket.clone());
    match client.ping().await {
        Ok(info) => checks.push(Check::ok(
            "herdr reachable",
            format!("protocol {} (v{})", info.protocol, info.server_version),
        )),
        Err(crate::herdr::HerdrError::ProtocolMismatch { expected, actual }) => {
            checks.push(Check::fail(
                "herdr reachable",
                format!("protocol mismatch: gateway pins {expected}, herdr reports {actual}"),
                "upgrade herdctl (or herdr) so the wire protocol numbers match",
                true,
            ))
        }
        Err(e) => checks.push(Check::fail(
            "herdr reachable",
            e.to_string(),
            "start herdr, or check the socket path in the config",
            true,
        )),
    }

    // 6. Web token available.
    match ensure_web_secret_readonly_impl() {
        Some(_) => checks.push(Check::ok("web token", "set")),
        None => checks.push(Check::fail(
            "web token",
            "no HERDCTL_WEB_SECRET and none saved",
            "set HERDCTL_WEB_SECRET, or just run herdctl once to auto-generate one",
            false,
        )),
    }

    // 7. allowed_roots exist.
    if let Some(c) = &config {
        let missing: Vec<_> = c
            .allowed_roots
            .iter()
            .filter(|p| !p.is_dir())
            .map(|p| p.display().to_string())
            .collect();
        if missing.is_empty() {
            checks.push(Check::ok(
                "allowed roots",
                format!("{} root(s) exist", c.allowed_roots.len()),
            ));
        } else {
            checks.push(Check::fail(
                "allowed roots",
                format!("missing: {}", missing.join(", ")),
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
    if let Some(state) = systemd_state("herdr-gateway-dev.service") {
        checks.push(Check::info("dev service", state));
    }

    print_report(&checks);
    checks.iter().all(|c| c.ok || !c.critical)
}

fn print_report(checks: &[Check]) {
    let _ = std::io::Write::flush(&mut std::io::stdout());
    println!("\n  herdctl doctor\n  ─────────────");
    for c in checks {
        let mark = if c.ok { "✓" } else { "✗" };
        println!("  {mark} {:<16} {}", c.label, c.detail);
        if let Some(fix) = &c.fix {
            println!("      → {fix}");
        }
    }
    let problems = checks.iter().filter(|c| !c.ok).count();
    let critical = checks.iter().filter(|c| !c.ok && c.critical).count();
    println!();
    if critical > 0 {
        println!("  {critical} blocking problem(s) — fix the ✗ lines above.\n");
    } else if problems > 0 {
        println!("  {problems} non-blocking note(s); the gateway can still run.\n");
    } else {
        println!("  All good — you're ready to run herdctl.\n");
    }
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
    if let Ok(t) = std::env::var("HERDCTL_WEB_SECRET") {
        if !t.trim().is_empty() {
            return Some(t);
        }
    }
    let env_path = config::config_dir().join("herdctl.env");
    let text = std::fs::read_to_string(&env_path).ok()?;
    for line in text.lines() {
        if let Some(v) = line.trim().strip_prefix("HERDCTL_WEB_SECRET=") {
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
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
        std::env::set_var("HERDCTL_WEB_SECRET", "tok-abc");
        assert_eq!(
            ensure_web_secret_readonly_impl().as_deref(),
            Some("tok-abc")
        );
        std::env::remove_var("HERDCTL_WEB_SECRET");
    }
}

//! `herdr-go doctor` — diagnoses setup problems and, in an interactive
//! terminal, offers a guided fix for each fixable one (D1). With `--check`, or
//! when stdin/stderr is not a terminal, it stays strictly read-only (D5/D15).

mod checks;
mod edit;
pub mod prompt;

pub use checks::{run_service_command, Check};

/// The security notice shown when `bind_addr` is not a loopback address: the
/// gateway is reachable beyond this machine, so the web login token becomes the
/// only boundary. Shared by the startup path (`main.rs`) and doctor's settings
/// editor (D16) so the wording is a single source of truth and never drifts.
pub fn non_loopback_bind_warning(addr: &std::net::SocketAddr) -> String {
    format!(
        "\n  ⚠ Listening on {addr} — reachable beyond this machine.\n    \
         herdr has no auth of its own, so the web login token is the only gate.\n    \
         Prefer a Tailscale/tailnet address, and put TLS (reverse proxy) in front\n    \
         if this is a shared LAN or the internet.\n"
    )
}

/// Diagnose the environment in up to three phases (D14). Phase 1 always runs:
/// diagnose every check and print the report. In an interactive terminal, and
/// only when `check_only` is false, phase 2 offers a guided fix for each
/// fixable problem and phase 3 re-runs every check once so the exit code
/// reflects the final, post-fix state (D11). With `check_only`, or when
/// stdin/stderr is not a terminal, phases 2 and 3 never run: one read-only
/// pass, no prompts, no writes (D5/D15, `--check` taking precedence over TTY
/// detection). Returns `true` when every critical check in the last pass that
/// ran passed.
pub async fn run(check_only: bool) -> bool {
    let checks = checks::build_checks().await;
    print_report(&checks);

    if check_only || !prompt::is_interactive() {
        return all_ok(&checks);
    }

    {
        use std::io::Write;
        let stdin = std::io::stdin();
        let mut reader = stdin.lock();
        let stderr = std::io::stderr();
        let mut writer = stderr.lock();
        let home = crate::config::home();
        let config_path = crate::config::default_config_path();
        if let Err(e) = checks::offer_fixes(&checks, &mut reader, &mut writer, &home, &config_path)
        {
            let _ = writeln!(writer, "  fix step failed: {e}");
        }
    }

    // Phase 3: one full re-run, unconditionally (whether or not a fix was
    // applied), so derived state (e.g. the socket, which a config fix
    // invalidates) is recomputed and the exit code is the final state.
    let final_checks = checks::build_checks().await;
    print_report(&final_checks);
    let ok = all_ok(&final_checks);

    // Phase 4: a single optional end-of-run settings editor (D17), reached only
    // in interactive mode (the guard above already returned for --check and
    // non-interactive). It does not change the exit code, which already reflects
    // the post-fix diagnostic state (D11).
    {
        use std::io::Write;
        let stdin = std::io::stdin();
        let mut reader = stdin.lock();
        let stderr = std::io::stderr();
        let mut writer = stderr.lock();
        let home = crate::config::home();
        let config_path = crate::config::default_config_path();
        if let Err(e) = edit::maybe_edit(&mut reader, &mut writer, &home, &config_path) {
            let _ = writeln!(writer, "  edit step failed: {e}");
        }
    }

    ok
}

/// The exit-code predicate: every critical check must have passed. A skipped
/// check is exempt explicitly (never a pass on its own critical flag, never a
/// blocker) per D14; a non-critical failure never blocks.
fn all_ok(checks: &[Check]) -> bool {
    checks.iter().all(|c| c.ok || c.skipped || !c.critical)
}

fn print_report(checks: &[Check]) {
    let _ = render_report(checks, &mut std::io::stdout());
}

fn render_report(checks: &[Check], w: &mut impl std::io::Write) -> std::io::Result<()> {
    w.flush()?;
    writeln!(w, "\n  herdr-go doctor\n  ─────────────")?;
    for c in checks {
        let mark = if c.skipped {
            "○"
        } else if c.ok {
            "✓"
        } else {
            "✗"
        };
        writeln!(w, "  {mark} {:<16} {}", c.label, c.detail)?;
        if let Some(fix) = &c.fix {
            writeln!(w, "      → {fix}")?;
        }
    }
    let problems = checks.iter().filter(|c| !c.ok && !c.skipped).count();
    let critical = checks
        .iter()
        .filter(|c| !c.ok && !c.skipped && c.critical)
        .count();
    let skipped = checks.iter().filter(|c| c.skipped).count();
    writeln!(w)?;
    if critical > 0 {
        writeln!(
            w,
            "  {critical} blocking problem(s) — fix the ✗ lines above.\n"
        )?;
    } else if problems > 0 {
        writeln!(
            w,
            "  {problems} non-blocking note(s); the gateway can still run.\n"
        )?;
    } else {
        writeln!(w, "  All good — you're ready to run herdr-go.\n")?;
    }
    if skipped > 0 {
        writeln!(
            w,
            "  {skipped} check(s) skipped — a prerequisite failed (see ○ above).\n"
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make(ok: bool, critical: bool, skipped: bool) -> Check {
        Check {
            ok,
            critical,
            label: "x".into(),
            detail: "d".into(),
            fix: None,
            skipped,
        }
    }

    #[test]
    fn all_ok_treats_a_skipped_critical_check_as_non_blocking() {
        assert!(all_ok(&[make(false, true, true)]), "skipped never blocks");
        assert!(
            !all_ok(&[make(false, true, false)]),
            "a real critical fail blocks"
        );
        assert!(
            all_ok(&[make(false, false, false)]),
            "non-critical fail never blocks"
        );
    }

    #[test]
    fn render_report_marks_skipped_distinctly() {
        let mut out = Vec::new();
        render_report(&[make(false, true, true)], &mut out).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains('○'), "skipped uses its own marker");
        assert!(
            s.contains("skipped"),
            "skipped is called out in the summary"
        );
    }
}

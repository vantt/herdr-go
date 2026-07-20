//! `herdr-go doctor` — a read-only environment check that diagnoses setup
//! problems and prints a one-line fix for each. It never mutates anything.

mod checks;
pub mod prompt;

pub use checks::Check;

/// Run all checks, print the report, and return `true` if every **critical**
/// check passed.
pub async fn run() -> bool {
    let checks = checks::build_checks().await;
    print_report(&checks);
    checks.iter().all(|c| c.ok || !c.critical)
}

fn print_report(checks: &[Check]) {
    let _ = std::io::Write::flush(&mut std::io::stdout());
    println!("\n  herdr-go doctor\n  ─────────────");
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
        println!("  All good — you're ready to run herdr-go.\n");
    }
}

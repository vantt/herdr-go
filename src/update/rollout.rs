//! EP5: the live-service update rollout (D3/D4/D9) — the single highest-risk
//! composition in this feature. Against a running service it performs, in
//! order: stop, swap the running binary, self-exec the newly-swapped binary to
//! merge config under the NEW version's own compiled defaults, start, poll
//! `/api/health` with a bounded budget, and — only if health never comes up —
//! roll back both the binary and the config and start again.
//!
//! Why this exact order (see approach.md's "EP5 design correction"): the update
//! process keeps running the OLD binary right up until the restart, so merging
//! before the swap would read the OLD binary's compiled defaults (violating
//! D5). Swapping first, then self-exec'ing the new binary with the hidden
//! `--internal-merge-config` verb, makes the merge run under the new defaults.
//!
//! This file lives inside the library crate, so every internal reference uses
//! `crate::`/`super::`, never the binary-crate self-path form.
//!
//! KNOWN LIMITATION (Windows): replacing a running executable's image file
//! while the process still holds it is expected to fail — the OS locks the
//! image. This is a new, not-yet-proven-safe path, distinct from the
//! pre-existing "Windows service restart unproven" gap. Real cross-platform
//! proof is EP6's smoke test, not this cell.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// Health-check retry budget: 10 attempts, 1 second apart (~10 seconds total).
const HEALTH_ATTEMPTS: usize = 10;
const HEALTH_INTERVAL: Duration = Duration::from_secs(1);

/// Every way [`perform_update`] can fail after it has begun touching the
/// running service.
#[derive(Debug, thiserror::Error)]
pub enum RolloutError {
    /// The binary swap itself failed. The on-disk binary was never replaced
    /// (write-then-rename leaves the original in place), and a start was
    /// attempted to bring the unchanged service back.
    #[error("failed to swap the binary: {0}")]
    Swap(super::swap::SwapError),
    /// The post-start health check never succeeded within the budget, so the
    /// previous binary and config were restored and the service was started
    /// again. `restart_exit_code` is that rollback-restart's exit code, so a
    /// caller can tell whether the rollback itself came back up.
    #[error("health check failed after update; rolled back, rollback-restart exit code {restart_exit_code}")]
    RolledBack { restart_exit_code: i32 },
}

/// True for any 2xx HTTP status code.
pub fn health_check_ok(status: u16) -> bool {
    (200..300).contains(&status)
}

/// Builds the loopback health-check URL from a bind address by reusing the port
/// after the last `:`. A `0.0.0.0`-style bind still serves on loopback, so this
/// always targets `127.0.0.1` — a connectable address. KNOWN LIMITATION: a bind
/// to a specific non-loopback interface (e.g. `192.168.1.5:8787`) is not
/// handled here; fixing that is out of scope for this cell.
pub fn health_check_url(bind_addr: &str) -> String {
    let port = bind_addr.rsplit(':').next().unwrap_or("8787");
    format!("http://127.0.0.1:{port}/api/health")
}

/// Perform the full update rollout against the running service (D3/D4/D9).
///
/// `new_bytes` must be the checksum-verified bytes from EP3's
/// `download_and_verify` — never a second, unverified download path (the EP5
/// carry-forward invariant that keeps D8's "verify before overwrite" honest).
pub async fn perform_update(
    new_bytes: &[u8],
    config_path: &Path,
    bind_addr: &str,
) -> Result<(), RolloutError> {
    // (1) Stop. Its exit code is deliberately ignored: the service may not have
    // been running yet, which is not an abort condition.
    let _ = crate::doctor::run_service_command("stop");

    // (2) Swap the running binary, capturing the backup path for rollback (D3/D9).
    let binary_backup_path = match super::swap::backup_and_swap_running_binary(new_bytes) {
        Ok(path) => path,
        Err(e) => {
            // The binary was never replaced; bring the unchanged service back.
            let _ = crate::doctor::run_service_command("start");
            return Err(RolloutError::Swap(e));
        }
    };

    // (3) Self-exec the just-swapped binary to merge config under the NEW
    // binary's own compiled defaults (D5). A failed merge never touches the
    // original config file (backup_and_recreate's guarantee), so it is
    // non-fatal here — an unbootable new binary is caught by the health check.
    let config_backup_path = merge_config_via_new_binary(config_path);

    // (4) Start the service on the new binary.
    let _ = crate::doctor::run_service_command("start");

    // (5) Poll `/api/health` until healthy or the budget is exhausted (D4).
    if poll_health(bind_addr).await {
        return Ok(());
    }

    // (7) Health never came up — roll back binary + config and start again (D9).
    rollback(
        &binary_backup_path,
        config_path,
        config_backup_path.as_deref(),
    )
}

/// Self-exec the current (just-swapped) executable with the hidden
/// `--internal-merge-config` verb so the merge runs under the new binary's own
/// compiled defaults. Returns the config backup path the subprocess printed to
/// stdout, or `None` when the spawn failed, the merge exited nonzero, or no
/// path was printed — all non-fatal to the update, only logged.
fn merge_config_via_new_binary(config_path: &Path) -> Option<PathBuf> {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => {
            eprintln!("update: could not locate the current executable for config merge: {e}");
            return None;
        }
    };
    let output = std::process::Command::new(exe)
        .arg("--internal-merge-config")
        .arg(config_path)
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let printed = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if printed.is_empty() {
                eprintln!("update: config merge succeeded but printed no backup path");
                None
            } else {
                Some(PathBuf::from(printed))
            }
        }
        Ok(out) => {
            eprintln!(
                "update: config merge exited with {}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            );
            None
        }
        Err(e) => {
            eprintln!("update: could not run config merge subprocess: {e}");
            None
        }
    }
}

/// Poll the health endpoint on a bounded budget (10 attempts, 1s apart).
async fn poll_health(bind_addr: &str) -> bool {
    let url = health_check_url(bind_addr);
    for attempt in 0..HEALTH_ATTEMPTS {
        if let Ok(resp) = reqwest::get(&url).await {
            if health_check_ok(resp.status().as_u16()) {
                return true;
            }
        }
        if attempt + 1 < HEALTH_ATTEMPTS {
            tokio::time::sleep(HEALTH_INTERVAL).await;
        }
    }
    false
}

/// Restore the previous binary and (if captured) the previous config, then
/// start the service again, returning the rollback-restart's exit code inside
/// [`RolloutError::RolledBack`]. Every restore step is best-effort and logged
/// rather than propagated, so the rollback always ends with a start attempt and
/// the exit code is never silently discarded (D9). Per D9 as written, health is
/// NOT re-polled after this second start — that end-to-end proof is EP6's smoke
/// test.
fn rollback(
    binary_backup_path: &Path,
    config_path: &Path,
    config_backup_path: Option<&Path>,
) -> Result<(), RolloutError> {
    match (std::env::current_exe(), std::fs::read(binary_backup_path)) {
        (Ok(exe), Ok(backup_bytes)) => {
            if let Err(e) = super::swap::backup_and_swap_binary(&exe, &backup_bytes) {
                eprintln!("rollback: failed to restore the previous binary: {e}");
            }
        }
        (Err(e), _) => {
            eprintln!("rollback: could not locate the current executable to restore: {e}")
        }
        (_, Err(e)) => eprintln!(
            "rollback: could not read the previous binary backup at {}: {e}",
            binary_backup_path.display()
        ),
    }

    if let Some(cfg_backup) = config_backup_path {
        match std::fs::read(cfg_backup) {
            Ok(bytes) => {
                if let Err(e) = std::fs::write(config_path, bytes) {
                    eprintln!("rollback: failed to restore the previous config: {e}");
                }
            }
            Err(e) => eprintln!(
                "rollback: could not read the config backup at {}: {e}",
                cfg_backup.display()
            ),
        }
    }

    let restart_exit_code = crate::doctor::run_service_command("start");
    Err(RolloutError::RolledBack { restart_exit_code })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_check_ok_true_for_2xx_false_otherwise() {
        assert!(health_check_ok(200));
        assert!(health_check_ok(201));
        assert!(health_check_ok(204));
        assert!(!health_check_ok(404));
        assert!(!health_check_ok(500));
    }

    #[test]
    fn health_check_url_targets_loopback_reusing_the_bind_port() {
        assert_eq!(
            health_check_url("0.0.0.0:8787"),
            "http://127.0.0.1:8787/api/health"
        );
    }
}

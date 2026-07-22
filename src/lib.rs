//! herdr-go library crate.
//!
//! Module-per-concern, hexagonal ports only at real seams (decision 4e3ef1a1):
//! `herdr` (HerdrControl + HerdrStream), `watcher::EventSource`, `store::Store`,
//! and later `notify::Notifier`. `security` is pure functions, called everywhere,
//! depending on nothing. `web` owns the Tier 2 relay which bridges directly to
//! `HerdrStream` and bypasses `core` by design.

pub mod config;
pub mod doctor;
pub mod herdr;
pub mod notify;
pub mod security;
pub mod store;
pub mod supervisor;
pub mod watcher;
pub mod web;

/// Build fingerprint (semver + short git sha + local build timestamp),
/// surfaced at `/api/health` and startup logs. Computed once in `build.rs`
/// so every display site shows the identical string.
pub const VERSION: &str = env!("HERDR_GO_FINGERPRINT");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_present() {
        assert!(!VERSION.is_empty());
    }
}

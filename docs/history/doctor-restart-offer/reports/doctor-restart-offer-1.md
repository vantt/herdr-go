# doctor-restart-offer-1

**Status:** DONE

**Outcome:** Added `active_service_restart()` (systemd on Linux via existing `systemd_state()`, launchd on macOS via `id -u` + `launchctl print`/`kickstart`, no `cfg` guard needed) and `offer_service_restart()`/`offer_service_restart_with()`, wired into `offer_fixes`' web-token match arm via a guard so a restart is only offered after a freshly-generated token — never on the already-satisfied no-op path. Confirm-gated; silent no-op when nothing is running.

**Files touched:** `src/doctor/checks.rs`

**Commit:** c68e4d0

Full trace and verification evidence: `.bee/cells/doctor-restart-offer-1.json`

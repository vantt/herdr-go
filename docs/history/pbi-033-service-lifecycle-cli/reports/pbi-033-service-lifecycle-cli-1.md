# pbi-033-service-lifecycle-cli-1

**Status:** [DONE]

**Outcome:** Added `herdr-go service {start|stop|restart|status}` — runtime-probes
systemd `--user` (Linux), launchd (macOS), and a shelled-out PowerShell
Scheduled Task call (Windows), thin pass-through to each native command's own
exit status/output, stderr error + exit 2 when no service manager is found.

**Files touched:** `src/main.rs`, `src/doctor/checks.rs`, `src/doctor/mod.rs`
(the last is a one-line deviation — a `pub use` re-export needed so `main.rs`
can reach `checks::run_service_command` across the module's privacy boundary;
not in the cell's declared file list).

**Full trace/evidence:** `.bee/cells/pbi-033-service-lifecycle-cli-1.json`

**Commit:** `b495cad`

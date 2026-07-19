# [DONE] windows-support-1

**Outcome:** Added one logical endpoint resolver for startup and doctor, retained Tokio Unix sockets on Unix, added bounded Tokio named-pipe connection behavior on Windows using herdr v0.7.4's GenericNamespaced-compatible name, and proved the supervisor restart remains shell-free with an explicit session.

**Files touched:** `src/herdr/socket.rs`, `src/main.rs`, `src/supervisor.rs`, and `src/doctor.rs` (required deviation so doctor consumes the same resolver). The temporary unresolvable dependency was removed; `Cargo.toml` and `Cargo.lock` have no net cell change.

**Evidence anchors:** endpoint resolution and platform connector at `src/herdr/socket.rs:30`; startup composition at `src/main.rs:181`; redacted doctor resolution at `src/doctor.rs:89`; restart command contract at `src/supervisor.rs:29`.

Full verification evidence and output: [`.bee/cells/windows-support-1.json`](../../../../.bee/cells/windows-support-1.json).

**Commit environment:** The cell capped green, but staging was denied before any index change because `.git/index.lock` is on a read-only filesystem. The verified working-tree changes remain intact for the orchestrator to commit selectively; the unrelated pre-existing `src/main.rs` migration edits were not staged or altered.

# cross-platform-install-3

**Status:** [DONE]

**Outcome:** release.yml's Package step now copies `packaging/herdr-go.plist` for the `aarch64-apple-darwin` matrix entry only (OS-conditional branch mirroring the existing `matrix.cross` pattern); both Linux entries keep copying `packaging/herdr-go.service` unchanged. ci.yml gained a new pinned `macos-14` job running `cargo fmt --all --check` / `cargo clippy --all-targets -- -D warnings` / `cargo test`, mirroring the existing `rust` job; the `rust`, `web`, and `windows-server-2022` jobs are untouched.

**Files touched:** `.github/workflows/release.yml`, `.github/workflows/ci.yml`

**Commit:** f76b82e

Full trace and verification evidence: `.bee/cells/cross-platform-install-3.json`

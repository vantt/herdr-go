# cross-platform-install-2 — install.sh Darwin branch + launchd plist + --uninstall

**Status:** [DONE] — capped, verify passed.

**Outcome:** `install.sh` now accepts Darwin at its OS guard, maps arm64/aarch64 to the `aarch64-apple-darwin` release asset (Intel Macs get a named source-build error per D11), generates and `launchctl`-loads a per-user LaunchAgent (idempotent bootout-then-bootstrap, `load -w` fallback), and adds an `--uninstall` flag for both Linux (systemd) and macOS (LaunchAgent) that leaves config/data/token untouched (D10). New `packaging/herdr-go.plist` launchd template invokes the binary with `--config` only and carries no secret. The existing Linux systemd flow is unchanged (kept byte-identical inside the `OS==Linux` branch).

**Files touched:**
- `install.sh`
- `packaging/herdr-go.plist` (new)

**Security note (D-fix):** The plist never contains, references, or embeds the login token or any secret. The binary resolves `HERDR_GO_WEB_SECRET` itself at startup from its config dir (`src/config/mod.rs` `ensure_web_secret()`), the analog of systemd's `EnvironmentFile=`. No `EnvironmentVariables` dict, no `herdr-go.env` read from the shell — this is the root fix for the earlier CRITICAL plist-embedded-token finding, enforced by the verify contract.

**Stale-plan note:** `plan.md` line 28 still describes the pre-fix design (an inline `EnvironmentVariables` dict populated from the env file). That text predates the CRITICAL fix; the cell action, its prohibitions, and its verify command are authoritative and were followed.

**Verify / trace:** see `.bee/cells/cross-platform-install-2.json` for the full trace, recorded verify output, and behavior-change evidence (single source per decision 0009).

**Deferred (not this cell):** real launchd load/restart/uninstall roundtrip proof runs on the slice-level `macos-14` CI runner (plan.md Slice 1) — no macOS host is available in this Linux execution environment.

---
artifact_contract: bee-plan/v1
mode: high-risk
approved_gate2: 2026-07-20T04:20:00Z
---

# Plan: cross-platform-install

Mode: `high-risk` — 4 risk flags: cross-platform, multi-domain (install script + Rust config layer + CI + release packaging), external systems (macOS launchd, Windows Task Scheduler), public contracts (new install/uninstall commands operators will rely on)
Why this is the least workflow that protects the work: it changes what an operator runs to install and how the binary resolves its own state directories on two platforms that have never run this code path before — a mistake here is either data loss (wrong native paths) or a broken install a new user hits on day one. Epic map + persona-panel-equivalent rigor (via validating's plan-checker/cell-review) protects that, at tiny/small ceremony's cost of missing exactly this kind of cross-cutting risk.

## Requirements (from CONTEXT.md)

- D1: macOS config+data under `~/Library/Application Support/herdr-go`; token protection reuses the existing `#[cfg(unix)]` branch unchanged.
- D2: macOS auto-start via a launchd LaunchAgent with KeepAlive/ThrottleInterval.
- D3: `install.sh` gets a Darwin branch (OS guard + arch mapping), never a second script.
- D4: Windows gets `install.ps1`, `irm | iex` distribution, installs to `%LOCALAPPDATA%\herdr-go\bin`.
- D5: Windows auto-start via a per-user Scheduled Task (logon trigger), never an elevated NT Service.
- D6: both new install paths are idempotent (never overwrite config/token, no duplicate auto-start registration).
- D7: contributor dev-as-live parity for macOS/Windows is OUT OF SCOPE (filed as PBI-015).
- D8: `release.yml`'s `aarch64-apple-darwin` Package step swaps `packaging/herdr-go.service` for a new `packaging/herdr-go.plist` on that entry only; `install.sh` stays in the macOS archive.
- D9: new pinned `macos-14` CI job in `ci.yml` (fmt/clippy/test).
- D10: `install.sh --uninstall` (Linux + macOS) and `install.ps1 -Uninstall` (Windows), all leave config/data/token untouched.
- D11: `x86_64-apple-darwin` (Intel) explicitly out of scope — Darwin branch only recognizes arm64/aarch64, named error + source-build pointer otherwise (filed as PBI-016).

## Discovery

L0 — every pattern needed already exists in-repo to copy from: `install.sh`'s existing Linux flow (OS/arch detection, idempotent config/token creation, unit install) is the direct template for both the Darwin branch and `install.ps1`; `src/config/mod.rs`'s existing `#[cfg(windows)]`/`#[cfg(not(windows))]` split is the direct template for the new `#[cfg(target_os = "macos")]` branch; `packaging/herdr-go.service` is the direct template for `packaging/herdr-go.plist`'s equivalent fields (per the gather-agent's mapping: `ExecStart`→`ProgramArguments`, `Restart=always`/`RestartSec=3`→`KeepAlive`+`ThrottleInterval`, `EnvironmentFile=`→ no native launchd equivalent, so the plist wraps the binary invocation with an inline `EnvironmentVariables` dict populated from the same env file at generation time by `install.sh`, since launchd plists cannot source an external env file at load time). No external research needed.

## Approach

Recommended path: two independently-provable slices, each proven on its own real GitHub-hosted CI runner before being considered done — following this session's own precedent (`windows-release-matrix`, `windows-username-length-fix`) of never trusting "compiles" as proof for platform-specific lifecycle code.

**Slice 1 — macOS** (current slice, cells prepared this round):
1. `src/config/mod.rs`: add `#[cfg(target_os = "macos")]` branch for config_dir/data_dir/native root — `~/Library/Application Support/herdr-go` for both (D1).
2. `install.sh`: Darwin branch — accept `Darwin` in the OS guard, map `arm64` (and reject `x86_64` with a named error per D11), write a launchd plist instead of a systemd unit, use `launchctl bootstrap`/`load` instead of `systemctl --user enable`, add `--uninstall` flag (D3, D10, D11).
3. `packaging/herdr-go.plist`: new launchd template (D2).
4. `release.yml`: swap the Package step's service-file copy for the macOS matrix entry only (D8).
5. `ci.yml`: new `macos-14` job (D9).
6. Real-CI proof: push a throwaway pre-tag exactly like `windows-release-matrix`'s validation did, confirm the macOS archive installs, starts via LaunchAgent, and uninstalls cleanly on a real `macos-14` runner; delete the tag/release immediately after (same protocol already used twice this session).

**Slice 2 — Windows** (future slice — NOT cells this round, per "current slice only"):
1. `install.ps1`: new PowerShell installer mirroring `install.sh`'s flow (download `herdr-go-x86_64-pc-windows-msvc.zip` from the latest release, install to `%LOCALAPPDATA%\herdr-go\bin`, create config/token via existing `#[cfg(windows)]` paths, register Scheduled Task, `-Uninstall` switch).
2. Real-CI proof: same throwaway-tag/workflow_dispatch protocol on `windows-2022`.

Rejected alternatives:
- A single cross-platform install script (e.g. a small Go/Rust bootstrap binary) instead of three OS-native scripts — rejected: adds a new build/distribution problem (how do you fetch the bootstrapper itself without already having a package manager?) to solve a problem three well-understood native conventions (curl\|bash, curl\|bash w/ Darwin branch, irm\|iex) already solve; not worth the complexity for 3 platforms.
- True Windows NT Service via a service-wrapper crate — rejected per D5 (elevation requirement breaks the convenience goal).
- Separate `install-macos.sh` script — rejected per D3 (fragments the one memorable install command).

Risk map:
| Component | Risk | Proof needed |
|---|---|---|
| macOS native path resolution (D1) | MEDIUM | real `macos-14` run must show config/data actually land under `~/Library/Application Support/herdr-go`, not silently fall through to the old XDG branch |
| launchd plist correctness (D2) | MEDIUM | real run must show the LaunchAgent actually restarts the process (KeepAlive) and survives a `launchctl kickstart`/crash-simulate, not just "loads without error" |
| install.sh Darwin branch not breaking the existing Linux branch | LOW | verify command diffs the Linux code path is byte-unchanged; Linux CI (ci.yml `rust`/`web` jobs) already re-runs on every push as a regression net |
| release.yml Package-step swap not breaking Linux entries | LOW | same shared-job risk class `windows-username-length-fix`'s lesson (D1's rename-regression pattern) warns about — verify explicitly diffs the two Linux entries' Package step text is unchanged |

## Test matrix

- **Cross-platform:** macOS-specific code path (`#[cfg(target_os = "macos")]`) must not compile into or affect Linux/Windows builds — verified by `cargo build` on Linux CI still succeeding unchanged.
- **Idempotency:** running `install.sh`'s Darwin branch twice must not duplicate the LaunchAgent registration or overwrite an existing token/config.
- **Uninstall roundtrip:** install → uninstall → config/data/token files still present, binary and LaunchAgent gone.
- **Failure path:** Intel Mac (`x86_64` Darwin) gets a named, actionable error, not a silent wrong-binary download or a generic crash.
- **Contract:** the macOS release archive, once built, contains `herdr-go.plist` and does NOT contain `packaging/herdr-go.service` (mirrors the exact allowlist-contract test pattern `windows-release-matrix-1`'s cell used).

## Out of scope

- Windows Scheduled Task + `install.ps1` (Slice 2 — next planning round after Slice 1 is proven).
- Contributor dev-as-live parity for macOS/Windows (D7, filed PBI-015).
- Intel Mac / `x86_64-apple-darwin` (D11, filed PBI-016).
- Any change to the existing Linux install.sh behavior beyond adding the Darwin branch alongside it.

## Current slice

**Slice 1 — macOS.** Entry state: `install.sh` hard-fails on Darwin; `config/mod.rs` has no macOS branch (falls through to Linux XDG paths); `release.yml`'s macOS archive ships an inert systemd unit; `ci.yml` has zero macOS coverage. Exit state: a macOS operator can run the exact same `curl -fSL .../install.sh | bash` command Linux operators use, get a working LaunchAgent-backed background service using real native `~/Library/Application Support` paths, and cleanly uninstall with `install.sh --uninstall`; this is proven on a real `macos-14` GitHub Actions runner, not just compiled. Files: `src/config/mod.rs`, `install.sh`, `packaging/herdr-go.plist` (new), `.github/workflows/release.yml`, `.github/workflows/ci.yml`. Verify: cargo test/clippy/fmt (existing `commands.verify`) plus each cell's own structural verify, plus the real-CI proof step described above.

## Cells

- `cross-platform-install-1` — config/mod.rs macOS native path branch
- `cross-platform-install-2` — install.sh Darwin branch + packaging/herdr-go.plist + --uninstall
- `cross-platform-install-3` — release.yml Package-step swap + ci.yml macOS job

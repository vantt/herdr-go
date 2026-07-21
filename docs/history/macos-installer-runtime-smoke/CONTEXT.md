# macOS Installer Runtime Smoke Test — Context

**Feature slug:** macos-installer-runtime-smoke
**Date:** 2026-07-20
**Exploring session:** complete
**Scope:** Standard
**Domain types:** RUN

## Feature Boundary

Add a real end-to-end smoke test for `install.sh`'s Darwin branch on `macos-14` CI: after the macOS release archive publishes, download it, run `install.sh` for real, prove the launchd LaunchAgent it registers actually starts the service and recovers after a real crash, then run `install.sh --uninstall` and prove cleanup matches the documented uninstall contract. This closes PBI-017 (cross-platform-install Slice 1's gap: path resolution and packaging were proven real, the install.sh flow itself never was). It is the macOS sibling of `windows-installer-runtime-smoke` (PBI-018, already closed) — same shape, adapted to launchd instead of Scheduled Task. Proving the Herdr binary round-trip on macOS is explicitly out of scope (D1) — filed separately as PBI-023 (renumbered from an initial PBI-020, which collided with a concurrent session's use of that id).

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Scope stays narrow: installer lifecycle only (install → launchd live → crash → restart recovery → uninstall → clean removal). Proving the Herdr binary round-trip on macOS is out of scope, filed as PBI-023 (renumbered, see Deferred Ideas). | User choice: keep this feature symmetric to `windows-installer-runtime-smoke` (small lane, easy to verify) rather than bundling in a materially larger proof — macOS's `ci.yml` `macos` job runs only fmt/clippy/test today, unlike Windows which already had a dedicated Herdr round-trip job before PBI-018 started. |
| D2 | The test runs POST-publish, in a new dedicated job (see D8) that runs after the macOS release archive is uploaded — not an appended step in the existing job, since D8 rules that out. Failure fails that job (red run); the asset stays published. | Same architecture as `windows-installer-runtime-smoke` D1 — `install.sh` downloads from a real public GitHub release URL, so proving the real unauthenticated end-user flow requires the asset already public. Unlike Windows (D10 there: appended step, already-dedicated job), macOS's publish lives in a shared matrix job (D8), so "post-publish" here means a new job, not a new step in the same job. |
| D3 | The test pins the exact just-tagged version via `install.sh`'s existing `HERDR_GO_VERSION` override (`install.sh:132`), never left as `latest`. | Avoids a race against a concurrently publishing tag. `install.sh` already supports the override. |
| D4 | Liveness proof is `/api/health` returning any 200 (`herdr_up` need not be true) — no Herdr binary is wired into the LaunchAgent's environment. | Confirmed via direct read of `install.sh`'s plist heredoc (lines 167-193): `ProgramArguments` carries only `--config`, no environment-injection block ("carries NO secret and NO environment-injection block"). Identical situation and resolution to `windows-installer-runtime-smoke` D8. |
| D5 | Crash-restart recovery is proven with a real wait, sized to launchd's `KeepAlive`/`ThrottleInterval=3` (3-second minimum between restarts, `install.sh:185-186`) plus buffer — not simulated or skipped, and much shorter than Windows' ~90s wait since launchd's floor is seconds, not a full minute. | `KeepAlive` (`SuccessfulExit: false`) plus `ThrottleInterval=3` is the real recovery mechanism under test. |
| D6 | Uninstall verification reuses `cross-platform-install` CONTEXT.md's D10 contract: after `install.sh --uninstall`, the LaunchAgent is unloaded/removed and the binary is gone, while config, data, and the login token under `~/Library/Application Support/herdr-go` are left untouched. | D10 already defines the correct end-state for macOS uninstall (`install.sh:54-87`'s own comment cites D10); this feature verifies that contract at runtime instead of redefining it. |
| D7 | Unlike Windows' unprovable `AtLogOn` trigger (`windows-installer-runtime-smoke` D4), macOS GitHub-hosted runners run as a real logged-in user with an active GUI session, so `launchctl bootstrap gui/$(id -u)` is expected to genuinely register and start the LaunchAgent in CI — not just prove a manually-started path. Treated as a working assumption, confirmed only by the script's first real CI run. | `macos-14` runners are known to run with a full user session (unlike headless `windows-2022` Server runners), but this repo has no existing macOS launchd CI precedent to cite directly. |
| D8 | The new post-publish smoke step goes in a brand-new, dedicated job — not appended into `release.yml`'s existing shared `build` matrix job, which currently mixes Linux and macOS in one `steps:` list. | Per this repo's own `critical-patterns.md` (windows-release-matrix learning): OS-guarding steps in a shared-matrix job self-contradicts a "leave other platforms untouched" constraint. A new dedicated job avoids touching the existing Linux/macOS build matrix at all. |
| D9 | The CI step masks the printed login token via GitHub Actions' `::add-mask::` immediately once captured, matching `install.sh`'s own plaintext echo (`install.sh:158`). | `install.sh` prints the token in plaintext by design (real end-user UX); masking it in CI output matches `windows-installer-runtime-smoke` D9's established posture. |

## Terms

(none new — reuses terms already pinned in `windows-installer-runtime-smoke` CONTEXT.md: "post-publish smoke test".)

## Existing Code Context

### Reusable Assets

- `scripts/windows-install-smoke.ps1` — the sibling Windows script (same feature shape: install → verify-live → crash → verify-restart → uninstall → verify-clean). Reusable *structure and function names* (`Assert-True`, `Wait-Until`, redact-then-mask token handling, `try/finally` cleanup) translated to bash — not reusable code directly (different shell, different OS primitives).
- `install.sh` (repo root) — the installer under test. Darwin branch already downloads via `HERDR_GO_VERSION` override, registers the `io.github.vantt.herdr-go` LaunchAgent (`KeepAlive`/`ThrottleInterval=3`/`RunAtLoad`), and supports `--uninstall` per D10 of `cross-platform-install`.
- `docs/history/learnings/20260719-windows-release-matrix-structural-verify.md` — the "new job, not OS-guarded shared steps" pattern this feature's D8 directly applies.

### Established Patterns

- `.github/workflows/release.yml`'s `build` job (matrix: 2 Linux targets + 1 macOS target, shared `steps:` list) — the macOS release archive currently publishes from inside this shared job (`Upload to release` step). The new smoke step does NOT go here (D8) — it goes in a new dedicated job triggered after this one publishes.
- `.github/workflows/ci.yml`'s `macos` job (lines 31-49) — fmt/clippy/test only, no runtime smoke of any kind today; confirms the D1 scope-narrowing premise.

### Integration Points

- `.github/workflows/release.yml` — new dedicated job (name TBD in planning), likely `needs: build` or independently fetching the published asset.
- `scripts/` — new bash script (name TBD in planning, parallel to `windows-install-smoke.ps1`).

## Canonical References

- `docs/history/windows-installer-runtime-smoke/CONTEXT.md` — the sibling feature this one mirrors; D1-D12 there are the direct precedent for D2-D9 here.
- `docs/history/cross-platform-install/CONTEXT.md` — D10 (uninstall leaves config/data/token untouched, both platforms).
- `install.sh` — the script under test.
- `docs/history/learnings/critical-patterns.md` — the windows-release-matrix shared-job learning behind D8.

## Outstanding Questions

### Deferred To Planning

- [ ] Exact new script name, new job name, and whether the job needs `needs: build` or independently polls the public release URL — implementation detail, planning's call.
- [ ] Whether D7's assumption (macOS CI runners have a real GUI session) holds is confirmed only empirically by the first real CI run, same honesty posture as `windows-installer-runtime-smoke` D12's pwsh-less local verify gap.
- [ ] Crash-injection mechanism: D5's `KeepAlive` (`SuccessfulExit: false`, `install.sh:180-184`) restarts the process ONLY on an unsuccessful exit — a graceful termination that exits 0 will NOT trigger launchd's restart, silently invalidating the recovery proof. Planning must pick a termination method that produces a non-zero/unsuccessful exit (e.g. `kill -9`, not a signal the process handles cleanly) to actually exercise this path — flagged by fresh-eyes review; the Windows sibling had the equivalent question (`taskkill` vs. another mechanism) and resolved it during planning, not exploring.

## Deferred Ideas

- PBI-023: macOS Herdr round-trip runtime smoke (launchd-equivalent of `scripts/windows-runtime-smoke.ps1`) — filed to `docs/backlog.md` (D1).

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked decisions, code context, canonical references, and deferred-to-planning questions. Validating and reviewing use locked decisions for coverage and UAT.

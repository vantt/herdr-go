# Windows Installer Runtime Smoke Test — Context

**Feature slug:** windows-installer-runtime-smoke
**Date:** 2026-07-20
**Exploring session:** complete
**Scope:** Standard
**Domain types:** RUN

## Feature Boundary

Add a real end-to-end smoke test for `install.ps1`'s actual end-user flow on `windows-2022` CI: after the existing `release-windows` job in `.github/workflows/release.yml` publishes the Windows release asset, download it, run `install.ps1` for real, prove the Scheduled Task it registers actually starts the service and recovers after a real crash, then run `install.ps1 -Uninstall` and prove cleanup matches the documented uninstall contract. This closes PBI-018 (cross-platform-install Slice 2's gap: the installer's structure was statically verified, its live behavior never was). It ends at Windows; the macOS equivalent is PBI-017, a separate feature.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | The test runs POST-publish: a new step appended to `release-windows` right after its existing "Upload to release" step, no restructuring of the publish flow. A failure fails the CI job (red run) — the same alert mechanism the rest of the pipeline already relies on. The asset stays published either way. | User chose this over a draft/promote pre-publish gate: `install.ps1` downloads from a real public GitHub release URL, so proving the real unauthenticated end-user flow requires the asset to already be public; gating pre-publish would need draft releases + a test-only auth tweak that deviates from testing the literal end-user path. |
| D2 | The test pins the exact just-tagged version under test (the release-triggering tag / `GITHUB_REF_NAME`) via `install.ps1`'s existing `$env:HERDR_GO_VERSION` override — never `latest`. | Using `latest` risks a race if another tag publishes mid-run; pinning guarantees the test proves the asset it's actually supposed to be testing. |
| D3 | `scripts/windows-runtime-smoke.ps1` (binary-level, direct `Start-Process` supervision) is untouched and keeps running as-is. A new, separate script proves the `install.ps1`/Scheduled Task lifecycle. Neither replaces the other. | The two scripts prove different risk surfaces — raw binary process supervision vs. the real installer + Scheduled Task registration/uninstall path — with no overlap. |
| D4 | The `AtLogOn` trigger itself is a documented, accepted gap: `windows-2022` GitHub Actions runners have no real interactive logon session, so firing the trigger via an actual logon cannot be proven in CI. The test proves only the manually-started path (`Start-ScheduledTask`, the same call `install.ps1` itself makes right after registration) plus `RestartInterval`-based crash recovery. | No way to simulate a real interactive user logon on a headless CI runner; documenting the gap is honest, blocking on it is not achievable. |
| D5 | Crash-restart recovery is proven with a real wait against the Scheduled Task's `RestartInterval` floor (~1 minute) — not simulated, not skipped. | `RestartInterval` has a hard 1-minute granularity floor (already called out in `install.ps1`'s own comments); only a real wait proves real recovery, and the added CI time is negligible next to the rest of the Windows release job. |
| D6 | Uninstall verification reuses `docs/history/cross-platform-install/CONTEXT.md`'s D10 contract as acceptance criteria: after `install.ps1 -Uninstall`, the Scheduled Task and installed binary are gone; config, data, and the login token under `%APPDATA%\herdr-go` are left untouched. | D10 already defines the correct end state for this exact command; this feature verifies that contract at runtime instead of redefining it. |
| D7 | Scope is `release.yml`'s `windows-2022` release pipeline only. A faster non-release (`ci.yml`) install-flow smoke test against a locally-hosted fake asset is filed as a new deferred backlog idea (PBI-019), not built here. | `install.ps1`'s real value under test is the actual public download path; a local-asset variant tests something materially different (script logic minus the real publish path) and needs its own scoping. |
| D8 | "Started"/"restarted" proof for this test means the herdr-go gateway process launched by the Scheduled Task responds on `/api/health` (any 200 response; `herdr_up` need not be true) — NOT the full Herdr API round-trip (login + agent create + pane screen/input) that `windows-runtime-smoke.ps1` already independently proves. The new test does not wire any Herdr binary into the Scheduled Task's environment. | `install.ps1`'s Scheduled Task registration carries no environment-injection block (by design — the binary resolves its own secret at startup), so the existing round-trip patterns (which require a live Herdr binary wired via `HERDR_GO_HERDR_BINARY`) cannot run under it without new, out-of-scope wiring. D3 already assigns the Herdr round-trip to `windows-runtime-smoke.ps1`; this test's job is the installer/Scheduled-Task lifecycle only. |
| D9 | The CI step masks the printed login token via GitHub Actions' `::add-mask::` immediately once captured, even though practical exposure here is near-zero (loopback-only gateway, ephemeral runner destroyed after the job). | `install.ps1` prints the token in plaintext by design (matches `install.sh`'s "print only on first creation" UX), but `windows-runtime-smoke.ps1` already treats this exact value as secret-shaped and redacts it in its own log handling. Matching that existing posture in CI is cheap and avoids an inconsistent precedent. |

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Post-publish smoke test | A verification step that runs after an artifact is already publicly released, as opposed to a pre-publish gate that would block publishing on failure. |

## Existing Code Context

### Reusable Assets

- `scripts/windows-runtime-smoke.ps1` — existing CI smoke script; reusable *patterns* only (login via `/api/login`, gateway round-trip through `/api/agents` + `/api/panes/.../screen`+`/input`, ACL assertions via `icacls.exe`) — its lifecycle (`Start-Process` on the raw compiled binary) is the wrong shape for this feature and is not touched.
- `install.ps1` (repo root) — the installer under test. Already downloads via `$env:HERDR_GO_VERSION` override, registers Scheduled Task `HerdrGo` (`AtLogOn` trigger, `RunLevel Limited`, `RestartCount 3` / `RestartInterval` 1 min), and supports `-Uninstall` per D10 of `cross-platform-install`. No known open defects (the prior BOM-in-config.json P1 for this script is already fixed and capped — verified in code: `UTF8Encoding($false)`).

### Established Patterns

- `.github/workflows/release.yml` `release-windows` job — build → existing binary-level smoke test → package → "Upload to release". The new step is appended after "Upload to release" per D1.

### Integration Points

- `.github/workflows/release.yml` — new step (or steps) added to the `release-windows` job.
- `scripts/` — new PowerShell script (name TBD in planning) implementing the install → verify-running → crash → verify-restart → uninstall → verify-clean sequence.

## Canonical References

- `docs/history/cross-platform-install/CONTEXT.md` — D5 (Scheduled Task, not NT Service, no elevation), D10 (uninstall leaves config/data/token untouched).
- `install.ps1` — the script under test.
- `scripts/windows-runtime-smoke.ps1` — sibling smoke test, reusable API-interaction patterns.

## Outstanding Questions

### Deferred To Planning

- [ ] Exact new script name and whether crash simulation kills the process via `taskkill` or another mechanism — implementation detail, planning's call.
- [ ] Whether the new step needs its own job (`needs: release-windows`) vs. an appended step in the same job — D1 only fixed the ordering (after upload), not the job topology; planning verifies which is mechanically simpler given `release-windows`'s existing structure.

## Deferred Ideas

- PBI-019: non-release (`ci.yml`) install-flow smoke test against a locally-hosted fake asset, for faster feedback than waiting on a real release — filed to `docs/backlog.md` (D7).

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked decisions, code context, canonical references, and deferred-to-planning questions. Validating and reviewing use locked decisions for coverage and UAT.

# macos-installer-runtime-smoke-1

**Status:** [DONE]
**Worker:** Stuart

## Outcome

Added `scripts/macos-install-smoke.sh` (real bash: install -> verify-live via `/api/health` -> crash via `kill -9` -> verify launchd `ThrottleInterval` restart -> `install.sh --uninstall` -> verify clean removal with config/data/token left untouched) and wired it into `.github/workflows/release.yml` as a new dedicated `macos-install-smoke` job (`needs: build`, `runs-on: macos-14`, `HERDR_GO_VERSION` pinned to `${{ github.ref_name }}`). Mirrors `scripts/windows-install-smoke.ps1`'s shape, translated to bash/launchd idioms.

## Files touched

- `scripts/macos-install-smoke.sh` (new)
- `.github/workflows/release.yml` (new job added; existing `build` job untouched)

## Verification

`bash -n` + YAML validity + full repo verify suite (`cargo test`, `cargo fmt --check`, `cargo clippy -D warnings`, `tests/rename_contract.sh`, `npm run bundle`, `npm run test -- --run`) all passed. Full trace and verification evidence: `.bee/cells/macos-installer-runtime-smoke-1.json`.

Local execution of the script itself was not possible (this dev box has no macOS/launchctl) — expected and accounted for per the cell's environment note and CONTEXT.md's D7/D12 posture. Real launchd correctness is proven on the next actual `macos-14` release CI run, not locally.

## Note on cap

`cells cap` emitted `JUDGE_STANDARD_INSUFFICIENT` (F5, semantic-checklist judge) on the `red_failure_evidence` field for this `behavior_change: true` cap. This is legitimate here, not a gap: no macOS install.sh runtime smoke test existed before this cell (confirmed via `docs/history/macos-installer-runtime-smoke/CONTEXT.md` D1 and its Established Patterns section — `ci.yml`'s `macos` job runs only fmt/clippy/test today), so there is no prior behavior to characterize as a "before" state. Recorded as a `deliberate_exceptions` entry in the evidence; cap succeeded (status: `capped`).

## Outstanding Questions

None blocking. Per CONTEXT.md's Outstanding Questions, D7's assumption (macOS GitHub-hosted runners genuinely support `launchctl bootstrap gui/$(id -u)` in CI) is confirmed only empirically by the first real `macos-14` CI run of this new job — not something this cell could prove locally.

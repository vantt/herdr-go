# Validation report — self-update-merge-config, current slice EP5

**Date:** 2026-07-22
**Lane:** high-risk — **the single highest-risk epic in this feature** (touches a live running service; a bad rollback could leave it worse off than before `update` ran)
**Scope:** EP5 only — cells `self-update-merge-config-14`, `-15`, `-16`

## Design correction (found during planning, before any cells cut)

The original Technical Design ordered "merge config → swap binary → restart." This was wrong: the `update` process is still running the OLD binary at that point, so merging then would use the OLD compiled defaults, violating D5. Corrected flow (see `approach.md`'s "EP5 design correction"): stop service → backup+swap binary → self-exec the *newly-swapped* binary with a hidden `--internal-merge-config <path>` verb (so the merge runs under the NEW binary's own compiled defaults) → start service → poll `/api/health` → roll back both binary and config on failure. User confirmed this direction before cells were cut.

## Reality gate

| Check | Result | Evidence |
|---|---|---|
| MODE FIT | PASS | Unchanged high-risk lane; this is the epic the lane exists to protect |
| REPO FIT | PASS | `main.rs`'s `Args`/`parse_args`/early-exit pattern, `run_service_command`'s standalone `start`/`stop` verbs (not just `restart`) for all 3 platforms, and `merge_config_on_upgrade`'s fail-closed guarantee all confirmed accurate against real source |
| ASSUMPTIONS | PASS (after repair) | `stop`/`start` confirmed to work standalone on systemd/launchd/Windows; `/api/health` confirmed to always return HTTP 200 (good — the poll measures "did the new binary boot," not unrelated backend liveness) |
| SMALLER PATH | PASS | 3 cells: hidden merge hook / binary swap / orchestration — each a coherent unit, matching the pattern established in EP1-EP4 |
| PROOF SURFACE | PASS (after repair) | All three verify commands confirmed non-tautological (exit non-zero) on the unmodified repo, both before and after repairs |

## Feasibility matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| `stop`/`start` work standalone on all 3 platforms (not just as part of `restart`) | Medium | Read all 3 per-OS implementations | Confirmed: systemd passes any verb through, launchd has explicit `start`/`stop` arms, Windows has separate `Start-ScheduledTask`/`Stop-ScheduledTask` cmdlets | PASS |
| A failed merge never touches the original config file | Medium | Trace `merge_config_on_upgrade` → `backup_and_recreate` → `persist_validated` | Confirmed by EP4's own shipped code and tests | PASS |
| Cell -16's instructed crate paths actually compile | **High (real finding)** | Check `herdr_go::` vs `crate::` usage inside vs. outside the lib crate | **Found**: cell -16 instructed `herdr_go::doctor::...` self-references, which do not resolve inside the lib crate (`src/update/` is *inside* `herdr_go`, not a consumer of it) — would have handed a worker non-compiling instructions. **Repaired**: corrected to `crate::doctor::...` | PASS (after repair) |
| Health retry budget is concrete enough to avoid false-positive rollback or a long hang | Medium | Check the cell's wording | Was "e.g. up to 10 attempts" (vague). **Repaired**: pinned to 10 attempts × 1s | PASS (after repair) |
| Rollback restores both binary and config, and its own success is observable | Medium | Trace the rollback path | Binary+config restore confirmed sound (reuses the same atomic-write+chmod machinery). Rollback's own restart exit code was previously discarded. **Repaired**: now captured into `RolloutError::RolledBack`'s field | PASS (after repair) |
| Schedule has no cycles | Low | `bee cells schedule` | `waves: [[-14,-15],[-16]]` (14/15 parallelizable, no file overlap), no cycles | PASS |

## Plan-checker (adversarial, `bee-review` subagent, opus) — safety/reliability persona panel

**Verdict: 0 BLOCKER (design-level), 1 CRITICAL (instruction-level, fixed), several WARNING/MINOR (fixed or explicitly scoped)**

Full 6-point safety walkthrough (every way this could leave the service worse off) is in the subagent's report — summary:
- Ignoring `stop`'s exit code: correct, not a bug (service may not have been running).
- Merge self-exec failing to launch, or the new binary being subtly broken despite passing checksum: caught downstream by the health check → rollback, which is the real backstop.
- Health retry budget vagueness: **fixed** (pinned to concrete numbers).
- Rollback's own atomic-write correctness: confirmed sound, reuses the same safety machinery as the forward swap.
- No health re-verify after rollback: matches D9's literal wording exactly — explicitly scoped, not silently missing, and now at least surfaces the rollback-restart's exit code so a failed rollback isn't reported as clean.
- EP3's "binary bytes only from `download_and_verify`" invariant: confirmed honored structurally — `perform_update` takes bytes as a parameter, does no download of its own.
- **New finding, explicitly scoped**: replacing a running executable on Windows is expected to fail (OS file lock) — a new, not-yet-proven-safe path distinct from the pre-existing Windows service-restart gap. Documented as a known limitation, real proof deferred to EP6's smoke test.

## Cold-pickup cell review

**Verdict: 1 CRITICAL (fixed), several MINOR (fixed or scoped)**

- **CRITICAL**: cell -16's `herdr_go::` self-references inside the lib crate — **fixed** to `crate::`.
- MINOR: cell -14's hidden-flag branch position relative to legacy-state migration — **fixed** (moved earlier in `main()`).
- MINOR: rollback-restart exit code was discarded — **fixed** (now captured).
- MINOR: Windows running-binary-swap unproven — **explicitly scoped** as a known limitation in cell -16's doc comment requirement, not silently left out.

## Repairs applied (this validating pass)

- `self-update-merge-config-16`: fixed `herdr_go::` → `crate::` throughout the action; pinned health retry budget to 10×1s; added `RolledBack { restart_exit_code }` requirement; added a Windows-limitation doc-comment requirement; verify updated to require `crate::doctor::run_service_command` and explicitly forbid `herdr_go::` in the file.
- `self-update-merge-config-14`: moved the hidden-flag early-exit branch before `migrate_default_state_if`.
- Re-confirmed all three verify commands still correctly fail (non-zero) on the unmodified repo after repairs.

## Advisor consult (AO2b/AO3)

No advisor configured. Recorded via `bee state advisor-ref record`, anchored to current feature/decision/plan-hash.

## Decision

**READY** — no open CRITICAL/BLOCKER items. The corrected design is sound; the one real instruction defect (compile-breaking crate paths) is fixed; remaining gaps are explicitly scoped, matching D9's own stated boundaries, with real end-to-end proof deferred to EP6's smoke test as originally planned.

## Gate 3

Gate bypass level `full` covers high-risk lanes — auto-approved, not asked.

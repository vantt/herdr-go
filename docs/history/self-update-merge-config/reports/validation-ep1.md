# Validation report — self-update-merge-config, current slice EP1

**Date:** 2026-07-22
**Lane:** high-risk
**Scope:** EP1 (release-checksum publishing) only — cells `self-update-merge-config-1`, `self-update-merge-config-2`

## Reality gate

| Check | Result | Evidence |
|---|---|---|
| MODE FIT | PASS | 5 risk flags cited in plan.md (audit/security, external provider, cross-platform, data model, multi-domain); high-risk is correctly the least-workflow-that-protects-the-work per its own stated rationale |
| REPO FIT | PASS | Cells reference real files confirmed present: `.github/workflows/release.yml` (inspected, line-anchored citations verified accurate by the plan-checker), `scripts/` directory (existing `macos-install-smoke.sh` precedent) |
| ASSUMPTIONS | PASS | The one blocking-shaped assumption (cross-job `checksums.txt` merge feasibility) is not actually blocking — cell -2 explicitly allows a per-job-group fallback; confirmed structurally sound either way against the real `release.yml` job graph (existing `needs: build` precedent at release.yml:194) |
| SMALLER PATH | PASS | 2 cells, zero dependencies on any other epic, zero running-service risk — already the smallest believable first slice |
| PROOF SURFACE | PASS | Both cells' `verify` commands run in this repo today: confirmed `python3 -c "import yaml..."` succeeds (pyyaml present), confirmed both `sha256sum` and `shasum` binaries are present locally |

## Feasibility matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| Cross-job checksum merge is possible in this repo's Actions structure | Medium | Inspect existing multi-job `needs:` usage in release.yml | `macos-install-smoke` job already uses `needs: build` (release.yml:194) — precedent confirmed | PASS |
| macOS lacks `sha256sum` (ships `shasum` only) | Low | Confirm both tools' presence/absence pattern is handled | Cell -1 explicitly requires detecting whichever tool is present; local environment has both, cell's dual-path requirement is testable | PASS |
| Cell -2's verify command doesn't false-positive on the real, unmodified file | High (would have silently passed a no-op) | Run the verify command against the actual current release.yml | **Initially FAILED** — `Get-FileHash` already present at release.yml:139-152 (unrelated third-party check) satisfied the original alternation-based grep. **Repaired**: verify now requires the literal string `generate-checksums.sh`, re-ran against the unmodified file → exit code 1 (correctly fails) | PASS (after repair) |
| Cell -1's script contract is unambiguous for a cold worker | Medium | Re-read action vs must_haves for internal consistency | Found genuine ambiguity ("tar.gz/zip files" vs "each file") plus an unhandled subdirectory case (real dist dirs contain an extracted `$name/` folder). **Repaired**: action + must_haves + verify all now pin the extension-scoped, skip-directories, never-self-include-output-file contract explicitly, and the verify test exercises a subdirectory case | PASS (after repair) |
| Schedule has no cycles / unsatisfiable deps | Low | `bee cells schedule` | `waves: [[-1],[-2]]`, `cycles: []`, `unsatisfiable_deps: []` (re-confirmed after cell patches) | PASS |

## Plan-checker (adversarial, `bee-review` subagent, opus)

**Verdict: 0 BLOCKER, 4 WARNING** (all four addressed below)

1. *Requirement coverage* — WARNING: cell -2's single-file-vs-fallback permissiveness could leave EP3 designed against an assumption EP1 didn't guarantee. **Addressed**: resolved in `approach.md`'s Questions section — EP3 must tolerate either outcome, noted explicitly for the next planning pass.
2. *Cell completeness* — WARNING: action text attributed cross-job artifact-passing need to the Windows job only, when the 3 Linux/macOS matrix legs are equally isolated runners. **Fixed** in cell -2's action text.
3. *Dependency correctness* — no issue; `deps: [self-update-merge-config-1]` on cell -2 is correct and matches the schedule.
4. *Key links* — WARNING: the "no reimplementing hashing inline" key_link was inconsistent with the Windows-native-command carve-out, which is exactly what let the pre-existing `Get-FileHash` satisfy the old verify. **Fixed**: key_link now explicitly excludes the pre-existing third-party `Get-FileHash` step from counting toward this cell's requirement.
5. *Scope sanity* — no issue; correctly CI-only, no hidden Rust/running-service scope forced.

## Cold-pickup cell review

**Verdict: 1 CRITICAL (fixed), 1 MINOR (fixed)**

- **CRITICAL** — cell -2's original verify passed on the real, unmodified `release.yml` (tautological — see feasibility matrix above). **Fixed and re-confirmed** (verify now exits 1 against the unmodified file).
- **MINOR** — cell -1's script contract was internally ambiguous and didn't account for a subdirectory in the scanned directory. **Fixed** (see feasibility matrix above); the cell's verify now includes a subdirectory in its test fixture.

## Repairs applied (this validating pass)

- `self-update-merge-config-1`: tightened `action`/`must_haves`/`verify` to pin the archive-extension-only, skip-subdirectories, stdout-only contract; verify now tests a subdirectory case.
- `self-update-merge-config-2`: fixed the tautological `verify` (removed the `Get-FileHash` alternation that let the unmodified file pass); corrected the job-topology description in `action`; tightened `must_haves` to explicitly exclude the pre-existing third-party checksum step from satisfying this cell.
- `approach.md`: resolved the "single checksums.txt vs. fallback" open question with evidence, and flagged the constraint forward to EP3.
- Re-ran `bee cells schedule` post-patch: unchanged, still `[[-1],[-2]]`, no cycles.

## Advisor consult (AO2b/AO3)

`resolveAdvisor` returned no configured advisor for this repo. Recorded per contract: `advisor_ref` written via `bee state advisor-ref record` (advisor: "unconfigured (resolveAdvisor returned null)"), anchored to the current feature, newest decision id, and `plan.md` sha256 — satisfies the mechanical AO3/AO13 precondition for Gate 3 on a high-risk lane.

## Decision

**READY** — no open CRITICAL/BLOCKER items remain. Both cells are cold-pickup-safe, their `verify` commands are proven runnable and non-tautological in this repo today, and the schedule is cycle-free.

## Gate 3

Gate bypass level `full` covers high-risk lanes — auto-approved, not asked. See `.bee/decisions.jsonl` for the audit entry and `.bee/state.json` for `approved_gates.execution`.

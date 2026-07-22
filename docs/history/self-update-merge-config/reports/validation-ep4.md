# Validation report — self-update-merge-config, current slice EP4

**Date:** 2026-07-22
**Lane:** high-risk
**Scope:** EP4 only — cells `self-update-merge-config-11`, `-12`, `-13`

## Reality gate

| Check | Result | Evidence |
|---|---|---|
| MODE FIT | PASS | Unchanged high-risk lane |
| REPO FIT | PASS | `ensure_config` (`mod.rs:846-869`), `backup_and_recreate`/`persist_validated` (`write.rs:31,86`), and both named pre-existing tests confirmed to exist exactly as cells assume |
| ASSUMPTIONS | PASS | `{root:?}` debug-format vs `to_string_lossy()` extraction confirmed byte-identical for all UTF-8 paths (empirically tested by the reviewer); `tempfile` confirmed a real dev-dependency |
| SMALLER PATH | PASS | 3 cells: pure refactor / pure merge / composition reusing existing infra — no reinvention |
| PROOF SURFACE | PASS | All three verify commands confirmed non-tautological (exit non-zero) on the unmodified repo, before and after repair |

## Feasibility matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| Extracting `default_config_json` doesn't change ensure_config's output | Medium | Trace the `{root:?}` debug-format path | Confirmed byte-identical for UTF-8 paths (real home dirs are UTF-8) | PASS |
| `merge_missing_fields` correctly preserves orphaned fields at the string layer | Low | Read the cell's spec against D6 | Confirmed, D6-consistent | PASS |
| D6's "orphan preserved, no warning" promise holds end-to-end through the persist step | **Medium (real finding)** | Trace the full compose path against `Config`'s `deny_unknown_fields` | **Found**: the final `persist_validated`/`Config::load_str` step will refuse (fail closed) if the merged JSON contains a field unknown to the *current* schema — only reachable if a future version ever removes a field, which has never happened. Safe outcome (refusal, not corruption), but the promise doesn't literally hold end-to-end for that hypothetical input. **Repaired**: added a 3rd cell -13 test proving the fail-closed behavior explicitly | PASS (after repair) |
| Schedule has no cycles | Low | `bee cells schedule` | `waves: [[-11],[-12],[-13]]`, no cycles | PASS |

## Plan-checker (adversarial, `bee-review` subagent, opus)

**Verdict: 0 BLOCKER, 2 WARNING** (addressed)

1. Requirement/decision coverage — WARNING (see feasibility matrix row above) — **repaired** with an explicit proving test.
2. Cell completeness — WARNING: cell -11's "byte-identical" claim isn't enforced by a byte-comparison test (only key-presence). Confirmed harmless: the extraction is a verbatim cut-paste, empirically proven byte-safe by the reviewer. Noted, not fixed (would require a new test asserting exact string equality against a hardcoded golden string, which is more brittle than useful here).
3. Dependency correctness — PASS.
4. Key links — PASS, cell -13 correctly forbids reimplementing write.rs's logic.
5. Scope sanity — PASS, no `main.rs` touches anywhere in this slice.

## Cold-pickup cell review

**Verdict: 0 CRITICAL, 3 MINOR** (all confirmed harmless or addressed)

- Cell -11's `{root:?}` concern — confirmed feasible/safe (see matrix).
- Cell -13's "root value is essentially inert" wording — technically imprecise (it IS used, and safely, in the one edge case where `allowed_roots` itself is the missing field — which today already can't happen since a config missing it already fails to load) — cosmetic, not fixed.
- Cell -12's verify could theoretically pass a weak implementation (grep-based verify limitation, inherent to the mechanism, same caveat noted in every prior epic) — the natural correct implementation is also the simplest one, low risk.

## Repairs applied (this validating pass)

- `self-update-merge-config-13`: added a 3rd required test (`merge_config_on_upgrade_fails_closed_on_field_unknown_to_current_schema`) proving the deny_unknown_fields fail-closed interaction explicitly, plus a doc-comment explaining it; verify updated to require it.
- Re-confirmed verify still correctly fails (non-zero) on the unmodified repo.

## Advisor consult (AO2b/AO3)

No advisor configured. Recorded via `bee state advisor-ref record`, anchored to current feature/decision/plan-hash.

## Decision

**READY** — no open CRITICAL/BLOCKER items.

## Gate 3

Gate bypass level `full` covers high-risk lanes — auto-approved, not asked.

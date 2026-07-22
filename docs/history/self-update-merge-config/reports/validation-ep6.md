# Validation report — self-update-merge-config, current slice EP6 (final epic)

**Date:** 2026-07-22
**Lane:** high-risk — final integration slice
**Scope:** EP6 only — cells `self-update-merge-config-18`, `-19`

## Whole-feature coverage (D1-D10)

Since this is the last slice, validating confirmed every locked decision is both implemented and reachable from the real `update` command once cell -18 lands:

| D | Decision | Implementing cell(s) | Reachable from `update` post-EP6 |
|---|---|---|---|
| D1 | releases/latest, no version-pin | -4, -7 | Yes |
| D2 | semver-prefix compare | -3, -4 | Yes |
| D3 | auto restart, no prompt | -15, -16 | Yes |
| D4 | post-restart health check | -16 | Yes |
| D5 | merge = new binary's defaults | -11, -12, -14 | Yes |
| D6 | orphan fields untouched | -12 | Yes |
| D7 | backup before merge | -13, -14 | Yes |
| D8 | checksum-verify before overwrite | -1, -2, -6, -8 | Yes |
| D9 | rollback binary+config on failure | -15, -16 | Yes |
| D10 | fail-closed when checksum missing | -1, -2, -6, -8 | Yes |

Before this slice, `download_and_verify`/`perform_update`/`compare` were reachable only from unit tests — dead code from the real CLI's perspective. Cell -18 closes that gap.

## Reality gate

| Check | Result | Evidence |
|---|---|---|
| MODE FIT | PASS | Final integration slice, unchanged high-risk lane |
| REPO FIT | PASS | Real shipped signatures confirmed: `UpdateStatus` (2 variants), `FetchError`/`RolloutError` (thiserror, Display works), `default_config_path()`, `Config::load_file`, main.rs insertion points all re-confirmed post-EP1-EP5 |
| ASSUMPTIONS | PASS (after repair) | Cell -18's Rust composition confirmed to actually compile (exhaustive match arms over real enum variants) — found and fixed a missing `PathBuf` qualification that would NOT have compiled as originally written |
| SMALLER PATH | PASS | 2 cells: wire the verb, then prove it end-to-end |
| PROOF SURFACE | PASS | Both verify commands confirmed non-tautological (exit non-zero) against the current repo, before and after repairs |

## Feasibility matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| Cell -18's match arms are exhaustive over the real shipped enums | Medium | Compare against actual `UpdateStatus`/`FetchError`/`RolloutError` definitions | Confirmed exhaustive, correct variant names | PASS |
| Cell -18's config-path resolution snippet compiles | **Medium (real finding)** | Check for required imports | **Found**: `main.rs` has no `use std::path::PathBuf` — the literal snippet wouldn't compile. **Repaired**: fully-qualified `std::path::PathBuf::from` | PASS (after repair) |
| Cell -19's CI runner matches existing precedent | Low | Check `macos-install-smoke`'s and the build matrix's pinned runner | Was `macos-latest` (drift); existing precedent is `macos-14`. **Repaired** | PASS (after repair) |
| Cell -19's env-var translation (SMOKE_FROM_VERSION → install.sh's HERDR_GO_VERSION) is unambiguous | Medium | Check install.sh's actual env var name | Confirmed install.sh only reads `HERDR_GO_VERSION`; the translation is required, not optional. **Repaired**: made explicit as a required truth | PASS (after repair) |
| Schedule has no cycles | Low | `bee cells schedule` | `waves: [[-18],[-19]]`, no cycles | PASS |

## Plan-checker (adversarial, `bee-review` subagent, opus)

**Verdict: 0 BLOCKER, several WARNING/MINOR** (all fixed)

1. Requirement coverage — PASS, correct composition order and fail-closed reporting.
2. Cell completeness — 2 WARNINGs (missing import; misleading "same as normal-startup" wording implying `ensure_config`-style creation) — **both fixed**.
3. Dependency correctness — PASS.
4. Key links — PASS, correctly forbids touching already-shipped composed units.
5. Scope sanity — PASS, pure "wire + prove", no new product decisions.
6. Whole-feature coverage — PASS, see table above.

## Cold-pickup cell review

**Verdict: 0 CRITICAL, 4 MINOR** (all fixed)

- Cell -18: missing `PathBuf` import (would not compile) — **fixed**. Verify's `grep -qi 'update'` help-text check is weak but meaningfully red today (zero "update" occurrences currently) — left as-is, matches acceptable prior pattern.
- Cell -19: `macos-latest` vs `macos-14` runner drift — **fixed**. Implicit env-var translation — **fixed**, now an explicit required truth.

## Repairs applied (this validating pass)

- `self-update-merge-config-18`: fixed `PathBuf` qualification, clarified load-only (never-create) semantics in wording.
- `self-update-merge-config-19`: pinned `macos-14` runner, made the `HERDR_GO_SMOKE_FROM_VERSION` → `HERDR_GO_VERSION` translation an explicit required truth.
- Re-confirmed both verify commands still correctly fail (non-zero) on the current repo state after repairs.

## Advisor consult (AO2b/AO3)

No advisor configured. Recorded via `bee state advisor-ref record`, anchored to current feature/decision/plan-hash.

## Decision

**READY** — no open CRITICAL/BLOCKER items. This is the final slice; once shipped, every locked decision D1-D10 is implemented and reachable from the real `herdr-go update` command.

## Gate 3

Gate bypass level `full` covers high-risk lanes — auto-approved, not asked.

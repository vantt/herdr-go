# Validation report — self-update-merge-config, current slice EP3

**Date:** 2026-07-22
**Lane:** high-risk (security-critical slice: D8/D10 checksum verification + fail-closed)
**Scope:** EP3 only — cells `self-update-merge-config-6`, `-7`, `-8`

## Reality gate

| Check | Result | Evidence |
|---|---|---|
| MODE FIT | PASS | Unchanged high-risk lane; this slice is the audit/security flag's core substance |
| REPO FIT | PASS | `scripts/generate-checksums.sh:31-35` confirmed checksums.txt line format (`<hash>  <filename>`, sha256sum-compatible); `release.yml:228-258` confirmed the shipped checksums job publishes exactly `checksums.txt`; `install.sh:89-104` + `release.yml` confirmed all 4 asset-name/target-triple mappings |
| ASSUMPTIONS | PASS | `sha2 0.11.0` confirmed already transitively locked (`Cargo.lock`), promoting to direct adds no new crate |
| SMALLER PATH | PASS | 3 cells, minimal: pure checksum logic / pure asset-selection / composition, each independently testable |
| PROOF SURFACE | PASS (tightened) | All three verify commands confirmed non-tautological (exit 1) on the unmodified repo, both before and after tightening |

## Feasibility matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| checksums.txt format matches what `generate-checksums.sh` produces | Low | Read the script | Line-for-line match confirmed | PASS |
| Asset-name-to-target-triple mapping is complete and correct for all 4 published platforms | Low | Cross-check `install.sh` + `release.yml` | All 4 combos confirmed, unsupported combos correctly return `None` | PASS |
| Fail-closed logic has no path that silently treats missing/garbage checksums as verified | High (the whole point of D10) | Walk every corruption/absence scenario | Empty/garbage checksums.txt, entries-for-other-assets-only, and HTTP-error-as-content all confirmed to degrade to refusal — reviewed explicitly by the security persona panel | PASS |
| Verify commands actually force the fail-closed test cases to exist (not just some passing test) | High | Re-derive and re-check the verify logic | **Initially a gap**: `cargo test --quiet update` alone would pass with happy-path-only tests. **Repaired**: verify now greps for exact named test functions covering every fail-closed branch | PASS (after repair) |
| Schedule has no cycles | Low | `bee cells schedule` | `waves: [[-6],[-7],[-8]]` (auto-serialized on shared `mod.rs`/`github.rs` file overlap, not a real dependency issue), no cycles | PASS |

## Plan-checker (adversarial, `bee-review` subagent, opus) — security persona panel

**Verdict: 0 BLOCKER, 3 WARNING** (all addressed or explicitly carried forward)

1. Requirement/decision coverage — PASS. D10's fail-closed logic is gap-free within this slice.
2. Cell completeness — PASS. checksums.txt format correctly specified.
3. Dependency correctness — PASS.
4. Key links — PASS.
5. Scope sanity — PASS. No EP4/EP5 creep.
6. **Security (fail-closed correctness)** — WARNING: D8's "verify before overwrite" is only enforceable across EP3+EP5 together — EP3 alone cannot guarantee it. **Addressed**: recorded as an explicit EP5 carry-forward invariant in `approach.md` (EP5 must source binary bytes exclusively from `download_and_verify`'s `Ok` result, never a second raw-download path) — a doc comment requirement was also added to cell -8's action for the code itself. Minor WARNING: downloads should call `.error_for_status()` mirroring existing discipline — **fixed**, added to cell -8.

## Cold-pickup cell review

**Verdict: 0 CRITICAL, 1 WARNING (fixed), 1 MINOR (noted, not blocking)**

- WARNING: verify commands for cells -6/-8 proved compile+presence+some-test-passes, not that the specific fail-closed test cases were written. **Fixed**: both verify commands now require exact test function names (`sha256_hex_matches_known_vector`, `parse_checksums_extracts_two_entries`, `checksum_matches_is_case_insensitive_and_rejects_wrong_hash` for -6; `verify_succeeds_when_checksum_matches`, `fails_closed_when_checksums_txt_asset_missing`, `fails_closed_when_entry_missing_for_asset`, `fails_when_checksum_mismatches` for -8), re-confirmed still exit 1 on the unmodified repo.
- MINOR: cell -8 is the heaviest of the three (assets field, `find_asset`, 3 new error variants, composition, 4 tests) but stays within one file and one coherent capability — not split further.

## Repairs applied (this validating pass)

- `self-update-merge-config-6`: action names 3 required test functions; verify greps for them.
- `self-update-merge-config-8`: action adds `error_for_status()` requirement + EP5 doc-comment invariant; names 4 required test functions; verify greps for them.
- `approach.md`: added "EP5 carry-forward invariant" section.
- Re-confirmed all three verify commands still correctly fail (exit 1) against the unmodified repo post-repair.

## Advisor consult (AO2b/AO3)

No advisor configured. Recorded via `bee state advisor-ref record`, anchored to current feature/decision/plan-hash.

## Decision

**READY** — no open CRITICAL/BLOCKER items. The fail-closed architecture is sound; the one cross-slice concern (D8's overwrite invariant) is explicitly carried forward to EP5, not silently assumed.

## Gate 3

Gate bypass level `full` covers high-risk lanes — auto-approved, not asked.

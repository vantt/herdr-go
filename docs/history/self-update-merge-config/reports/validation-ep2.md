# Validation report — self-update-merge-config, current slice EP2

**Date:** 2026-07-22
**Lane:** high-risk
**Scope:** EP2 (version awareness) only — cells `self-update-merge-config-3`, `self-update-merge-config-4`

## Reality gate

| Check | Result | Evidence |
|---|---|---|
| MODE FIT | PASS | Unchanged from EP1 — same high-risk lane, same plan.md rationale |
| REPO FIT | PASS | `Cargo.toml` confirmed: reqwest 0.12, tokio (full), serde_json, serde derive all present; `src/lib.rs:19-22` VERSION const confirmed; `build.rs:20` fingerprint format confirmed (`{semver} ({sha}, {timestamp})`, no trailing suffix — corrected an inaccurate example found during review) |
| ASSUMPTIONS | PASS | Real GitHub tags confirmed plain `vX.Y.Z` (`git ls-remote --tags origin`: v0.1.0, v0.1.1, v0.1.2), resolving approach.md's deferred question — manual parse is sufficient, no `semver` crate needed |
| SMALLER PATH | PASS | 2 cells, minimal: pure compare logic isolated from network fetch |
| PROOF SURFACE | PASS (after repair) | Both cells' original verify (`cargo test --quiet update`) was a tautology — 0 tests filtered still exits 0. Tightened to require the target file, a `#[test]` attribute, and correct module wiring; both now confirmed to exit 1 against the unmodified repo |

## Feasibility matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| Real GitHub tags are plain semver, no prerelease metadata | Low | `git ls-remote --tags` | v0.1.0, v0.1.1, v0.1.2 confirmed | PASS |
| `reqwest`/`tokio`/`serde_json` are usable without new dependencies | Low | Inspect Cargo.toml | All three present with needed features | PASS |
| An async HTTP-transport-injection pattern exists in-repo to reuse | Medium | Inspect `checks.rs:793` and `main.rs:299` | **Neither is async-injectable** — `offer_service_restart_with` is sync, `main.rs:299`'s reqwest use is a bare non-injected client. **Repaired**: cell -4 rewritten to require a fetch/parse split (pure sync parser, thin untested async wrapper) instead of a nonexistent precedent | PASS (after repair) |
| Cells' verify commands are non-tautological | High (would silently pass empty work) | Run verify against the unmodified repo | Both now exit 1 (correctly fail) | PASS (after repair) |
| Schedule has no cycles | Low | `bee cells schedule` | `waves: [[-3],[-4]]`, no cycles, re-confirmed post-patch | PASS |

## Plan-checker (adversarial, `bee-review` subagent, opus)

**Verdict: 0 BLOCKER, 2 WARNING** (both addressed)

1. Requirement/decision coverage — PASS, D1/D2's sub-rules each map to a cell must_have.
2. Cell completeness — WARNING (MINOR): cell -3's fingerprint example string had a fabricated ` — herdr-go` suffix not present in the real format. **Fixed**.
3. Dependency correctness — PASS, -4 genuinely needs -3's compare output for D2's decision.
4. Key links — PASS, `mod github;` + composition correctly required.
5. Scope sanity — PASS, both cells correctly stay out of EP3 (asset/checksum parsing) and EP6 (CLI wiring).

## Cold-pickup cell review

**Verdict: 0 CRITICAL, 2 MINOR** (both fixed)

- Cell -3: inaccurate fingerprint example (see above). **Fixed.**
- Cell -4: cited a synchronous precedent (`offer_service_restart_with`) for an async-transport injection with no async prior art in the repo — under-specified, not infeasible. **Fixed**: rewritten to the fetch/parse split, which sidesteps needing new async-injection machinery entirely.

## Repairs applied (this validating pass)

- `self-update-merge-config-3`: corrected the fingerprint-format example in `action`.
- `self-update-merge-config-4`: rewrote `action`/`must_haves` to the fetch/parse-split shape (pure sync parser + thin untested async wrapper), replacing the misleading sync-precedent citation.
- Both cells: tightened `verify` to eliminate the "0 tests filtered = pass" tautology.
- Re-ran `bee cells schedule` post-patch: unchanged, `[[-3],[-4]]`, no cycles.

## Advisor consult (AO2b/AO3)

No advisor configured. Re-recorded via `bee state advisor-ref record` (prior ref from EP1 was stale — a new decision had landed since). Anchored to current feature, newest decision id, and `plan.md` sha256.

## Decision

**READY** — no open CRITICAL/BLOCKER items remain.

## Gate 3

Gate bypass level `full` covers high-risk lanes — auto-approved, not asked.

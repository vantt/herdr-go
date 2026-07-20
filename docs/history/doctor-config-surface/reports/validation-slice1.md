# Validation Report — doctor-config-surface, Slice 1 (write foundations)

**Date:** 2026-07-20
**Mode:** high-risk
**Cells:** doctor-config-surface-1, -2, -3
**Gate-bypass level active:** full

## Reality Gate

| Dimension | Verdict | Evidence |
|---|---|---|
| MODE FIT | PASS | 6 risk flags incl. a hard-gate flag (audit/security) per plan.md's mode-gate record; high-risk is correct, not over- or under-scoped. |
| REPO FIT | PASS | `src/doctor.rs`, `src/config/mod.rs`, `src/main.rs`, `tests/rename_contract.sh`, `Cargo.toml` all inspected directly; cited line ranges for `prepare_token_directory`/`write_new_token`/`validate_token_protection`, `RawConfig`, `Secrets`, and the migration-seam test confirmed present and matching cell claims (one citation off by a few lines, non-load-bearing, noted below). |
| ASSUMPTIONS | PASS | `rpassword` confirmed absent from `Cargo.toml` today (cell 1's "one new dependency" claim holds). No prompt/TTY crate anywhere in `src/`. |
| SMALLER PATH | PASS | tiny/small caps (0-1 flags, ≤3 files) don't fit 6 flags + a hard-gate flag; spike doesn't apply (no single yes/no proof — feasibility already established in Discovery). |
| PROOF SURFACE | PASS | `cells schedule` reports 2 waves, 0 cycles: {cell-1, cell-2} parallel, {cell-3} depends on cell-2 (file-ownership serialization on `src/config/mod.rs`, not a real API dependency — cell-3 reuses existing `mod.rs` helpers, not cell-2's new `write.rs`). |

## Baseline

Recorded green at plan time (`cargo test`, `cargo clippy`). Re-verified fresh this session:
- `cargo test --quiet` → 97 passed, 0 failed.
- `cargo clippy --quiet -- -D warnings` → silent (0 warnings).
- `bash tests/rename_contract.sh` → **found red** at session start (2 unrelated pre-existing regressions, see below) → fixed via a separate fix-first tiny lane (`readme-demo-bind-restore`, cells -1 and -2, both capped) → now green.

**Baseline-gate discovery (handled, not part of this feature's scope):** `tests/rename_contract.sh` had drifted in 5 assertions (readme-rewrite, commit `c7b7ea9`, deliberately relocated content from README.md into `docs/`, but the contract test wasn't updated to match) and had one test-methodology false-positive (`assert_after_preflight`'s first-match search collided with cross-platform-install's `--uninstall` early-exit branch, which shares 3 mutation-pattern substrings with the real post-preflight install sequence — confirmed by inspection that the real occurrences at lines 213/215/237 are correctly ordered after the systemd preflight check at line 114; this was a test false-positive, not a live regression). Fixed, verified, capped, committed as its own lane, unrelated decisions/cells kept separate from doctor-config-surface's own decision log.

**Known environment gap (not a code defect):** `rg` (ripgrep) is not installed as a real binary in this sandbox — only available as Claude Code's own interactive-shell alias, invisible to `bash script.sh` subprocesses. 3 guards inside `tests/rename_contract.sh` (`if rg ...; then fail ...`) silently no-op here rather than failing (swallowed exit 127 inside `if`). Recorded in `docs/history/learnings/critical-patterns.md`. Not blocking — those guards default to "pass" when unrunnable, and are expected to run for real in CI.

## Feasibility Matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| `rpassword` masked-entry crate is a real, compatible, single-purpose dependency | LOW | Confirm absence today + known ecosystem status | `grep rpassword Cargo.toml` → no match; well-known cross-platform crate (unix termios + `windows-sys`), edition-2021 compatible. Live crates.io version-resolution not checkable in this sandbox (no network) — flagged WARNING by the plan-checker, not blocking. | READY WITH CONSTRAINT (noted, not gating) |
| `write_new_token` can be reused as-is for D10's "replace in place" | MEDIUM | Inspect the helper's actual semantics | `write_new_token` opens with `create_new(true)` → refuses an existing path (`AlreadyExists`, confirmed against its own test). Cell 3 originally said "reuse write_new_token... REPLACE in place" without reconciling this. **Fixed**: cell 3 now specifies temp-file-plus-atomic-rename, never a bare `write_new_token` call against an existing path. | READY (cell repaired) |
| `Secrets`' pre-existing `#[derive(Debug, ...)]` doesn't leak token fields under cell 3's "no secret in Debug output" truth | MEDIUM | Inspect the struct | `#[derive(Debug, Clone, Default)]` at `src/config/mod.rs:42` renders all 3 token fields verbatim under `{:?}`; no live caller exists today, but the truth is only real once this cell adds redaction. **Fixed**: cell 3 now scopes this explicitly. | READY (cell repaired) |
| `secrets_read_from_env_only` test still expresses the right thing once cell 3 adds the env-file fallback | MEDIUM | Inspect the test | Confirmed at `src/config/mod.rs:965-969`: asserts `from_env().github_token.is_none()` with empty process env — true only when no `herdr-go.env` happens to exist on the test machine once the fallback exists. **Fixed**: cell 3 now requires updating/reworking this test as part of its own scope. | READY (cell repaired) |
| `cells schedule` reports zero cycles, correct wave shape | required | `bee cells schedule --feature doctor-config-surface --json` | 2 waves: `{cell-1, cell-2}` then `{cell-3}`; 0 cycles, 0 unsatisfiable deps. | PASS |

## Plan-Checker (adversarial, bee-review / opus)

Dispatched as a background review-tier agent against CONTEXT.md, plan.md, all 3 cells, and the actual source they cite. Full findings: see agent transcript (session-local). Summary:

- **Decision coverage:** complete for Slice 1's "write foundations" scope. D5/D6/D7/D8/D9/D10/D12/D13/D15 all covered across the 3 cells; D3/D11/D14/D16/D17 correctly deferred to Slice 2. No locked decision silently dropped.
- **BLOCKER (now resolved):** `tests/rename_contract.sh` red at baseline, independent of this feature — handled via the `readme-demo-bind-restore` fix-first lane (above).
- **WARNING (now resolved via cell repair):** cell 3's "reuse `write_new_token`... REPLACE in place" instruction, `Secrets`' Debug-derive leak risk, `secrets_read_from_env_only` test-update gap — all three folded into cell 3's action/must_haves/prohibitions.
- **WARNING (accepted, non-blocking):** `rpassword` live version-resolution unverifiable offline; cell 3→cell 2 dependency is a file-ownership serialization guard, not a real API dependency (correctly modeled either way since both cells cannot land in the same wave without collision on `src/config/mod.rs`).
- **Minor (accepted, folded into cell 1):** the 10 checks in `src/doctor.rs` are inline blocks inside `run()`, not separate functions — cell 1's action now specifies the split shape explicitly.
- **Overall verdict: READY WITH CONSTRAINTS → all constraints applied → READY.**

## Cell Review (cold pickup)

- **doctor-config-surface-1:** COLD-PICKUP READY (after clarifying the checks.rs split shape and confirming the baseline is green).
- **doctor-config-surface-2:** COLD-PICKUP READY, no changes needed — cleanest of the three per the reviewer.
- **doctor-config-surface-3:** COLD-PICKUP READY (after the replace-in-place mechanism, Debug-redaction scope, and test-update scope were made explicit).

## Decision

**READY.**

## Gate 3 — Execution Approval

Advisor: not configured in `.bee/config.json` (AO2b: not a hard dependency, recorded and proceeding — `state advisor-ref record`, digest above). `advisor_ref` recorded non-stale against the current feature, newest decision id, and `plan.md` sha256.

Gate-bypass level `full` is active: the human has deliberately lifted the high-risk/hard-gate human-checkpoint floor for Gates 1-3. Per that level, execution is self-approved rather than asked — recorded below, machine report linked, work proceeds to swarming.

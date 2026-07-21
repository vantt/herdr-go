# Validation: web-create-sheet, Phase 1 (slice 5 of new-shell-new-agent)

## Reality Gate

| Check | Result | Evidence |
|---|---|---|
| MODE FIT | PASS | 0 risk flags; 5 product files (api.ts, main.ts, terminal.ts, create-sheet.ts [new], switcher.ts) exceed `small`'s 3-file cap; story-sized behavior (destination fetch/render, two submit verbs, inline errors, post-create navigation). `standard` correctly chosen per plan.md's own accounting. |
| REPO FIT | PASS | Every referenced file read directly and confirmed to exist: `web/src/api.ts`, `web/src/main.ts`, `web/src/views/{switcher,terminal}.ts`, `web/test/{api,switcher,terminal}.test.ts`, `docs/specs/web-api.md`, `web/package.json` (`typecheck`/`test` scripts exist). Backend contract cross-checked against `src/web/create.rs` and `src/web/api.rs` directly (plan-checker) — every field name cell 1 assumes is real. |
| ASSUMPTIONS | PASS | vitest file-targeting (`npm run test -- --run test/<file>.test.ts`) proven by direct execution (12/12 passed against the existing `api.test.ts`). `terminal.ts` only reads `agent.kind`/`agent.display`/`agent.pane_id` — confirmed by direct read, supports S5's minimal-reference approach. jsdom's unimplemented-canvas errors are non-fatal (observed directly in this session's own full-verify run: the error printed to stderr, the test still passed) — informs cell 2's proof strategy. |
| SMALLER PATH | PASS | File count (5) exceeds `small`'s cap; no smaller split indicated. Cell boundaries follow real coupling (data layer / routing widen / view / wiring), matching the `web-create-endpoints` "coupling not modules" lesson. |
| PROOF SURFACE | PASS | Every cell's verify command (`cd web && npm run typecheck && npm run test -- --run test/<file>.test.ts`) proven runnable. New test files/assertions don't exist yet on the current tree — confirmed red by absence. |

## Feasibility Matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| Backend response/error shapes match `web-api.md` | LOW | Direct source read | `src/web/create.rs`, `src/web/api.rs` read by plan-checker; field names match exactly | PASS |
| `terminal.ts` can render from a minimal reference (no full `AgentRow`) | MEDIUM | Confirm only `kind`/`display`/`pane_id` are read | Direct read of `terminal.ts` (both this session's exploring fresh-eyes pass and this validating pass) | PASS |
| `renderTerminal()` doesn't crash under jsdom (xterm needs canvas) | MEDIUM | Empirical proof or a safer test strategy | jsdom's not-implemented errors observed non-fatal in this session's own full-verify run; cell 2 revised to use a pure-function proof as the primary truth-check rather than relying on this | ADDRESSED (design changed rather than proven either way) |
| Schedule has no cycles, matches plan's approach | LOW | `cells schedule --json` | Run twice (before and after cell edits): `[[1,2],[3],[4]]`, zero cycles both times | PASS |
| The S5 minimal-reference type is single-owned and consistently typed across cells 2/3/4 | HIGH (structural) | Plan-checker cross-cell read | 2 plan-checker passes: iteration 1 found 2 BLOCKERs (no owner, inconsistent field sets; cell 1's success payload unspecified), fixed by pinning `NewPaneRef` to cell 2 and adding a cell-1 truth; iteration 2 confirmed both resolved | PASS (after fix) |

## Plan-Checker (2 iterations, both logged)

**Iteration 1 — CHANGES REQUIRED:** 2 BLOCKERs (S5 type ownership/consistency; cell 1's success payload unspecified), 3 WARNINGs (missing cell3→cell2 dependency edge; cell 2's jsdom/xterm proof risk; cell 2 mischaracterizing where the type decision was deferred).

**Fixes applied:** cell 2 now solely owns and exports `NewPaneRef {pane_id, workspace_id, label, name?}` from `web/src/main.ts`; cell 1 gained an explicit truth requiring `createPane`/`createAgent`'s success branch to expose `pane_id`/`tab_id`(+`name`); cell 3 gained a dependency on cell 2 plus `main.ts` in `read_first` and now imports `NewPaneRef` rather than restating it; cell 2's proof strategy shifted to a pure `{kind, display}` derivation function, demoting a full DOM `renderTerminal()` call to an optional smoke test; cell 4 clarified the `loadHealth()` extension needed for S4 and requires passing cell 3's `NewPaneRef` through unmodified.

**Iteration 2 — READY:** both BLOCKERs and all three WARNINGs confirmed resolved; wave shape unchanged (`[[1,2],[3],[4]]`, zero cycles); no new issue introduced (field sets consistent across cells 2/3/4, no intra-wave file collision, no prohibition conflict).

## Cold-Pickup Cell Review

1 CRITICAL (cell 1's success-payload contract — same root cause as plan-checker BLOCKER 2, fixed together), 3 MINOR (cell2/cell3 field-set disagreement — fixed via `NewPaneRef` ownership; cell 3's dangling `D3` citation — parent CONTEXT.md added to `read_first` with an inline gloss; cell 4's "reuse the existing health signal" under-description — clarified that `loadHealth()` must be extended, not merely read). All addressed in the same edit pass as the plan-checker fixes.

## Process Note

Before this validation began, `approved_gates.execution` and `mode` were found stale-`true`/stale-`high-risk` in `.bee/state.json`, carried over from the just-closed `web-create-endpoints` feature (the generic `state set --feature` transition used during exploring/planning does not reset gates the way the dedicated `state start-feature` verb does). Caught during orient, before any cell was claimed against the stale flag. Corrected via `state gate --name execution --approved false` and `state set --mode standard`; filed as P2 friction for `bee-grooming` (layer: state).

## Approval Block

**Decision:** READY

**Mode:** standard, 0 risk flags. No hard-gate flag. `gate_bypass_level=full` covers Gate 3 at every lane, so no human question and no advisor consult (advisor consult is a mechanical precondition for `high-risk`/hard-gate work only, per AO2b/AO3 — this lane doesn't trigger it).

**Scope of this approval:** Phase 1 cells only (`web-create-sheet-1` through `-4`). Phase 2 (S1 disambiguation) is out of scope and returns to planning as its own future slice.

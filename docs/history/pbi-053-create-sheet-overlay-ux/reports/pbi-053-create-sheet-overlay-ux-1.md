# pbi-053-create-sheet-overlay-ux-1

**Status:** [DONE]

**Outcome:** Reworked `renderCreateSheet` (`web/src/views/create-sheet.ts`) into two collapsed-by-default dropdown fields (Destination, Type) plus an explicit "New" button that is the sole creation trigger (D1/D2). Opening one dropdown closes the other (D7). Popup positioned `position:absolute` under a `position:relative` trigger wrapper, never `position:fixed` (D4). All existing per-row destination/type data and markup preserved (D8). `web/test/create-sheet.test.ts` rewritten: all 7 existing cases now drive open→select→New instead of a direct action-row click, plus 1 new test for D7's mutual-exclusion behavior (8 tests total, all passing). No changes to `web/src/api.ts` or `web/src/views/switcher.ts` (D9).

**Files touched:** `web/src/views/create-sheet.ts`, `web/src/styles.css`, `web/test/create-sheet.test.ts`

**Verification:** `cd web && npm run test -- --run create-sheet && npm run typecheck` — 8/8 tests passed, typecheck clean. Full trace and verification evidence: `.bee/cells/pbi-053-create-sheet-overlay-ux-1.json`.

**Commit:** `6a8959b` — `feat(pbi-053-create-sheet-overlay-ux-1): collapse create-sheet destination/type into dropdowns with explicit New button`

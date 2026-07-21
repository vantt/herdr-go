# web-create-sheet-4

**Status:** [DONE]

Wired the create FAB (D1) and create-sheet.ts's overlay (cell 3) into
switcher.ts, health-gated per S4 via the existing `loadHealth()` call, and
threaded `onCreated` through `main.ts` to navigate into the new pane's
terminal detail (D6) on a successful create.

## Files touched

- `web/src/views/switcher.ts` — FAB markup/wiring, `onCreated` prop, extended
  `loadHealth()` to drive the FAB's disabled state.
- `web/src/main.ts` — switcher case now passes `onCreated` calling
  `navigate({ name: "terminal", agent: ref })`.
- `web/test/switcher.test.ts` — 5 new tests for FAB health-gating, sheet
  open, and `onCreated` pass-through; updated one pre-existing call site for
  the new required prop.
- `web/src/styles.css` — deviation (auto-added, not in the cell's `files`
  list): `.fab` and `.create-sheet` + row styling, needed for the FAB's
  bottom-right position/disabled state and the sheet's overlay chrome.
  Neither this cell nor cell 3 had `styles.css` in scope despite needing it
  — see the cell trace for the full deviation note.

Full trace/evidence: `.bee/cells/web-create-sheet-4.json`.

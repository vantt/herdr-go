---
artifact_contract: bee-plan/v1
artifact_readiness: implementation-ready
mode: tiny
---

# terminal reply UI: bottom bar + toggled reply overlay

## Problem

- The reply input sheet is permanently on screen: `.reply-sheet` sets
  `display: flex`, which outranks the UA `[hidden] { display: none }` rule, so
  the `hidden` attribute the JS toggles has no effect — the overlay never hides.
- The control bar sits at the top; the user wants it fixed at the bottom.
- "Press Enter (submit)" defaults to checked; the user wants Enter to NOT submit
  by default.

## Approach

- Move the control bar (`.term-bar`) to the bottom of the flex column and add a
  Reply button to it; drop the floating `.reply-fab`.
- Make `hidden` work on the sheet: add a `.reply-sheet[hidden] { display: none }`
  rule (higher specificity than `.reply-sheet`). The Reply button opens it,
  Cancel/Send closes it — start hidden.
- Move the bar's safe-area padding from top to bottom.
- Default the submit checkbox to unchecked.

## Mode gate

Risk flags: 1 (existing covered behavior — terminal.test.ts). Files: 2
(`web/src/views/terminal.ts`, `web/src/styles.css`). → `tiny`.

## Verify

- `cd web && npm run bundle` succeeds; `npm run test -- --run` green.
- Browser (emulated phone): terminal opens with NO input overlay and the bar at
  the bottom; tapping Reply shows the overlay; Cancel hides it; the submit
  checkbox is unchecked by default.

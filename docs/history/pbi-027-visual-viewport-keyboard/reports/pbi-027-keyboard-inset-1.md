# pbi-027-keyboard-inset-1

**Status:** [DONE]

**Outcome:** Added the pure exported `computeKeyboardInset` helper and wired a feature-detected `window.visualViewport` resize listener into `openReply`/`closeReply` so the reply-sheet stays above the OS keyboard (D1-D4).

**Files touched:**
- `web/src/views/terminal.ts`
- `web/test/terminal.test.ts`

**Verification:** `cd web && npm run test -- --run` — passed. Full trace and evidence: `.bee/cells/pbi-027-keyboard-inset-1.json`.

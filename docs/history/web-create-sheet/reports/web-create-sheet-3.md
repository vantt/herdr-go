# web-create-sheet-3

**Status:** DONE
**Worker:** Tim
**Commit:** de1cb3d

## Outcome

Added `web/src/views/create-sheet.ts` rendering the create bottom sheet
(destination list on top, Shell then preset rows below), backed by
`web/test/create-sheet.test.ts` covering S2 caveats (null/stale path), Shell-
first/preset-order, both create paths building the `NewPaneRef` handed to
`onCreated`, the overlapping-tap guard, and inline S3 error handling that
keeps the sheet open.

## Files touched

- `web/src/views/create-sheet.ts` (new)
- `web/test/create-sheet.test.ts` (new)

Full trace and verification evidence: `.bee/cells/web-create-sheet-3.json`.

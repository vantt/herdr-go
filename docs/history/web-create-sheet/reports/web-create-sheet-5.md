# web-create-sheet-5

**Status:** [DONE]
**Worker:** Otto

## Outcome

Added a pure `collisionSuffixes()` function in `web/src/views/create-sheet.ts` that flags destinations whose `{label, path}` collides with another entry in the currently fetched list (including two entries both having `path: null`). `renderDestinations()` appends ` · <last-4-of-workspace_id>` to a colliding destination's label only; non-colliding destinations render byte-identical to Phase 1. `createPane`/`createAgent` calls are untouched and keep using the full `dest.workspace_id`.

## Files touched

- `web/src/views/create-sheet.ts`
- `web/test/create-sheet.test.ts`

## Verification

`cd web && npm run typecheck && npm run test -- --run test/create-sheet.test.ts` — passed (7/7 tests, including the 6 pre-existing unmodified assertions).

Full trace and evidence: `.bee/cells/web-create-sheet-5.json`.

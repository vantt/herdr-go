# pbi-052-group-header-chevron-status-1

**Status:** [DONE]

**Outcome:** Removed `.status-badge` from workspace group headers; `.workspace-chevron` now carries status color, wash background, and reused pulse/blink decor via a non-rotating `.workspace-chevron-wrap` wrapper, plus a new `.sr-only` accessible status span. Unknown status gets color+wash with no animation; zero-agent groups stay plain per D7.

**Files touched:**
- `web/src/views/switcher.ts`
- `web/src/styles.css`
- `web/test/switcher.test.ts`

**Verify:** `cd web && npm run test -- --run switcher && npm run typecheck` — passed (21/21 tests, clean typecheck).

Full trace and evidence: `.bee/cells/pbi-052-group-header-chevron-status-1.json`.

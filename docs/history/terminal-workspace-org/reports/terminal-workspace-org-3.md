# terminal-workspace-org-3

[DONE] Extracted a pure exported `groupByWorkspace()` helper in `web/src/views/switcher.ts`; `renderList` branches on its result — with exactly 1 distinct `workspace_id` it keeps today's flat `<ul><li>` markup (plus the new always-on `tab_label` sub-caption, per D3); with 2+ it wraps rows in collapsible per-workspace `<section>`s sorted alphabetically by `workspace_label` (D7), each header showing `workspace_label` plus a status badge for `workspace_status` reusing the existing `.status-badge`/`.status-dot` classes (D4), all expanded by default (D8) with collapse state kept in an in-memory `Set` scoped to the `renderSwitcher` closure — never persisted (D6). Pull-to-refresh touch handlers untouched.

Files touched: `web/src/views/switcher.ts`, `web/src/styles.css`, `web/test/switcher.test.ts` (new).

Verify: `cd web && npm run typecheck && npm run test -- --run` — tsc clean, Test Files 4 passed (4), Tests 18 passed (18) including 3 new `groupByWorkspace` cases (empty array, 1 group, 2+ groups sorted alphabetically).

Full trace/evidence: `.bee/cells/terminal-workspace-org-3.json`.

Commit: `4bb0a36`.

No deviations, no Advisor Consults, no outstanding questions.

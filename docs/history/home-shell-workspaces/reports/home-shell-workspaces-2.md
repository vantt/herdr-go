# home-shell-workspaces-2 — [DONE]

Frontend consumes the widened `GET /api/agents` `{agents, shells}` shape and renders shell-only workspaces on home.

- **Outcome:** `fetchAgents()` returns `{agents, shells}` (new `ShellRow`/`AgentsResponse` types); the switcher renders each shell pane in a zero-agent workspace as its own row (folder path primary, `Shell · <tab>` caption, no watermark, no status badge — D1/D2/D6), groups shells by their own `workspace_id` alongside agent groups (D4), hides a group's header badge when it has zero agent rows (D7), and taps a shell row into terminal detail via a `NewPaneRef` with `label = path ?? workspace_label` (D5).
- **Files touched:** `web/src/api.ts`, `web/test/api.test.ts`, `web/src/views/switcher.ts`, `web/test/switcher.test.ts`
- **Verify:** `cd web && npm run typecheck && npm run test -- --run test/api.test.ts test/switcher.test.ts` → tsc clean; 39 passed (2 files). Recorded passed.
- **Commit:** `5b0f7fc`
- **Trace / evidence:** `.bee/cells/home-shell-workspaces-2.json`

`main.ts` untouched: its `agents ? switcher : login` truthiness and the `onSelect` call site both remain valid under the widened `onSelect(target: AgentRow | NewPaneRef)` signature.

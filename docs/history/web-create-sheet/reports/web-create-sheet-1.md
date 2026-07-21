# web-create-sheet-1

[DONE] — added `fetchCreateOptions`, `createPane`, `createAgent` to `web/src/api.ts`, mirroring the `fetchAgents`/`sendReply` pattern; `createPane`/`createAgent` return a discriminated success/error result that preserves the backend's `{error}` message on 400/409/502 without throwing, and the success branch carries `pane_id`/`tab_id` (plus `name` for the agent case).

Files touched:
- `web/src/api.ts`
- `web/test/api.test.ts`

Full trace/evidence: `.bee/cells/web-create-sheet-1.json`

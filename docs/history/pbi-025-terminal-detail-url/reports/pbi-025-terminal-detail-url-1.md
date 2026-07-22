# pbi-025-terminal-detail-url-1

**Status:** [DONE]

**Outcome:** Terminal detail now has a dedicated, refreshable `/terminal/<pane_id>` URL: `pushState` on forward navigation, `history.back()` for in-app Back with a `popstate` listener keeping browser/phone Back in sync (D1/D2); `bootstrap()` parses the URL and resolves `pane_id` against `agents[]`/`shells[]`, falling back silently to switcher on no match (D3); switcher/login stay at `/` (D4); a login triggered by a stale-session terminal URL redirects back into that terminal on success via a closure variable, `LoginProps` signature unchanged (D5).

**Files touched:** `web/src/main.ts`, `web/test/main.test.ts` (new)

**Verify:** `cd web && npm run typecheck && npm run test -- --run` — passed (66/66 tests, 15 new).

**Commit:** `bf8e7d4`

Full trace and evidence: `.bee/cells/pbi-025-terminal-detail-url-1.json`

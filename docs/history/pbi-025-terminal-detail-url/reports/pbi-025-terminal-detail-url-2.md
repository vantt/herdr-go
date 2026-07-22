# pbi-025-terminal-detail-url-2

**Status:** [DONE]

**Outcome:** Fixed the P1 finding from review session `review-pbi-025-terminal-detail-url-20260722` — `parseTerminalPaneId` (`web/src/main.ts`) now catches a `decodeURIComponent` failure and returns `null`, matching the existing D3 silent-switcher fallback, instead of letting an uncaught `URIError` blank-screen `bootstrap()`. Added a regression test in `web/test/main.test.ts`'s `parseTerminalPaneId` describe block.

**Files touched:** `web/src/main.ts`, `web/test/main.test.ts`

**Commit:** `1d72c1c`

Full trace and verification evidence: `.bee/cells/pbi-025-terminal-detail-url-2.json`

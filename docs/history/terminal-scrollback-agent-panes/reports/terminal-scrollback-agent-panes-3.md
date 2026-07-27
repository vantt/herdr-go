# terminal-scrollback-agent-panes-3

[DONE]

Added a "load older" trigger to the terminal-detail screen: scrolling
`.term-viewport` to its top fires exactly one `?history=1` request and does a
full wholesale re-render via the existing `applyScreen()` path (not a
prepend). Raised the render row clamp from 400 to 1000 to match herdr's own
scrollback cap, so history beyond 400 rows is actually reachable instead of
being silently discarded. Added explicit scroll-position preservation
(distance-from-bottom, clamped) covering both the row/line-cap ceiling
crossing and a content-shrink poll tick (the "load older" read is a one-shot
expansion, not a persistent poll mode). Added an in-flight guard so the
1500ms poll loop and a history-load request can never interleave. Added a
`fetchScreen(paneId, history)` param to `web/src/api.ts` for the new query
param.

Added real DOM-level test coverage for `renderTerminal` (6 new cases,
mounting the actual component against a mocked `fetch` and a real `#term-viewport`
node) plus 4 unit tests for the extracted `preserveScrollTop` pure helper --
the pre-existing suite only covered `computeKeyboardInset`/`stripAnsiLen`/`terminalHead`.
20/20 tests passing (up from 10); full web suite 98/98 passing; `tsc --noEmit`
clean.

Files touched: `web/src/views/terminal.ts`, `web/src/api.ts`, `web/test/terminal.test.ts`.

Commit: `7372efc`.

Full trace/evidence: `.bee/cells/terminal-scrollback-agent-panes-3.json`.

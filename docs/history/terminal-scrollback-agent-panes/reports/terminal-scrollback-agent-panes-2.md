# terminal-scrollback-agent-panes-2

[DONE]

Wired an optional `?history=1` query param onto `GET /api/panes/:pane/screen`:
absent keeps the pre-existing `Recent(80)` response unchanged; present routes
through `PaneScroller::read_history` and maps its returned `ScreenRead` into
the same `ScreenBody{text, revision}` shape. No new endpoint, no new response
type, no spec files touched (D11's spec sync is a separate bee-scribing step).

Files touched: `src/web/screen.rs`.

Commit: `dd7a703`.

Full trace/evidence: `.bee/cells/terminal-scrollback-agent-panes-2.json`.

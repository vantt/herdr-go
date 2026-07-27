# terminal-scrollback-agent-panes-1

**Status:** DONE

**Outcome:** `Herdr::read_pane` now takes explicit `source: ReadSource`/`lines: usize`
arguments (`SocketHerdr` sends the real `source` value instead of hardcoding
`"recent"`, capped at 1000 lines for `Recent`); added `Herdr::send_text`
(raw `pane.send_text` passthrough) implemented on `SocketHerdr`/`FakeHerdr`;
extended `FakeHerdr` with a history-aware per-pane shape (`seed_scroll_pane`,
`sent_text_log`); added the `PaneScroller` port (`src/herdr/pane_scroller.rs`)
implementing the visible-vs-recent(1000) compare-and-escalate mechanism with
D10's WHY-comment; updated the one production call site
(`src/web/screen.rs`) and the `RecordingHerdr` test stub
(`src/web/create.rs`) to compile against the new signature.

**Files touched:** `src/herdr/mod.rs`, `src/herdr/socket.rs`,
`src/herdr/fake.rs`, `src/herdr/pane_scroller.rs` (new), `src/web/create.rs`,
`src/web/screen.rs`.

Full trace and verification evidence: `.bee/cells/terminal-scrollback-agent-panes-1.json`.

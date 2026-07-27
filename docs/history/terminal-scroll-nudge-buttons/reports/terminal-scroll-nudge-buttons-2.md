# terminal-scroll-nudge-buttons-2

[DONE] — added floating up/down scroll-nudge buttons to the terminal-detail view for every session kind: the up button calls `loadOlder()` directly, the down button jumps `scrollTop = scrollHeight` (reusing cell-1's existing scroll-listener resume path, no duplicate logic); buttons idle-fade after inactivity and hide outright while the Reply or Keys sheet is open; positioned `position: absolute` against a newly `position: relative` `.view-terminal`, never `position: fixed`, no agent-kind branching. Added 4 new tests (button presence across agent kinds, up-tap history fetch, down-tap return-to-live, sheet-collision hiding).

Files touched: `web/src/views/terminal.ts`, `web/src/styles.css`, `web/test/terminal.test.ts`

Full trace and verification evidence: `.bee/cells/terminal-scroll-nudge-buttons-2.json`

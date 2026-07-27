# terminal-scroll-nudge-buttons-1

[DONE] — added a `viewingHistory` pause state gating `poll()`, entered via the existing scroll-to-top `loadOlder()` trigger and cleared by a symmetric bottom-threshold check in the same scroll listener (which also resumes `poll()` immediately, D7); rewrote the stale revert-on-next-tick test to assert the pause, and added a new test for the D6 sheet-open resume path.

Files touched: `web/src/views/terminal.ts`, `web/test/terminal.test.ts`

Full trace and verification evidence: `.bee/cells/terminal-scroll-nudge-buttons-1.json`

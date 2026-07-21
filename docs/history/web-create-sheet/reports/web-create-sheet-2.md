# web-create-sheet-2 — report

**Status:** [DONE]

**Outcome:** Added the canonical `NewPaneRef` type in `main.ts`, widened `Route`'s
terminal variant and `TerminalProps.agent` to `AgentRow | NewPaneRef`, and
extracted a pure exported `terminalHead()` deriving `{kind, display}` from either
shape (unit-tested directly). The existing AgentRow switcher→terminal path is
byte-for-byte unchanged.

**Files touched:**
- `web/src/main.ts`
- `web/src/views/terminal.ts`
- `web/test/terminal.test.ts`

**Verify:** `cd web && npm run typecheck && npm run test -- --run test/terminal.test.ts`
— tsc clean, vitest 5 passed (5).

Full trace and verification evidence: `.bee/cells/web-create-sheet-2.json`

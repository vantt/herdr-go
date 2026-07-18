# agent-card-legibility-1 — Report

**Status:** [DONE]

**Outcome:** `renderAgentCard` now shows `row.title` (kind-fallback, 2-line clamp)
and a merged kind/tab caption, with `.status-badge` untouched and a new
`kindAccentColor`-driven, `aria-hidden` watermark. Verify green (typecheck +
vitest, 20/20 tests).

**Files touched:**
- `web/src/views/switcher.ts`
- `web/src/styles.css`
- `web/test/switcher.test.ts`

**Commit:** `81315d8` — `feat(agent-card-legibility-1): redesign renderAgentCard title/caption + kind watermark`

Full trace/evidence: `.bee/cells/agent-card-legibility-1.json`

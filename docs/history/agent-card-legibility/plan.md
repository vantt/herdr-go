---
artifact_contract: bee-plan/v1
artifact_readiness: implementation-ready
mode: tiny
---

# Plan: agent-card-legibility

Mode: `tiny` — 1 risk flag: existing covered behavior (`switcher.ts` has test
coverage from `groupByWorkspace`, though `renderAgentCard` itself is untested
today). 2 files (`web/src/views/switcher.ts`, `web/src/styles.css`), no
data/API change, one direct task.

## Requirements (from CONTEXT.md)

- D1: `.agent-path` uses `row.title` (kind-fallback) instead of `row.display`,
  wraps up to 2 lines (`line-clamp: 2`) instead of single-line ellipsis.
- D2: `.agent-kind`/`.agent-tab` merge into one caption line ("kind · tab" or
  just "kind" when `tab_label` is empty).
- D3: `.status-badge` unchanged.
- D4: new low-opacity `kind`-monogram watermark, color hashed deterministically
  from `kind`, `aria-hidden`.

## Discovery

L0 — skip. Current code fully read this session (CONTEXT.md's Existing Code
Context cites exact line numbers, independently verified by the fresh-eyes
reviewer during exploring). No further verification needed before shaping the
cell.

## Approach

Recommended path: one cell, `renderAgentCard` rewritten per D1-D3 (structural
changes to existing markup/CSS), plus a new pure exported `kindAccentColor`-style
helper for D4's hash-to-hue, tested the same way `groupByWorkspace` was
(established precedent — see
`docs/history/learnings/20260718-terminal-workspace-org-population-sites.md`).

Rejected alternative: a lookup table mapping known kinds ("claude", "codex") to
fixed colors — rejected because D4 explicitly requires it to work for any future
kind with no code change (a hash function satisfies this for free; a lookup
table would need updating every time herdr adds an agent kind).

Risk map:

| Component | Risk | Reason | Proof needed |
|---|---|---|---|
| `renderAgentCard` markup change (D1/D2) | LOW | Same function, restructuring existing fields already available on `AgentRow` | `npm run typecheck` + manual render check |
| Hash-to-hue watermark (D4) | LOW | New pure function, no DOM/state dependency | unit test on the function itself (boundary: same kind → same color across calls) |
| CSS line-clamp (D1) | LOW | Standard, widely-supported CSS property in this app's target browsers (mobile Safari/Chrome) | visual check |

## Test matrix (tiny — the 2-3 dimensions that bite)

- Input extremes: `title` empty (falls back to `kind`, same as today's
  `display_for` logic) must not render "undefined" or a blank line.
- Scale: a `kind` value never seen before (not "claude"/"codex") must still
  produce a valid, stable color from the hash function, not crash or produce an
  invalid CSS color.
- Business logic boundary: `tab_label` empty vs non-empty — caption shows just
  "kind" (no dangling separator) vs "kind · tab", exactly matching D2.

## Out of scope

- Any change to `groupByWorkspace`/workspace section headers — untouched.
- `docs/specs/switcher.md` sync — separate scribing step after this cell caps
  (Data Dictionary rows 1-3 need updating: title no longer includes kind prefix,
  kind+tab become one element, new watermark element added).

## Current slice

- Entry state: `renderAgentCard` renders `row.display` (kind+title combined,
  1-line ellipsis), `.agent-kind` and `.agent-tab` as separate caption lines, no
  watermark.
- Exit state: `renderAgentCard` renders `row.title` (kind-fallback, up to 2
  lines), one merged "kind · tab" caption, unchanged status badge, and a new
  low-opacity kind-monogram watermark behind the card content.
- Files bounded: `web/src/views/switcher.ts`, `web/src/styles.css`,
  `web/test/switcher.test.ts` (new test for the hash-to-hue helper).
- Verify: `cd web && npm run typecheck && npm run test -- --run`

## Cells

- `agent-card-legibility-1` — redesign renderAgentCard per D1-D4

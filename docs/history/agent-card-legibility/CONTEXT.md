# Agent Card Legibility — Context

**Feature slug:** agent-card-legibility
**Date:** 2026-07-18
**Exploring session:** complete
**Scope:** Quick
**Domain types:** SEE

## Feature Boundary

Redesign the agent card markup/CSS in `web/src/views/switcher.ts` and
`web/src/styles.css` only — how a single agent row displays, not the workspace
grouping around it (already built, feature `terminal-workspace-org`). No data
model change: `kind`, `tab_label`, `status` already exist on `AgentRow`. No
backend touch.

## Locked Decisions

Fully settled through a design conversation with the user before this exploring
session started (quoted where useful) — not a Socratic-locking pass, formalized
here with D-IDs per the surface-scope-earlier rule.

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | The title line (`.agent-path`) switches from `row.display` (backend-formatted `"{kind} · {title}"`) to `row.title` alone (falling back to `row.kind` when `title` is empty — the same fallback `Snapshot::display_for` already applies server-side, just re-applied client-side against the raw `title` field `AgentRow` already carries), and wraps up to 2 lines (CSS `line-clamp: 2`) instead of today's single-line `white-space: nowrap` + ellipsis truncation. | User: title doesn't show enough to be readable/useful at a glance. Dropping the `"{kind} · "` prefix (kind is now covered by D2's caption and D4's watermark) gives the actual descriptive text more room, which is the real content the operator needs to read — fixed during fresh-eyes review, which caught that keeping `row.display` as-is would triple-show kind (title prefix + D2 caption + D4 watermark). |
| D2 | `.agent-kind` and `.agent-tab` merge into one caption line ("kind · tab"), cutting the card from 3-4 text rows to 2 (title + this caption). When `tab_label` is empty, show just the kind with no dangling separator (same omission rule the current `.agent-tab` conditional already applies, just relocated). This caption is now the one place `kind` appears as text (D1 no longer prefixes it). | Reclaims vertical space for D1's 2-line title without losing any information already shown. |
| D3 | The status badge (`.status-badge` — pill with dot + text label, e.g. "Working"/"Blocked") stays exactly as-is: no size, layout, or style change. | User explicitly said "status big" — an earlier-considered option (shrink to a dot-only indicator) was raised and rejected in the same conversation. |
| D4 | Each card gets a new low-opacity background watermark: a monogram (the `kind` field's first character, uppercase) in a color deterministically derived by hashing the `kind` string to a hue — stable per kind value across renders/sessions, not random, and requires no code change when herdr reports a kind this app has never seen before. | User's own idea, explicitly proposed as an alternative to manual pane renaming: gives a fast visual anchor (color + letter) without requiring the operator to read anything. |

### Agent's Discretion

Exact hex/HSL values for the hash-to-hue function, the monogram's font
size/opacity/position within the card, and the precise `line-clamp` CSS
properties are left to planning/implementation — none of these change what D1-D4
already fix, and the user explicitly said implementation-level specifics (exact
colors, sizing) don't need to come back for approval.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| watermark | The new low-opacity monogram background decoration on each agent card (D4) — purely decorative, `aria-hidden`, never competes with or obscures the card's real text content. |
| hash-to-hue | A pure function mapping a `kind` string to a stable color (e.g. a simple string hash mod 360 for an HSL hue) — same kind always produces the same color, no lookup table to maintain per kind. |

## Specific Ideas And References

- User's own framing of the problem (quoted, Vietnamese): "tôi không phân biệt
  được đó là pane gì, làm sao để người dùng human nhận thức một tên pane để tự
  map vào trí nhớ mình là nhiệm vụ pane đó đang làm gì" — the operator can't
  tell panes apart and wants to self-map a pane to what it's doing, from memory,
  without extra manual work.
- Two alternatives were raised and explicitly rejected before D1-D4 were reached:
  manual pane renaming (too much upkeep — "pane làm việc thay đổi liên tục nên
  đổi tên cũng là một gợi ý hay, nhưng tôi muốn brainstorm để làm sao ko cần
  phải làm việc đó" — pane work changes constantly, renaming isn't worth the
  effort), and real Anthropic/OpenAI brand logos for the kind watermark
  (trademark risk, no fallback for unknown kinds — user picked the neutral
  monogram instead, a direct answer via `AskUserQuestion`).

## Existing Code Context

Read directly this session (not delegated — small, already-familiar surface).

### Reusable Assets

- `web/src/views/switcher.ts:106-121` — `renderAgentCard`, the function this
  feature modifies. Currently renders 3 rows (`.agent-path`, `.agent-kind`,
  conditionally `.agent-tab`) plus the status badge.
- `web/src/styles.css:425-473` — `.agent-card`/`.agent-info`/`.agent-path`/
  `.agent-kind`/`.agent-tab` rules this feature edits. `.agent-path` currently
  has `overflow: hidden; text-overflow: ellipsis; white-space: nowrap;`
  (styles.css:458-460) — the exact rule D1 replaces.
- `web/src/styles.css:525` — `.status-badge` (D3: untouched).
- `web/test/switcher.test.ts` — existing test file (from `terminal-workspace-org`)
  covering `groupByWorkspace`; if this feature introduces a new pure function
  (e.g. the hash-to-hue helper), it follows the same test-pattern precedent
  already established there.

### Established Patterns

- Pure, exported helper functions in `web/src/views/*.ts` get unit tests in
  `web/test/<view>.test.ts` (established by `stripAnsiLen` in `terminal.ts` and
  `groupByWorkspace` in `switcher.ts` — see
  `docs/history/learnings/20260718-terminal-workspace-org-population-sites.md`).
  A hash-to-hue function is a natural candidate for the same treatment.

### Integration Points

- `web/src/views/switcher.ts` — `renderAgentCard` is the only render function
  touched; `renderWorkspaceSection`/`groupByWorkspace` (workspace grouping) are
  unaffected, agent cards render identically whether inside a workspace section
  or in the flat single-workspace list.

## Canonical References

- `docs/specs/switcher.md` — the current BA spec for this screen; its Data
  Dictionary (rows 1-4) and Behaviors sections need a sync pass once this
  feature ships (kind/tab merge into one caption element, title now multi-line).

## Outstanding Questions

### Resolve Before Planning

*(none — D1-D4 fully cover the scope, no gray areas remain)*

### Deferred To Planning

- [ ] Exact `line-clamp` CSS properties (`-webkit-line-clamp` vendor prefix
      needs, fallback for non-webkit) — implementer's choice within D1.
- [ ] Exact hash function and hue-to-color mapping for D4's watermark —
      implementer's choice, any stable deterministic function satisfies D4.

## Deferred Ideas

- Manual pane renaming via herdr's own rename socket method (already tracked as
  backlog PBI-008) — explicitly not this feature's solution, user wants to avoid
  the manual-upkeep cost entirely, but PBI-008 remains a legitimate separate idea
  for a different use case (e.g. long-lived panes that don't change task often).

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked
decisions, code context, canonical references, and deferred-to-planning questions.
Validating and reviewing use locked decisions for coverage and UAT.

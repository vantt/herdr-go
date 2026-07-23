# Group Header Chevron Status — Context

**Feature slug:** pbi-052-group-header-chevron-status
**Date:** 2026-07-23
**Exploring session:** complete
**Scope:** Quick
**Domain types:** SEE

## Feature Boundary

In the switcher's workspace group header (`renderWorkspaceSection`, `web/src/views/switcher.ts:269-293`), delete the `.status-badge` pill entirely and move its status color/wash/animation decor onto the existing `.workspace-chevron` collapse/expand icon, so the icon itself becomes the group's status indicator — nothing else in the switcher (agent-row badges, shell rows, rotate mechanic) changes.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Remove `.status-badge` entirely from the workspace group header — `renderGroupBadge`'s markup and its call site in `renderWorkspaceSection` are deleted; no pill/text remains in the DOM. | User's original backlog request text (docs/backlog.md PBI-052) — no gray area. |
| D2 | `.workspace-chevron` becomes the status indicator: icon color = `--status-{working\|blocked\|done\|idle\|unknown}`, reusing the exact `@keyframes pulse` (working) / `blink` (blocked) rules `.status-badge`/`.status-dot` already define, plus a circular background behind the icon using the matching `--status-*-wash` token — same token set the badge used. **The pulse/blink animation and the wash background must ride a non-rotating wrapper/background layer around the `<svg>` (e.g. a `::before` or wrapping element), never applied directly to the `<svg>` itself** — `pulse` animates `transform: scale(...)`, which would silently override D3's `rotate()` transform if both landed on the same element. | User's original request text; reuse confirmed against `web/src/styles.css:572-657,821-831`. Wrapper-layer constraint added after fresh-eyes review caught the D2/D3 transform collision (pulse's `transform: scale()` vs D3's `transform: rotate()` on the same element). |
| D3 | The existing rotate mechanic (`rotate(90deg)` collapsed ↔ `rotate(0deg)` expanded, driven by `[aria-expanded]`) is unchanged — the new status layer overlays on top, rotate behavior is untouched. | User stated this explicitly in the backlog row's open question (b); restated here as a locked constraint, not actually open. |
| D4 | Add accessible text (`aria-label` or a visually-hidden span) on the `.workspace-header` button carrying the status label (`STATUS_LABEL[status]`, `web/src/views/switcher.ts:35-41`), so removing the visible text pill doesn't silently drop status information for screen-reader users. | Auto-locked under `gate_bypass_level=full` (approval-class question, confident best-practice default — decisions 0010/dcf01d7b/a93994d3). |
| D5 | A group whose real `workspace_status` is `"unknown"` (a genuine rollup value herdr returned — distinct from hsw-D7's zero-agent absent case) gets the SAME color+wash chevron treatment as any other status, using `--status-unknown`/`--status-unknown-wash`, no animation — matching `.status-badge.status-unknown`'s current behavior exactly. | User-answered gray area (recommended option chosen): keep color+wash for unknown, no animation. |

### Agent's Discretion

Exact CSS mechanics (background circle diameter/padding, selector naming, exact wrapper markup shape) are left to planning/implementation — the only hard constraint is D2's wrapper-layer requirement (pulse/blink + wash must not land directly on the rotating `<svg>`) plus D1-D5 above.

## Existing Code Context

### Reusable Assets

- `web/src/styles.css:587-657` — `.status-badge`, `.status-dot`, per-status color/wash rules (`.status-working`, `.status-blocked`, `.status-done`, `.status-idle`), and `@keyframes pulse`/`blink` — the exact decor D2 reuses.
- `web/src/styles.css:821-831` — `--status-unknown`/`--status-unknown-wash` tokens and `.status-badge.status-unknown` rule (no animation) — what D5 reuses.
- `web/src/styles.css:572-581` — current `.workspace-chevron` rule (`color: var(--text-muted)`, `transform: rotate(90deg)`, transition) and its `[aria-expanded="false"]` override — the element D2/D3 build on.

### Established Patterns

- `renderGroupBadge` (`web/src/views/switcher.ts:260-267`) — current badge markup and its hsw-D7 empty-string short-circuit (`status === null || no agent rows`) — this function and its call site (`switcher.ts:286`) are deleted per D1. hsw-D7's absent-when-no-agents behavior is preserved (a zero-agent group's chevron stays plain, no status decor at all — never rendered as "unknown").
- `AgentStatus` type (`web/src/api.ts:5`) — `"working" | "blocked" | "done" | "idle" | "unknown"` — the five values D2/D5 must handle.

### Integration Points

- `web/src/views/switcher.ts:269-293` (`renderWorkspaceSection`) — where the badge call is removed and the chevron gains status classes.
- `web/src/styles.css:587-657,821-831` — where the reused decor rules are retargeted/added for `.workspace-chevron`.

## Canonical References

- `docs/specs/switcher.md` R14 (`docs/specs/switcher.md:221-223`) and hsw-D7 — current spec text describing the group header status badge; needs rewriting after implementation to describe chevron-as-indicator instead of a separate badge, keeping hsw-D7's absent-when-no-agents rule intact. This is a `bee-scribing` task after execution, not part of this CONTEXT.md.
- `docs/backlog.md` PBI-052 row — original request text, source of D1-D3.

## Deferred Ideas

- None raised during this session — the request was already narrowly scoped by the user (explicitly excludes individual agent-row badges at `switcher.ts:221-222`).

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads D1-D5, the reusable-assets list, and the canonical references above. Out of scope: agent-row badges, shell rows, and the rotate mechanic itself (D3 keeps it as-is).

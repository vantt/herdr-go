# Shell Row Text Alignment Fix — Context

**Feature slug:** pbi-049-shell-row-align
**Date:** 2026-07-22
**Exploring session:** complete
**Scope:** Quick
**Domain types:** SEE

## Feature Boundary

Fix a visual regression on the switcher's shell card: since PBI-046 added a leading `.shell-icon`, the card's text block (`.agent-info`) is pushed to the far-right edge of the card instead of sitting left-aligned right after the icon; this feature restores the pre-PBI-046 left-aligned text position for shell cards only, and ends there — no other switcher/home layout changes.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | The fix is scoped to `.shell-row` only. The shared `.agent-card` rule (`web/src/styles.css:454-468`, `justify-content: space-between`) is not modified. | Agent cards have 2 flow children (`.agent-info` + `.status-badge`) and correctly rely on `space-between` to push the badge right; changing the shared rule would also move the agent card's status badge. |
| D2 | Visual target: on a shell card, `.shell-icon` stays at the leftmost position (unchanged, already correct per PBI-046), and `.agent-info` (path + caption) sits immediately to its right, left-aligned within the remaining space — matching the pre-PBI-046 text position, just with the icon now in front of it. | User confirmed this is a "restore left-align like before" request, not a new layout. |
| D3 | No other PBI-046 shell-row properties change: the icon itself (glyph, size, color) and the solid-black `.shell-row` background (`web/src/styles.css:519-521`) stay exactly as they are. | Scope discipline — this feature fixes only the text-alignment regression, nothing else PBI-046 delivered. |

### Agent's Discretion

The exact CSS technique (e.g. `margin-right: auto` on `.shell-row .agent-info`, vs. overriding `justify-content` on `.shell-row`, vs. another equivalent scoped rule) is left to planning/implementation, as long as D1-D3 hold and no other card layout is affected.

## Existing Code Context

### Reusable Assets

- `web/src/styles.css:454-468` — the shared `.agent-card` rule (`display:flex; align-items:center; justify-content:space-between; text-align:left`) that both agent cards and shell rows inherit.
- `web/src/styles.css:516-521` — the existing `.shell-row` override block (solid black background only, from PBI-046 D4) — the established pattern for scoping shell-only CSS without touching `.agent-card`.

### Established Patterns

- Scoped override via a `.shell-row` rule layered after the shared `.agent-card` rule (same file, same technique already used for the background-color override) — the alignment fix should follow this same pattern rather than editing `.agent-card` itself.

### Integration Points

- `web/src/views/switcher.ts:234-251` (`renderShellRow`) — emits the `.agent-card.shell-row` button with exactly two flow children: `.shell-icon` and `.agent-info`. No markup change is expected; this is a CSS-only fix.

## Canonical References

- `docs/backlog.md` — PBI-049 row (flipped to `in-flight` this session, references this slug).
- `docs/history/pbi-046-shell-card-group/CONTEXT.md` — the feature that introduced `.shell-icon` and `.shell-row`, and established the "scope shell-only CSS via `.shell-row`, don't touch `.agent-card`" precedent this fix continues.

## Outstanding Questions

None blocking planning.

## Deferred Ideas

None — this is a scoped fix of an existing regression, not a source of new backlog ideas.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs (D1-D3) are stable and cited, never reinterpreted. Planning picks the concrete CSS technique under "Agent's Discretion" and produces the tiny-lane cell.

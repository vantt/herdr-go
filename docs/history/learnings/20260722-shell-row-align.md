---
date: 2026-07-22
feature: pbi-049-shell-row-align
categories: [css, layout]
severity: minor
tags: [flex-layout, css-reuse, switcher]
---

# Shell row icon and text split apart instead of sitting together

## What Happened

`.shell-row` (web/src/styles.css) inherits `.agent-card`'s `display: flex;
justify-content: space-between`. `.agent-card` was designed for its own 2
in-flow children (`.agent-info` + `.status-badge` — `.agent-watermark` is
`position: absolute`, out of flow), so `space-between` correctly puts info on
the left and the badge on the right. `renderShellRow` (web/src/views/switcher.ts)
emits a different pair of in-flow children — `.shell-icon` + `.agent-info`, no
status badge at all, by design (R13). With only 2 items and no third to
balance against on the right, `space-between` pushed the icon to the far-left
edge and the info block flush to the far-right edge, instead of grouping them
together on the left with the existing gap.

## Root Cause

A shared flex-layout rule was reused by a variant markup shape with a
different number of in-flow children than the rule was designed to lay out.
`justify-content: space-between`'s behavior depends entirely on how many flex
items are actually present — nothing in the rule itself, or in the class name
`.shell-row`, signaled that its child count differed from `.agent-card`'s.

## Recommendation

When a CSS variant class layers on top of a shared flex-container rule (the
established pattern here: override on the variant class, never edit the base
rule — see the 20260722-shell-label-merge-execution-dispatch.md learning),
check whether the variant's rendered markup has the **same number of in-flow
children** the base rule's `justify-content`/`align-items` was tuned for. A
child-count mismatch silently changes the distribution (edges spread apart
instead of grouped) with no error and no failing type check — it only shows
up visually. If the count differs, the variant needs its own
`justify-content` override, not just an additive rule for what's visually new
(background, icon, etc).

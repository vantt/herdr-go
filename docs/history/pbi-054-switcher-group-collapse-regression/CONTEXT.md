# Switcher group-header collapse/expand fix — Context

**Feature slug:** pbi-054-switcher-group-collapse-regression
**Date:** 2026-07-23
**Exploring session:** complete
**Scope:** Quick
**Domain types:** SEE

## Feature Boundary

Fix `.workspace-rows` (Trang Home switcher, `web/src/views/switcher.ts:298`) so clicking a
`.workspace-header` toggle actually hides/shows the group's card list — it currently never
visually collapses regardless of click state. Nothing else on the switcher screen changes.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Confirmed root cause: `.workspace-rows` carries `class="agent-list workspace-rows"` (`switcher.ts:298`), and `.agent-list { display: flex; ... }` (`styles.css:445-452`) is an author-origin, normal-priority CSS declaration. Per the CSS cascade, author-normal always outranks user-agent-normal (`[hidden] { display: none }`) regardless of selector specificity — so the `hidden` attribute the click handler sets (`switcher.ts:349`) never actually hides the list. The chevron's own rotate animation is unaffected and correct (`styles.css:572-581`, untouched history). | This is the exact bug class the codebase already fixed 4 times elsewhere in the same file — `.reply-sheet[hidden]`, `.keys-pad[hidden]`, `.create-sheet[hidden]`, `.dropdown-popup[hidden]` (`styles.css:950,1034,1078,1195`), each with a comment naming this precise mechanism. `.workspace-rows[hidden]` is the one place that override was never added. |
| D2 | **Not a PBI-052 regression.** The collapsible-grouping feature itself (introduced in commit `4bb0a36`, "render workspace/tab badge + collapsible grouping") never actually collapsed visually, since `.agent-list`'s `display: flex` rule predates it (present since the project's first commit `89be198`). PBI-052 (`9839096`, chevron status decor) did not touch the click handler, the rotate CSS, or the `.workspace-rows`/`.agent-list` class list — confirmed by diff audit. The user is only now noticing it because PBI-052 drew fresh attention to the group header. `docs/backlog.md`'s PBI-054 row is corrected to reflect this (see capture note below); the original "suspected PBI-052 regression" framing stands corrected, not removed. | User confirmed symptom as "icon xoay đúng, nhưng card không collapse/expand" — matches D1 exactly (rotate works, list-hide does not), ruling out the originally-suspected pulse/blink-animation-interference theory. |
| D3 | Fix scope: add one CSS rule, `.workspace-rows[hidden] { display: none; }`, matching the 4 existing precedents' pattern exactly. No JS changes — the click handler, `aria-expanded` toggling, and chevron rotate are already correct. | Smallest fix that matches established in-repo precedent; nothing else is broken. |

## Existing Code Context

### Reusable Assets

- `web/src/styles.css:950,1034,1078,1195` — 4 existing `[hidden]` override rules for the identical author-vs-UA cascade conflict; D3's fix copies this exact pattern for `.workspace-rows`.

### Established Patterns

- Author-origin `display` rules always beat the UA `[hidden]{display:none}` default in this codebase's CSS — any element that both uses `hidden` for show/hide AND has its own `display` override needs an explicit `<selector>[hidden] { display: none; }` rule. `.workspace-rows` is the one place this was missed.

### Integration Points

- `web/src/views/switcher.ts:337-352` — click handler already sets `hidden`/`aria-expanded` correctly; no change needed here, only cited so planning doesn't re-touch working code.

## Canonical References

- `git show 9839096` — PBI-052 diff, audited in full during this exploring session; confirms zero overlap with the collapse/expand mechanism.
- `git log --oneline -G'\.agent-list \{' -- web/src/styles.css` → `89be198` — proves `.agent-list { display:flex }` predates the collapsible-grouping feature (`4bb0a36`).

## Outstanding Questions

None — root cause is proven by CSS cascade-origin rules (deterministic, not browser-dependent) plus git history, not by assumption.

## Deferred Ideas

- Automated regression coverage for this class of bug (an element using `hidden` while also carrying its own `display` override) is not attempted here — jsdom does not load the project's external stylesheet by default, so this bug is provable by code/history reading but not currently assertable by a unit test. Leaving as manual/visual confirmation, consistent with this repo's standing WebKit/CSS-verification-gap pattern (see `docs/history/learnings/critical-patterns.md`). — deferred, not blocking this fix.

## Handoff Note

CONTEXT.md is the source of truth. D1-D3 fully determine the fix: one CSS rule, no JS changes,
one file touched (`web/src/styles.css`). Planning should size this as `tiny`.

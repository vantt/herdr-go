# Create-Sheet Overlay UX Redesign — Context

**Feature slug:** pbi-053-create-sheet-overlay-ux
**Date:** 2026-07-23
**Exploring session:** complete
**Scope:** Standard
**Domain types:** SEE

## Feature Boundary

Redesign the "Create Shell/Agent" overlay sheet (`renderCreateSheet`, `web/src/views/create-sheet.ts`) so its two always-expanded, scrolling card lists (destination cards, shell/preset action cards) become two collapsed-by-default dropdown/combobox fields plus an explicit "New" confirm button — without losing any information currently shown per card, and without changing the create/error behavior already locked by `docs/specs/create-sheet.md` R1-R5.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Two fields — Destination and Type — are independent dropdown/combobox selections; neither auto-triggers creation. | User explicitly chose this over "select Type = create immediately" (which would have matched today's `renderActions` click-to-create behavior) — trades 1 extra tap for an explicit, unambiguous confirm step. |
| D2 | A single "New" button, separate from both dropdowns, is the only action that creates the Shell/Agent. | Direct consequence of D1. |
| D3 | Both dropdowns pre-select a default the moment the sheet opens (first destination; "Shell" as default Type) so "New" is immediately actionable without forcing the user to touch either dropdown first — mirrors today's `selectedIndex = 0` default in `load()` (`create-sheet.ts:205`). | Minimal behavior change from today; avoids a dead-on-open state where New has nothing to submit. |
| D4 | Each dropdown's option list, when open, renders as a floating popup layered on top of the sheet (not an inline accordion that pushes sheet content down). | User's explicit choice, made with the known trade-off surfaced: this repo has prior WebKit/iOS Safari overlay bugs (`docs/history/pbi-027-visual-viewport-keyboard/CONTEXT.md` — `position:fixed` issues, first-ever `window.visualViewport` use). Planning/validating must budget explicit mobile-Safari verification for the popup's positioning/z-index, since there is no existing in-repo pattern to copy. |
| D5 | No native `<select popover>` / "customizable select" API. The dropdown is a hand-rolled component (button + ARIA listbox-style popup), reusing the existing card markup/styling for options. | Confirmed via scout: zero existing combobox/popover/`<details>`/listbox pattern anywhere in `web/src` (first-of-its-kind in this app) and the newer native customizable-`<select>` API has no reliable Safari support, which is this app's real target (mobile-first, `viewport-fit=cover`, documented WebKit quirks). Native `<select>` was already ruled out pre-exploring (options can't be decorated as cards). |
| D6 | The two dropdowns stack vertically (Destination above Type), not side-by-side. | Mobile-first narrow viewport; matches today's vertical list order. |
| D7 | Opening one dropdown closes the other if it was open (mutually exclusive). | Standard combobox behavior; avoids two floating popups stacked at once inside an already-scrolling 70vh sheet. |
| D8 | Every field/value currently shown per card is preserved exactly — destination label, disambiguating suffix (`collisionSuffixes`), path or "no folder yet" fallback, caveat (`destinationCaveat`: "Folder not detected" / "Folder may be stale"); preset/type label only (never the underlying command). Only the *presentation* changes (collapsed line vs always-expanded list); no information is cut. | Re-confirms the direction already chosen 2026-07-23 per `docs/backlog.md` PBI-053 row, before this exploring session opened. |
| D9 | `docs/specs/create-sheet.md` R1-R5 behavior is unchanged: unresolved-folder destinations stay selectable and caveat-marked (R1); Shell creation still succeeds in an unresolved destination while agent-start there is refused (R2); a create failure is shown inline and the sheet is never dismissed on error (R3); the label-only disambiguator rule (R4) and preset-command-hidden rule (R5) both carry over unchanged. | These requirements are layout-agnostic — they govern data/behavior, not the card-list presentation being replaced. |

### Agent's Discretion

- Exact collapsed-state summary text/truncation for each dropdown's closed line (e.g. how a long path or disambiguator suffix is shortened to fit one line) — implementer's call within D8's "no information cut when expanded" constraint.
- Exact popup positioning/anchoring mechanics for D4 (CSS anchor positioning vs manual JS measurement) — implementation detail, informed by whatever mobile-Safari verification planning/validating decide is needed.
- New button placement within the sheet (e.g. sticky footer vs inline after both dropdowns) — implementer's call, must remain reachable without requiring the sheet's own scroll if avoidable.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Destination | One `Destination` (`web/src/api.ts:43-48`): a workspace identified by `workspace_id` + `label` + `path` + `path_is_live`. Two destinations can share a path (worktree vs main checkout) but are never merged — dropdown must keep them as distinct entries (existing `collisionSuffixes` disambiguation). |
| Type | The create action: "Shell" (always present, fixed) or one of the dynamically-configured `agent_presets` (`PresetOption { label }`, `web/src/api.ts`) — list length varies (can be empty beyond "Shell", or more than 3); never hardcoded to a fixed enum. |
| New button | The single explicit confirm action (D2) that replaces today's "click an action row = create immediately" trigger. |

## Specific Ideas And References

- User's original rough suggestion (pre-exploring, recorded in `docs/backlog.md` PBI-053 row): "2 field dropdown/selectbox — path and type — with a 'New' button to activate." This exploring session confirmed the "New" button part explicitly (D1/D2) rather than assuming it from the backlog note alone.

## Existing Code Context

From the quick scout only. Downstream agents read these before planning.

### Reusable Assets

- `web/src/views/create-sheet.ts:18-22` (`destinationCaveat`), `:31-39` (`collisionSuffixes`), `:86-116` (`renderDestinations`), `:118-134` (`renderActions`) — the exact data-shaping and per-row markup to preserve inside each dropdown's option cards.
- `web/src/api.ts:43-56` — `Destination` and `PresetOption`/`CreateOptions` shapes, unchanged by this feature.
- `web/src/styles.css:1076-1124` — current sheet chrome (`max-height:70vh; overflow-y:auto`, `sheet-up` animation, shared chrome comment noting `.reply-sheet`/`.keys-pad` also use it) to extend, not replace.

### Established Patterns

- None found for combobox/popover/disclosure widgets anywhere in `web/src` (switcher.ts, terminal.ts included) — this is a first-of-its-kind component in the app (per D5).

### Integration Points

- `web/src/views/create-sheet.ts` — `renderCreateSheet`, `load()`, `open()`/`close()`, `selectedIndex` state all need rework for the two-dropdown + New-button model.
- `web/src/styles.css:1076-1124` — sheet and row styling to extend for collapsed/expanded dropdown states and the floating popup (D4).
- `docs/specs/create-sheet.md` — R1-R5 requirements to re-cite unchanged (D9); layout-describing sections to rewrite for the new structure.

## Canonical References

- `docs/specs/create-sheet.md` — current spec, R1-R5 behavior invariants (D9).
- `docs/history/pbi-027-visual-viewport-keyboard/CONTEXT.md` — prior WebKit/iOS Safari overlay/positioning issues in this app, relevant risk context for D4.
- `docs/backlog.md` PBI-053 row — original request, 2026-07-23 pre-exploring UX direction (no native `<select>`, cards preserved, both fields become combobox).

## Outstanding Questions

### Resolve Before Planning

(none — all product-level gray areas locked above)

### Deferred To Planning

- [ ] Exact ARIA pattern/keyboard-nav spec for the hand-rolled combobox (listbox role, `aria-expanded`, focus management) — planning should confirm the accessibility baseline this component must meet.
- [ ] Mobile-Safari verification approach for the D4 floating popup (manual device test vs an automatable check) — given this repo currently has no way to prove WebKit overlay behavior beyond manual testing (see PBI-027's own open gap).

## Deferred Ideas

- PBI-048 (project identity independent of `workspace_id`) — unrelated pre-existing backlog item, not touched by this feature; destinations sharing a path stay distinct per D-Terms above, consistent with current disambiguation.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs (D1-D9) are stable. Planning reads locked decisions, existing code context, canonical references, and the deferred-to-planning questions above. Validating and reviewing use these locked decisions for coverage and UAT.

---
artifact_contract: bee-plan/v1
mode: standard
approved_gate2: 2026-07-23T05:33:00Z
---

# Create-Sheet Overlay UX Redesign — Plan

**Feature slug:** pbi-053-create-sheet-overlay-ux
**CONTEXT.md:** `docs/history/pbi-053-create-sheet-overlay-ux/CONTEXT.md` (D1-D9)

## Mode Gate Record

Flags counted: 2 —
1. *Changes behavior an existing test asserts*: `web/test/create-sheet.test.ts` currently asserts that clicking an action row (`.action-row`) immediately calls `createPane`/`createAgent` (tests at lines 115-148, 150-172, 174-197). D1/D2 replace this with an explicit "New" button trigger — a covered contract changes, not a bugfix that keeps it green.
2. *Requires weakening/deleting/replacing existing proof*: the same tests must be rewritten (not just supplemented) to drive the new two-dropdown + New-button flow.

No other flag applies (no auth/authz/data-model/audit/external-system/public-contract/cross-platform/multi-domain surface — this is a single internal frontend view; `Destination`/`PresetOption`/API endpoints are all unchanged per D8/D9).

2 flags → **standard** lane. Product files: `web/src/views/create-sheet.ts`, `web/src/styles.css`, `web/test/create-sheet.test.ts` — 3 files, all tightly coupled (the test asserts against markup the same change introduces), so they stay in one cell rather than split (see Learning below).

## Discovery

**L1 — quick verify**, evidence-based, no separate `discovery.md` (restates plan, not a multi-candidate comparison):

- No existing combobox/popover/anchor-positioning JS utility exists anywhere in `web/src` (confirmed in exploring, D5) — `getBoundingClientRect` does not appear anywhere in `web/src/*.ts` (grep, this session). Nothing to reuse; a small amount of new positioning code is unavoidable.
- This repo has an explicit, documented rule against `position:fixed` for mobile overlays: `web/src/styles.css:739-743` — a comment on `.view-terminal` states fixed positioning fought `-webkit-overflow-scrolling` and rendered "zoomed/rigid" on mobile. D4 already accepted the popup-over-inline trade-off with this risk surfaced; the concrete way to honor D4 *without* re-triggering this exact bug class is: anchor each popup with `position: absolute` against a `position: relative` wrapper around its own trigger row (a local stacking context scrolled along with the sheet), never `position: fixed` against the viewport. This needs zero JS-measured coordinates — CSS anchoring only — which sidesteps the specific mobile-Safari failure mode on record. This is the recommended approach below, not a locked decision (implementation detail, per CONTEXT.md's Agent's Discretion).

## Approach

**Chosen path:** extend `renderCreateSheet` (`web/src/views/create-sheet.ts`) in place — no new file/module. The destination list and action list keep their existing per-row markup/data (label, suffix, path, caveat — D8) and rendering functions (`collisionSuffixes`, `destinationCaveat`, `renderDestinations`, `renderActions` bodies stay materially the same), but each list is now wrapped in a collapsible dropdown:

- A trigger `<button>` per field (Destination, Type) showing the current selection as one line (`aria-haspopup="listbox"`, `aria-expanded`).
- The existing `<ul>` (`destination-list`/`action-list`) becomes the popup's listbox, `position: absolute` under its own `position: relative` trigger wrapper (see Discovery), hidden unless that field's dropdown is open.
- Two new pieces of state: `openDropdown: "destination" | "type" | null` (D7 — opening one closes the other) and `selectedPreset: string | null` (the Type selection; `null` means "Shell", matching today's `data-kind="shell"` semantics) — replacing the current click-triggers-immediately `handleAction` wiring.
- `renderActions`' click handler no longer calls `handleAction` directly (D1) — it sets `selectedPreset` and closes the Type dropdown instead.
- A new "New" button, disabled only while `submitting`, calls the existing `handleAction(selectedPreset)` body verbatim (D2) — `selectedIndex`/`selectedPreset` both default-populated on `load()` (D3: `selectedIndex = 0` already exists; add `selectedPreset = null` alongside it) so New is actionable immediately on open, matching today's low-friction default.
- `styles.css` gains the collapsed-trigger row style, the popup positioning rule (`position: relative` wrapper / `position: absolute` popup, scoped not to fight the existing `.create-sheet` `overflow-y:auto`), and the New button style — extending the existing sheet chrome block (`:1076-1124`) rather than a new section.

**Rejected alternative:** a separate reusable `Dropdown`/`Combobox` component module. Rejected per YAGNI/KISS — this sheet is the only place in the app that needs this pattern (D5: first-of-its-kind), extracting a generic component now would be speculative generalization for a single caller; revisit only if a second call site appears.

**Risk map:**

| Component | Risk | Proof needed |
|---|---|---|
| Popup positioning on mobile Safari | MEDIUM | No repo-native way to prove WebKit rendering short of manual device testing (CONTEXT.md Deferred-to-Planning item, mirrors PBI-027's own open gap). The `position:absolute`-in-`position:relative` approach avoids the *specific* failure mode on record (`position:fixed`), which lowers but does not zero out the risk. Cell's `must_haves` requires this pairing explicitly; manual verification is called out as a follow-up the user does after merge, same as PBI-027. |
| Rewriting `create-sheet.test.ts`'s 5 create-trigger-dependent tests (lines 115-197) to drive dropdown-open → select → New-click instead of one action click | LOW | Mechanical; existing assertions (payload shape, error handling, double-submit guard) are unchanged in substance, only the DOM interaction sequence that reaches them changes. |
| `styles.css` scope creep onto unrelated `.create-sheet` rules | LOW | Learning 2 precedent (`docs/history/learnings/20260721-web-create-sheet-type-ownership-and-css-scope.md`) — `styles.css` is explicitly in this cell's `files` list from the start. |

**Test matrix (12 edge dimensions, scaled to a standard-lane single-file UI change — only dimensions with real signal listed):**

- **Happy path:** open sheet → default Destination/Type pre-selected → New creates a Shell (mirrors current test at line 115-128, re-pointed at New button).
- **Boundary/empty:** zero presets (`presets: []`, already covered by existing collision test's fixture at line 82) — Type dropdown still offers exactly "Shell".
- **Concurrent/re-entrancy:** double/triple-click on New while `submitting` — must not fire two create requests (mirrors existing test at line 150-172, re-pointed).
- **Error path:** create-call failure shown inline, sheet stays open, New re-enabled (mirrors existing test at line 174-197).
- **State machine:** opening Type while Destination is open closes Destination first (D7) — new test, no prior equivalent.
- **Selection integrity:** destination caveat/suffix/path rendering inside the opened popup — same assertions as today's tests (lines 51-99), re-pointed at the popup's now-conditionally-rendered list.

## Current Slice — Cells

One cell. All three files are tightly coupled (test assertions target the exact markup/selectors the implementation change introduces), so splitting risks the cross-cell mismatch this repo has already hit once (Learning 1, `20260721-web-create-sheet-type-ownership-and-css-scope.md`) — keeping them in one cell removes that risk entirely rather than mitigating it.

Cell spec below is created via `bee.mjs cells add` in the Prep step, not written here (plan.md is frozen after Gate 2, D1 — cells live only in `.bee/cells/`).

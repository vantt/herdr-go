---
date: 2026-07-21
feature: web-create-sheet
categories: [decision, failure, pattern]
severity: critical
tags: [cell-scoping, plan-checking, cross-cell-types, css-scope, state-hygiene]
---

# Learning: web-create-sheet (Phase 1)

## Learning 1 — A cross-cell shared type needs one pinned owner before cells are cut, especially when it's added as a mid-plan fix

**Category:** decision / failure
**Severity:** critical
**Tags:** [cell-scoping, plan-checking, cross-cell-types]
**Applicable-when:** planning introduces (or a fresh-eyes/validation review adds) a type or contract that more than one cell must produce or consume — a DTO, an event shape, a shared interface.

### What Happened

`S5` (the minimal post-create navigation reference) was added to `web-create-sheet`'s CONTEXT.md as a fix during exploring's fresh-eyes review — a plain shell structurally can never produce a full `AgentRow`, so the parent feature's "navigate into the new pane" requirement was unimplementable as originally scoped. CONTEXT.md correctly deferred the exact type shape to planning. Planning cut cell 2 (define + widen routing) and cell 3 (build the sheet, construct the value) without pinning a single name/field-set for that shared type. Two independent reviewers — the adversarial plan-checker and the cold-pickup cell reviewer, working from different evidence — converged on the identical defect: cell 2 described `{pane_id, display, name?}`, cell 3 described `{pane_id, name?, workspace_id, label}`, and cell 1's create-call success payload was never required to expose the fields cell 3 needed at all. Because cell 3's `files` list couldn't touch `api.ts`/`main.ts`, a shape mismatch would have surfaced as a `tsc` failure the blocked cell was *prohibited from fixing*, forcing orchestrator intervention mid-wave.

### Root Cause

A type-shape decision was correctly deferred from exploring to planning, but planning cut cell boundaries before resolving it — leaving three cells free to each invent a compatible-looking but not identical shape.

### Recommendation

When CONTEXT.md defers a cross-cell type's exact shape to planning, resolve it *before* writing the cells: name the type, list its exact fields and optionality, assign exactly one cell as its sole owner/exporter, and make every consuming cell's `must_haves`/prohibitions state "import this type, never redefine its field set." Also require the producing cell (e.g., the one wrapping the API call whose response feeds the type) to have an explicit truth naming which response fields are load-bearing — a `must_haves.truths` that never states this is the same defect one hop upstream. This generalizes the earlier `web-create-endpoints` lesson ("cell boundaries follow coupling, not modules") into type-contract terms specifically.

---

## Learning 2 — Two consecutive UI-adding cells both needed `styles.css`; neither declared it in scope

**Category:** failure
**Severity:** critical
**Tags:** [cell-scoping, css-scope, planning-completeness]
**Applicable-when:** planning shapes any cell whose action adds new rendered markup (a component, an overlay, a button) in this repo's frontend.

### What Happened

Cell 3 (`create-sheet.ts`) and cell 4 (the FAB + switcher wiring) both needed new CSS — the sheet's overlay chrome and the FAB's fixed positioning/disabled state — but `web/src/styles.css` was in neither cell's declared `files` list. Cell 4's worker caught it at execution time, added `.fab` and `.create-sheet` rules as a transparently recorded, additive-only deviation, and cited real precedent: prior UI-adding cells in this repo (`terminal-reply-ui-1`, `terminal-nav-keys-2`) *did* include `styles.css` in their scope. The recovery worked — nothing shipped broken — but the gap recurred across two consecutive cells of the same feature, not once.

### Root Cause

Planning didn't cross-check "cells that add new DOM/view surface" against "cells that declare `styles.css`," even though this repo has a standing, checkable precedent for that pairing.

### Recommendation

When shaping a cell whose action adds new rendered markup, either include `web/src/styles.css` in its `files` list or state explicitly in the action that no new styling is needed. Before finalizing a plan with UI-adding cells, grep prior UI-adding cells' `files` lists for the `styles.css` pairing as a mechanical precedent check — this is exactly the kind of check that belongs in the plan-checker's scope-sanity dimension, not left to worker self-correction.

---

## Learning 3 — Stale `approved_gates`/`mode` from a prior feature, caught before any cell claimed against it (process note, already filed as friction)

**Category:** failure
**Severity:** standard
**Tags:** [state-hygiene, gate-safety]
**Applicable-when:** transitioning from a just-closed feature to a new one via `state set --feature` rather than `state start-feature`.

### What Happened

At validating's orient step, `.bee/state.json` still showed `approved_gates.execution: true` and `mode: high-risk`, both stale carry-overs from the just-closed `web-create-endpoints`. The exploring→planning transition used the generic `state set --owner ... --feature web-create-sheet` call (matching AGENTS.md's documented step 6), which updates `feature`/`phase` but not `approved_gates`/`mode` — only the dedicated `state start-feature` verb resets all four gates atomically. No cell had been claimed yet, so this was a genuine near-miss, not an incident.

### Root Cause

The documented feature-transition flow (a generic `state set`) and the gate-resetting flow (`state start-feature`) are two different verbs, and nothing in the documented flow signals when to use the atomic one.

### Recommendation

Already filed as P2 friction (`.bee/backlog.jsonl`, layer `state`) for `bee-grooming` — this is a bee-tooling gap, not a host-project pattern, so it is not duplicated into `critical-patterns.md`. Future sessions transitioning between features should treat any generic `state set --feature` call as leaving gate state suspect until this friction is resolved upstream, and verify `approved_gates`/`mode` freshly at validating's orient step regardless (which is exactly what caught this instance).

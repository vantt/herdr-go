---
date: 2026-07-23
feature: pbi-054-switcher-group-collapse-regression
categories: [pattern, decision, failure]
severity: critical
tags: [css-cascade, hidden-attribute, regression-triage, bee-tiny-lane, behavior-change-flag]
---

# Learning: Switcher group-collapse "PBI-052 regression" was actually a 5-day-old CSS cascade bug

**Category:** pattern, decision, failure
**Severity:** critical
**Tags:** [css-cascade, hidden-attribute, regression-triage, bee-tiny-lane]
**Applicable-when:** any element toggled via the `hidden` DOM attribute/property that also carries an author `display` rule; any bug report that blames the most-recently-touched commit for a break.

## What Happened

User reported the switcher's workspace-group header stopped collapsing/expanding smoothly, filed same-day as PBI-052 (chevron status-decor move) and assumed as a PBI-052 regression. Exploring's investigation (diff audit of `9839096` + `git log -G'\.agent-list \{' -- web/src/styles.css` → `89be198`) proved PBI-052 touched none of the relevant code, and the real defect predates it by 5 days: `.workspace-rows` carries class `agent-list workspace-rows`, and `.agent-list { display: flex }` (present since the project's first commit) is an author-origin CSS rule that always outranks the UA `[hidden] { display: none }` default regardless of specificity. The collapsible-grouping feature (`4bb0a36`) never actually collapsed visually since it was introduced — its own test suite covered grouping/sorting logic but never clicked `.workspace-header` or asserted on the resulting `display`. Fixed with one CSS rule, `.workspace-rows[hidden] { display: none; }`, matching an identical fix already applied 4 times elsewhere in the same stylesheet (`.reply-sheet`, `.keys-pad`, `.create-sheet`, `.dropdown-popup`).

## Root Cause

1. **CSS bug:** author-normal-origin declarations always beat user-agent-normal-origin declarations in the cascade, independent of selector specificity — so any element using `.hidden`-attribute toggling for show/hide, whose own class also sets `display`, silently never hides unless an explicit `<selector>[hidden] { display: none; }` override exists. This codebase has now hit this exact bug class 5 times.
2. **Triage bug:** the backlog report blamed the most-recently-touched commit (PBI-052) purely by recency, without a git-history check, before any investigation.
3. **Tooling bug (bee's own):** the cell capped with `trace.behavior_change: false` despite its `must_haves.truths` describing plainly user-visible behavior ("clicking... visually hides..."). `behavior_change` on a cell is opt-in via an explicit `--behavior-change` flag at `cells cap` time; nothing cross-checks it against the cell's own `must_haves.truths` wording. The `tiny` lane is also fully exempt from Decision 0004's evidence-proof gate (`hasOutput || hasEvidence`), so the cell capped on a bare `verify_passed: true` with no recorded verify output. Net effect: `scribingDebt()` never flagged this cell, and `docs/specs/switcher.md` was only synced because of a manual, self-initiated decision — not because any tooling forced it.

## Recommendation

- When any element in `web/src/styles.css` toggles visibility via the `hidden` attribute/property, add an explicit `<selector>[hidden] { display: none; }` rule with a comment naming the cascade-origin mechanism (copy the exact comment style already used at `.reply-sheet[hidden]`, `.keys-pad[hidden]`, `.create-sheet[hidden]`, `.dropdown-popup[hidden]`, `.workspace-rows[hidden]`) — do this at the same time the element gains its own `display` rule, not after a user reports it broken.
- Before writing a "regression from feature X" bug report, run the two-command git audit first: `git show <X's commit> -- <affected files>` (does X actually touch the broken code?) and `git log -G'<suspected-cause-pattern>' -- <file>` (when did the real cause land?). Only write the causal claim down after both checks, not before.
- When capping ANY cell (any lane, tiny included) whose `must_haves.truths` describes a user-observable behavior (visual state, interaction outcome, data returned to a user), pass `--behavior-change true` explicitly at `cells cap` time — do not rely on the tiny lane's evidence-gate exemption to make this optional in practice. A behavior-change cell that caps `false` is invisible to `scribingDebt()` and can ship with no BA-spec trace at all.

**Full entry:** this file

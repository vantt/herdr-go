---
date: 2026-07-21
feature: home-shell-workspaces
categories: [failure, pattern]
severity: critical
tags: [typescript, whole-project-typecheck, cell-scoping, verify-authoring, fake-fidelity]
---

# Learning: home-shell-workspaces

## Learning 1 — Splitting a cell across a changed export's signature and its sole consumer deadlocks under whole-project `tsc`

**Category:** failure
**Severity:** critical
**Tags:** [typescript, whole-project-typecheck, cell-scoping]
**Applicable-when:** planning a multi-cell slice in this repo's `web/` package (or any package whose `tsconfig.json` sets a whole-project `include`), where one cell changes an exported function's return/parameter type and a different cell owns that function's call site.

### What Happened

`home-shell-workspaces` was originally planned as 3 cells: backend, then a frontend data-layer cell (change `fetchAgents()`'s return shape), then a frontend rendering cell (update `switcher.ts`'s consumption). `web/tsconfig.json`'s `include: ["src", "test"]` means `npm run typecheck` (`tsc --noEmit`) checks the **entire** project on every run, not just the changed file. The data-layer cell's own verify could never pass in isolation: `switcher.ts:120-125`'s `renderList(rows: AgentRow[])` call site would type-error against the new shape, and that cell was prohibited from touching `switcher.ts`. The fix (the rendering cell) was gated behind the data-layer cell capping — a genuine deadlock, not just an inconvenient order. Two independent reviewers (the adversarial plan-checker and a separate cold-pickup cell reviewer) converged on the identical finding from different evidence.

### Root Cause

The cell boundary was drawn along a signature/consumer line under a typechecker whose scope is whole-project. That specific split is structurally unsound regardless of dependency ordering — neither half can pass its own verify alone.

### Recommendation

Before finalizing a multi-cell TypeScript slice in a whole-project-`tsc` package, check whether any cell changes an exported function's type while a *different* cell owns that function's only call site(s). If so, merge those cells into one deliverable (as done here — cell 3 dropped, its scope merged into the data-layer cell) rather than trying to preserve the split with a workaround. This is now evidenced twice in this repo's history in the same shape; treat it as a standing planning check for any TS slice, not a one-off surprise.

---

## Learning 2 — Before dispatching to plan-checker, mechanically compare a cell's verify command against its action text

**Category:** pattern
**Severity:** critical
**Tags:** [verify-authoring, cell-scoping, pre-flight-check]
**Applicable-when:** authoring any cell whose `verify` command asserts a concrete threshold (a minimum count, an exact name/prefix) that only the worker's own new code can satisfy.

### What Happened

`home-shell-workspaces-1`'s first draft verify command required `cargo test --lib -- homeshell_ ... grep -qE '([3-9]|[1-9][0-9]+) passed'` — at least 3 passing tests matching the exact prefix `homeshell_`. The cell's action text never told the worker to write that many tests, or to use that exact prefix. A worker following only the action text could legitimately produce a cell that structurally cannot pass its own verify. This was caught by the orchestrator itself — a direct string comparison between the verify command and the action text — before the plan-checker subagent was even dispatched.

### Root Cause

The verify command was authored with a numeric/naming assertion baked in without a matching explicit commitment in the action text — a drift introduced at cell-authoring time, not something a domain-level review pass is naturally aimed at.

### Recommendation

Whenever a cell's `verify` command contains a concrete threshold a worker's own new code must satisfy (a count, an exact test-name prefix, a named artifact), mechanically confirm the action text explicitly commits to that exact threshold before the cell is sent to plan-checker. This is a cheap, no-domain-knowledge check — pure text comparison — distinct from plan-checker's more expensive structural review, and it should run unconditionally on every authored cell, not only when something feels off.

---

## Learning 3 — A second confirmed instance of FakeHerdr diverging from real herdr (reinforces an existing critical pattern, no new entry)

**Category:** failure
**Severity:** standard
**Tags:** [fake-fidelity]
**Applicable-when:** touching `src/herdr/fake.rs`'s seed data or any code path that reads `Workspace.agent_status` for a zero-agent workspace.

### What Happened

`fake.rs`'s `w3` seed hardcodes `AgentStatus::Idle` for a zero-agent workspace, with a comment claiming this matches real herdr. Direct verification against `upstreams/herdr/src/workspace/aggregate.rs:91-105` showed real herdr's `aggregate_state` defaults to `AgentState::Unknown` for that exact case. Filed as friction (P2, layer `verification`), not fixed in this slice — this feature's D7 sidesteps the discrepancy entirely by hiding the group header badge on a zero-agent-row count, never reading the `workspace_status` value at all, so the wrong fixture value doesn't affect it.

### Root Cause

Same as the already-promoted "Reviewing work that has a fake and a real implementation" critical pattern: a test fixture's comment asserted fidelity to real behavior without being re-verified against upstream source.

### Recommendation

No new critical entry — this is exactly the existing pattern recurring. Worth noting: this was caught by the same direct-source-read discipline that produced D7 in the first place (reading `aggregate.rs` to verify a design concern happened to also expose the stale fixture). Any future feature that reads/displays `workspace_status` text for a zero-agent workspace should check this friction item first rather than trust `w3`'s current `Idle` value.

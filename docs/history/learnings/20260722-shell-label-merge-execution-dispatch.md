---
date: 2026-07-22
feature: pbi-046-shell-card-group
categories: [decision, failure, pattern]
severity: critical, standard, standard
tags: [workspace-grouping, label-identity, worker-dispatch, model-tier]
---

# Learning: Label Is Not Identity, and Gather Workers Are Not Execution Workers

## Decision: Grouping by workspace_label is a deliberate, accepted-risk stand-in for a missing project-identity concept

**Category:** decision
**Severity:** standard
**Tags:** [workspace-grouping, label-identity]
**Applicable-when:** a future feature wants to treat two records as "the same thing" using a human-readable label field.

### What Happened

The original backlog wording for PBI-046 claimed a workspace with both an agent and a shell pane was split into two home-screen groups. On pickup, this was verified false: `src/web/api.rs:80-96` already server-side filters a shell pane out entirely whenever its exact `workspace_id` also has an agent (proven by the passing test `homeshell_workspace_with_agents_contributes_no_shell_rows`). The user re-checked and confirmed what they actually saw was two *different* `workspace_id`s (e.g. a git worktree and its main checkout) sharing the same `workspace_label`.

### Root Cause

`workspace_label` (`src/herdr/wire.rs:111`) is a display string, not an identity key — it defaults to a directory basename but is independently user-renameable (`docs/specs/herdr-port.md:44`), and no "project"/repo-root concept links a worktree's workspace record back to its main checkout anywhere in the data model. Grouping by label is therefore a best-effort visual match, not a proof of same-project.

### Recommendation

When a feature request assumes two records are "the same" based on a matching display/label field, verify whether that field is a real identity key (unique, non-renameable) before implementing a merge on it. If it is not, either (a) build the real identity concept first, or (b) implement the label-match as an explicitly accepted-risk shortcut and document the collision risk in the spec's Business Rules/Open Gaps — never implement it silently as if the label were reliable. Here: `docs/specs/switcher.md` R16 states the risk explicitly; PBI-047 tracks the real fix as a separate, deferred backlog item rather than blocking or silently expanding this feature's scope.

## Failure: A gather-tier (read-only) worker was dispatched for cell execution, wasting a full round-trip

**Category:** failure
**Severity:** critical
**Tags:** [worker-dispatch, model-tier]
**Applicable-when:** dispatching any bee cell for execution (tiny/small single-worker dispatch, or a standard/high-risk swarm wave).

### What Happened

While dispatching cell `pbi-046-shell-card-group-1` for execution, the orchestrator initially spawned it with `subagent_type: "bee-gather"` (the pinned type the swarming-reference's Spawn table lists for the "generation" tier) plus a `[bee-tier: generation]` marker. `bee-gather` is a read-only I/O-offload worker (Read/Grep/Glob only, no Bash/Edit/Write per its own agent definition) — it correctly refused to claim reservations, edit files, run verify, or cap the cell, and returned a `DONE_WITH_CONCERNS` digest instead of doing the work. The cell had to be re-dispatched from scratch with the runtime's default agent type (no `subagent_type` override) and a `model` param (`sonnet`) instead of a tier marker, which then executed correctly.

### Root Cause

The swarming-reference's tier→subagent_type table (`bee-gather` for generation / `bee-extract` for extraction / `bee-review` for review) describes the Delegation contract's **I/O-offload gather/extract/review roles** used for mechanical multi-file lookups (planning bootstrap, schedule computation, etc.) — none of those three rendered agents have write or Bash-mutation tools. Reading that table as "the pinned type for a cell's tier" (rather than "the pinned type for a gather/extract/review-shaped *dispatch*") silently misapplies it to an execution dispatch, which always needs Write/Edit/Bash regardless of judged tier. The correct execution-dispatch pattern is: no `subagent_type` override (runtime default, write-capable) + a `model` param carrying the tier — never one of the three pinned read-only types, and never a bare `[bee-tier: …]` marker paired with a write-capable dispatch (that pairing is reserved for `ceiling`, which has no rendered agent and legitimately runs as the session model).

### Recommendation

Before dispatching a cell for execution (tiny/small single-worker or a swarm wave member), confirm the target subagent type is write-capable. Never pass `subagent_type: "bee-gather"`, `"bee-extract"`, or `"bee-review"` to an execution dispatch — those three are exclusively for read-only I/O-offload gather/extract/review steps. For an execution dispatch, resolve the tier to a `model` param instead (e.g. `sonnet` for generation, per `.bee/config.json`'s `models.claude.<tier>`), and leave `subagent_type` at its write-capable default. Filed as bee friction (see below) since the swarming-reference's own table wording is what caused the misapplication — worth a documentation fix in bee itself, not just a one-off correction here.

## Pattern: Verify a code comment's factual claim against a passing test before trusting it as a locked decision's rationale

**Category:** pattern
**Severity:** standard
**Tags:** [exploring, verification]
**Applicable-when:** a backlog item or code comment cites a prior decision's guarantee as the reason something is (or isn't) possible.

### What Happened

Before locking any CONTEXT.md decisions, the orchestrator dispatched read-only research workers (`bee-gather`/`bee-extract`, correctly used for their intended I/O-offload role here) to trace the actual backend filtering logic and its test coverage, rather than trusting the backlog item's restatement of a code comment ("D3 guarantees the two never share a workspace"). This surfaced that the comment was accurate about the wire payload but the backlog item's premise about *why* (same `workspace_id` overlap) was wrong — the real cause was a label collision across different `workspace_id`s. Locking the CONTEXT.md decision on the corrected premise avoided building the wrong fix (a same-`workspace_id` merge, which would never trigger).

### Root Cause

A comment or backlog note can be operationally true about today's code while being wrong about the mechanism, if nobody re-derives it from the actual guarded/tested behavior.

### Recommendation

When a gray area's premise rests on "the code/comment already guarantees X," dispatch a targeted read-only research pass (bee-gather/bee-extract, matching their intended role) to confirm X against the actual enforcing code and its test, before asking the user to lock a decision or before planning proceeds — this is the same discipline `critical-patterns.md`'s existing "verify a CLI flag against --help" entry already established for external tools, applied here to internal code comments and prior decisions.

## Pattern: CSS layering without mutating the shared base class

**Category:** pattern
**Severity:** standard
**Tags:** [css, styling, variant-scoping]
**Applicable-when:** a UI variant (here: shell rows) needs to override part of a shared base class (here: `.agent-card`) without affecting the base class's other users (here: agent cards).

### What Happened

D4's solid-black shell background was implemented as a new `.shell-row` rule layered on top of the existing shared `.agent-card` rule (`styles.css:454-463`), rather than editing `.agent-card` itself or forking a second card class. The existing markup already applied both classes together (`class="agent-card shell-row"`, `switcher.ts:214`), so only a new CSS rule was needed — no JS change for the background itself. `.agent-card` stayed byte-identical; agent cards were unaffected by construction, not by care.

### Root Cause

The variant's markup already carried both a shared base class and its own variant class before this feature touched it, so no new abstraction was needed — only a natural attachment point.

### Recommendation

When a variant needs to override part of a shared base rule, check whether the variant's markup already carries its own class alongside the shared one (grep the rendered `class="..."` string) before writing a `must_haves.prohibitions` entry or editing the shared rule. Add the override as a new rule scoped to the variant's own class, layered by CSS cascade order, and explicitly prohibit editing the shared rule in the cell that implements it — this is what let this cell's `verify` and `prohibitions` both stay simple and mechanically checkable (`grep -q 'shell-row' styles.css` + `.agent-card` diff-untouched).

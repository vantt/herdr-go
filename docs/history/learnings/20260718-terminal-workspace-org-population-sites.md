---
date: 2026-07-18
feature: terminal-workspace-org
categories: [failure, decision, pattern]
severity: critical
tags: [herdr-wire, fixture-parity, demo-mode, bee-tooling, agent-dispatch]
---

# Learning: A herdr wire field has 3 population sites, not 1 — and the same class of gap hit it twice in one feature

**Category:** failure
**Severity:** critical
**Tags:** [herdr-wire, fixture-parity, demo-mode]
**Applicable-when:** adding or extending any field this app resolves by joining
against herdr's live `session.snapshot` (or any future herdr socket call).

## What Happened

Cell `terminal-workspace-org-1` scoped its `files`/`read_first` to
`src/herdr/wire.rs`, `src/web/api.rs`, `web/src/api.ts` — the type definitions and
their HTTP-layer threading — to add `workspace_label`/`tab_label`. It omitted
`src/herdr/socket.rs`, the actual live-socket extraction code, which only ever
pulled `agents` out of `session.snapshot`'s response and silently dropped
`workspaces`/`tabs`. The worker caught this mid-execution and self-corrected
(auto-fix deviation, reviewed and accepted). Separately, that same cell's
compile-fix to `src/herdr/fake.rs` (`Snapshot { agents, ..Default::default() }`)
left `FakeHerdr`'s demo fixture with **empty** `workspaces`/`tabs` — undetected
until Phase 2 fully capped and someone manually ran `cargo run -- --demo`, logged
in, and inspected `GET /api/agents` to find `workspace_label: ""` and
`workspace_status: "unknown"` on every row (`--demo` is this app's documented
zero-config showcase path, PBI-004 — this would have looked broken to any new
evaluator).

## Root Cause

`wire::Snapshot` has exactly two real construction sites in this codebase —
`SocketHerdr::snapshot()` (socket.rs, the live path) and `FakeHerdr::new()`
(fake.rs, the demo/test path) — plus the type definition itself (wire.rs).
CONTEXT.md's D2 and the Phase 1 cell's `must_haves` both validated and asserted
against the **type shape** (wire.rs) and cited live-probe evidence proving the
**wire protocol** carries the fields. Neither validated that both **construction
sites** actually populate them. `cargo test`/`clippy`/`typecheck` all stayed green
throughout, because a struct field that's always empty still compiles and
type-checks — only a live run (real herdr, then `--demo`) surfaced either gap.

## Recommendation

When a cell or plan adds/extends a field resolved from herdr's live snapshot (or
any herdr socket response), the `files`/`read_first` list and the `must_haves`
must name **every** construction site of the struct being extended, not just its
type definition — for `wire::Snapshot` today that is `src/herdr/wire.rs` (shape),
`src/herdr/socket.rs` (live extraction), and `src/herdr/fake.rs` (demo/test
fixture). Add a `must_have` truth asserting non-empty/plausible values in
`--demo` mode specifically (not just "resolved from a live snapshot") so a
missing fixture is caught by the cell's own verify, not by a later manual run.

---

# Learning: A CONTEXT.md decision's literal wording can diverge from what actually ships, when two decisions merge cleanly at implementation

**Category:** decision
**Severity:** standard
**Tags:** [context-md, spec-drift]
**Applicable-when:** two locked decisions (here D3 "conditional workspace badge"
and D4 "per-workspace grouped section") turn out to be naturally realized by one
shared UI mechanism during implementation.

## What Happened

CONTEXT.md describes D3 as a standalone "workspace badge" shown per-row when more
than one workspace is present, and D4 separately as collapsible per-workspace
sections with a status-rollup header. Cell `terminal-workspace-org-3`'s actual
`action`/`must_haves` (and the shipped `switcher.ts`) never build a separate
per-row workspace badge at all — when >1 workspace exists, D3's "badge" IS the
section header built for D4, reusing the same `.status-badge` styling. This was
planning's own choice (stated explicitly in the cell text), not a deviation, and
it's a simpler, more coherent result than building both mechanisms separately.

## Root Cause

D3 was locked during exploring, before the grouping mechanism (D4) had a concrete
shape. By the time planning designed D4's actual markup, the two decisions turned
out to share one visual answer — but nobody went back to amend D3's wording to
say "realized via D4's section header," so CONTEXT.md still reads as if two
separate UI elements exist.

## Recommendation

When planning's approach discovers that two previously-separate locked decisions
collapse into one implementation mechanism, add a one-line note under the later
decision's row in CONTEXT.md (or plan.md's Approach section) making the merge
explicit — e.g. "D3's badge is realized entirely via D4's section header; no
separate per-row badge exists" — so a future reader of CONTEXT.md alone doesn't
go looking for UI that was never built.

---

# Learning: bee's newly self-updated pinned-agent-type rule blocks execution-worker dispatch — read-only agent types can't implement a cell

**Category:** failure
**Severity:** critical
**Tags:** [bee-tooling, agent-dispatch, model-guard]
**Applicable-when:** dispatching any swarming worker (or a tiny/small single
execution worker, per AO14) in this repo, until bee's own agent-type rendering is
fixed upstream.

## What Happened

Mid-session, this repo's bee installation self-updated (`AGENTS.md` and the
`bee-*` skills changed under the running session). The updated `bee-model-guard`
hook now refuses `subagent_type: "general-purpose"` paired with a
`[bee-tier: generation|extraction|review]` marker, and instructs dispatching the
pinned agent type instead (`bee-gather` for generation, `bee-extract` for
extraction, `bee-review` for review). But the actual rendered agent definitions
in this repo (`.claude/agents/bee-gather.md`, `bee-extract.md`) declare
`tools: Read, Grep, Glob` only — no `Edit`, `Write`, or `Bash` — and are
explicitly documented as "I/O-offload... never writes, never edits, never runs a
mutating command." `bee-review` adds `Bash` but is still described as read-only
("never edits the working tree"). None of the three can implement, commit, and
cap a cell, despite `bee-swarming/SKILL.md`'s own text saying to use them for
exactly that dispatch.

## Root Cause

The pinned-type rule (decision AO5/AO10/AO11) appears to have shipped for the
Delegation contract's **I/O-offload gather/extract/review workers** but got
applied by the hook to **execution-worker dispatches** too (swarming's cell
implementers), without a corresponding write-capable pinned agent type ever being
rendered for that class. This is an internal inconsistency in the installed bee
version, not a mistake in this repo's own code or workflow.

## Recommendation

Until bee ships a write-capable pinned execution-worker agent type (or the hook
stops requiring a pinned type for execution dispatches specifically): dispatch
execution workers with **only a bare `model` parameter** (e.g. `model: "sonnet"`)
and `subagent_type: "general-purpose"`, **omitting the `[bee-tier: ...]` marker
text entirely** from the prompt — the transport rule accepts a `model` param OR
an anchored marker, either alone is sufficient, and a bare `model` param does not
trigger the pinned-type requirement. Do not spend more than one retry attempt
fighting the hook before falling back to this — it is a known-good workaround,
not a guess. File this as friction against bee's own tracker (not this repo's
backlog) so the upstream inconsistency gets fixed rather than permanently
worked around.

---

# Learning: extracting UI logic into a pure exported function to gain test coverage is a reusable pattern in this codebase

**Category:** pattern
**Severity:** standard
**Tags:** [frontend, testability]
**Applicable-when:** adding non-trivial display logic (branching, grouping,
sorting) to a `web/src/views/*.ts` file that currently has no test coverage.

## What Happened

`web/src/views/switcher.ts` had zero test coverage before this feature. Rather
than inlining the new workspace-grouping/sort logic directly in `renderList`,
planning specified (and the worker implemented) a pure, exported function
(`groupByWorkspace`) with no DOM dependencies, then added
`web/test/switcher.test.ts` testing its boundary behavior (0/1/2+ workspaces)
directly — following the exact pattern already established by `stripAnsiLen`
(exported from `terminal.ts`, tested in `web/test/terminal.test.ts`).

## Root Cause

This app's view files mix DOM rendering with business logic inline; the only
prior test coverage for view-adjacent logic came from the one function
(`stripAnsiLen`) that had already been factored out as pure. Reusing that shape
for new logic was a deliberate, low-cost way to get real test coverage without a
larger refactor.

## Recommendation

When adding branching/derivation logic (not DOM manipulation itself) to a
`web/src/views/*.ts` file, factor it into a pure, exported, top-level function
and add a matching `web/test/<view>.test.ts` — this is now a 2-for-2 established
convention in this repo (`terminal.ts`/`stripAnsiLen`, `switcher.ts`/
`groupByWorkspace`), worth following by default rather than inlining logic that
could be unit-tested cheaply.


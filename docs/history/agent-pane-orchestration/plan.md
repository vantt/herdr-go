---
artifact_contract: bee-plan/v1
mode: high-risk
# approved_gate2: <unset>
---

# Agent Pane Orchestration — Plan

**Feature:** agent-pane-orchestration · **Backlog:** PBI-043
**Sources of truth:** `CONTEXT.md` (D1-D20), `approach.md` (chosen mechanism + risk map)

## Mode gate

Risk flags counted from the request and CONTEXT.md:

1. **data loss** — the merge pane runs `bee worktree merge --cleanup` unattended, which performs `git worktree remove --force` and `git branch -d` (D3, D15).
2. **external systems** — the `herdr` binary and, for D11, GitHub.
3. **multi-domain** — herdr CLI, bee CLI, git worktrees, and bee skill authoring.

`data loss` is a hard-gate flag, so the lane is **high-risk** regardless of the count. Smaller modes are insufficient: `standard` would skip the persona panel over an unattended process that deletes branches, and the AGENTS.md red flag "a hard-gate change routed below high-risk" applies directly. Slice 1 in isolation would score `standard`; the feature is planned at the higher lane because slice 2 is inside it (D7).

Product files touched: the skill directories plus one runner script — `.claude/skills/**` and `.agents/skills/**` are *this repo's* managed projections, and `docs/**` is never counted (D6). The lane is set by the flags, not the file count.

## Discovery

L2, recorded in `approach.md`: four candidate loop mechanisms compared, one chosen. No repo precedent exists for a long-running agent loop — verified, not assumed.

## Slices

### Slice 1 — the cockpit, and dispatch

The skill, the runner, the layout bootstrap, and the dispatch loop.

1. **The skill** — `SKILL.md` plus references, authored in both managed roots (D11). Contains the dispatch loop's instructions and the merge loop's (slice 2 wires the latter up, but one skill covers both roles; the runner passes the role).
2. **The runner script** — the `while true; do claude -p …; sleep 60; done` driver from `approach.md`, parameterised by role.
3. **Layout bootstrap** — one command that creates the cockpit and runtime tabs and the cockpit's three panes (D13), leaving each control pane running its runner. Panes name themselves on first iteration (D17).
4. **The dispatch loop itself** — free-slot count (D18), dispatchable set (D1), impact ranking announced to chat (D16), lane refusal failing closed (D6), `bee worktree new` then `herdr pane split --cwd` then start the working agent (D14).

**Not in slice 1:** any merging, any `--cleanup`, any pane closing.

### Slice 2 — merge

The merge loop (D2, D3, D15): detect finished worktrees from bee state only (D20), merge with `--cleanup`, close the runtime pane, escalate `MERGE_VERIFY_RED` to chat without retrying.

### Slice 3 — upstream

Port and open the PR to `thanhsmind/beegog` from the fork (D11). Acceptance is the PR opened, not merged.

## Test matrix (edge dimensions, scaled to the lane)

| Dimension | Case that must be covered |
|---|---|
| Empty | Nothing dispatchable — the loop announces nothing, changes nothing, and keeps looping |
| Boundary | Exactly 4 occupied slots — dispatch does not create a 5th |
| Adversarial | A backlog row whose lane cannot be classified — skipped as high-risk (D6), reported |
| Adversarial | A runtime pane with no label, and one whose label is stale with a dead session — reported, neither reclaimed nor counted free (D18) |
| Failure | A `bee worktree new` that refuses (slug exists, dirty tree) — reported, loop continues (D19) |
| Failure | `MERGE_VERIFY_RED` — no merge, no retry, escalated (D3, slice 2) |
| Concurrency | Two worktrees finishing in the same 60-second window |
| State | An item whose CONTEXT.md exists but which already has a worktree — not re-dispatched (D1c) |
| Resource | 4 working agents live while one verify holds the lock (D12) |

## Risks

Carried from `approach.md`. The two HIGH items — D6's fail-closed refusal, and unattended `--cleanup` — are the gating proofs for slice 1 and slice 2 respectively.

## Open questions for validating

1. D6's lane-classification mechanism (fail-closed behavior already decided; mechanism not).
2. Whether the runner script lives inside the skill directory or this repo's `scripts/` — decides whether it ports upstream with the skill (D11).
3. What gesture stops the loops (D19 says only the human stops them; by what means is unspecified).

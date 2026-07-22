# Implement Plan — agent-pane-orchestration, slice 1

**Status:** awaiting Gate 3 · **Lane:** high-risk · **Date:** 2026-07-22
**Projected from:** `plan.md` (frozen at Gate 2), `approach.md`, cells `agent-pane-orchestration-1..4`
**Decisions:** `CONTEXT.md` D1-D20 — cited, never reinterpreted

## What slice 1 delivers

A new bee skill, `bee-orchestrating`, plus two scripts, that let a dispatch agent run forever in a named herdr pane and fill free runtime slots with ready backlog items. **Nothing merges and nothing is deleted in this slice** — that is slice 2.

## Affected files

| File | Cell | Why |
|---|---|---|
| `.claude/skills/bee-orchestrating/scripts/control-loop.sh` | 1 | The unbounded loop; one cold `claude -p` per iteration |
| `.agents/skills/bee-orchestrating/scripts/control-loop.sh` | 1 | Byte-identical Codex-root copy (D11) |
| `.claude/skills/bee-orchestrating/SKILL.md` | 2 | Dispatch-role instructions, readable cold |
| `.agents/skills/bee-orchestrating/SKILL.md` | 2 | Byte-identical copy |
| `.claude/skills/bee-orchestrating/scripts/bootstrap-cockpit.sh` | 3 | Builds the D13 layout in one command |
| `.agents/skills/bee-orchestrating/scripts/bootstrap-cockpit.sh` | 3 | Byte-identical copy |
| `.claude/skills/bee-orchestrating/references/dispatch-dry-run.md` | 4 | The three recorded proof transcripts |
| `.agents/skills/bee-orchestrating/references/dispatch-dry-run.md` | 4 | Byte-identical copy |

Nothing under `src/`, `web/src/`, or `docs/specs/` is touched (D10).

## Implementation steps

**Wave 1 — cells 1 and 2 in parallel**

1. *Runner* (cell 1). Shell loop: run the role's prompt through a fresh `claude -p … --model sonnet`, then `sleep` the interval. No `--continue`, no `--resume` — the cold start per iteration is what keeps context flat across thousands of iterations. A non-zero exit is reported and the loop proceeds to the next sleep; there must be **no** code path where a failed iteration ends the loop, achieved structurally in the shell rather than by instructing the agent (D19). Test-only flags `--max-iterations` and `--command` exist so the loop can be proved without spending tokens.
2. *Skill* (cell 2). The document a cold agent reads every iteration, so it must be self-sufficient. Dispatch role: occupancy from bee state and pane labels, never from `agent_status` (D18, D20); dispatchable set per D1 via a reverse index over `docs/history/*/CONTEXT.md`; refuse everything when `gate_bypass` is below `full`; skip any row that is hard-gate **or unclassifiable** (D6, fail closed); rank by impact and announce the choice with its reason into the chat pane before acting (D16); self-name first (D17). Defines `--dry-run`: full decision reported, nothing changed.

**Wave 2 — cell 3**

3. *Bootstrap.* `herdr tab create` for cockpit and runtime; in cockpit, `pane split --direction right` then `--direction down` on the right pane to yield chat / dispatch / merge (D13). Must **not** pre-label the control panes — labelling is the occupant's job (D17). `--no-start` builds the layout without launching agents so it can be verified cheaply; `--dry-run` prints the commands.

**Wave 3 — cell 4**

4. *Proof.* Three dry-runs recorded as a reference document: the real backlog (must conclude nothing is dispatchable), an ordinary fixture (must name one PBI with a reason), and a fixture carrying a hard-gate row plus an unclassifiable row (must skip both, with reasons). Fixtures live under `.bee/tmp/`, never in `docs/backlog.md`.

## Validation plan

Accepted evidence is recorded in `reports/validation-slice-1.md`. Already proven: every herdr and `claude` flag the design uses, live `pane current --current` output, byte-identical skill roots, and an acyclic three-wave schedule. Still to prove, as cell verifies:

- **Cell 1** — three consecutive failing iterations all run; the loop survives its own errors.
- **Cell 4** — the fail-closed refusal. This is the gating proof: until it passes, the dispatcher must not be left running unattended.

Every cell is `change_class: behavior`, so capping requires the verify observed **red before the work and green after**; an assertion is not evidence.

## Prohibitions for this slice

- No merging, no `--cleanup`, no pane closing (slice 2).
- No state file or registry (D18).
- No `herdr worktree create|open|remove` — worktrees come from `bee worktree new` only.
- No change to herdr-go product code or config (D9, D10).

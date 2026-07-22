# Validation — agent-pane-orchestration, slice 1

**Lane:** high-risk · **Date:** 2026-07-22 · **Cells:** agent-pane-orchestration-1..4

## Reality gate

| Check | Result | Evidence |
|---|---|---|
| MODE FIT | **PASS** | `bee worktree merge --cleanup` performs `git worktree remove --force` and `git branch -d` (documented in the CLI's own `worktree merge` help). Unattended deletion is the data-loss hard-gate flag; one hard-gate flag sets high-risk regardless of count. |
| REPO FIT | **PASS** | Every herdr verb the plan uses exists: `tab create`, `pane split --direction right\|down --ratio --cwd`, `pane close`, `pane list`, `pane rename`, `pane current`, `agent start --cwd -- argv` (checked against `/home/vantt/.local/bin/herdr --help`). `herdr pane current --current` returned a live record (`pane_id w7:pN`, `workspace_id w7`). `claude --help` confirms `-p/--print`, `-c/--continue`, `-r/--resume`, `--model`, `--permission-mode`, `--append-system-prompt`. The two managed skill roots are byte-identical at the head of `bee-hive/SKILL.md`, so a new skill can be hand-authored into both. |
| ASSUMPTIONS | **PASS** | See matrix. No assumption rests on model knowledge; the two still open are scheduled as execution proofs, not asserted. |
| SMALLER PATH | **PASS (considered, kept)** | Cell 3 (bootstrap script) could be dropped — the layout can be built by hand once with four herdr commands. Kept because the layout must be rebuilt after every machine restart, and doing that by hand each time is precisely the friction this feature exists to remove. Recorded rather than assumed. |
| PROOF SURFACE | **PASS after repair** | Initially **FAIL**: `approach.md` names D6's fail-closed refusal as a HIGH risk requiring proof "before the dispatch loop is ever started unattended", but cell 4 only seeded a *dispatchable* fixture — the highest risk in the slice had no proof at all. Cell 4 was rewritten to add case (c): a hard-gate row and an unclassifiable row, both of which must be skipped with a stated reason. |

## Feasibility matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| A shell loop can drive headless `claude` with no human input | MEDIUM | flag existence | `claude --help`: `-p/--print`, `--model`, `--permission-mode`, `--append-system-prompt` | **PASS** |
| A failing iteration cannot terminate the loop | MEDIUM | command output | cell 1 verify: 3 consecutive failing iterations must all run | **PENDING** (execution proof) |
| herdr can build the D13 layout | LOW | CLI surface | `tab create`, `pane split --direction --ratio --cwd`, `pane close` all present | **PASS** |
| An agent can learn its own `pane_id` | LOW | runtime probe | `herdr pane current --current` returned `pane_id`, `tab_id`, `workspace_id`, `label`, `agent_status` live | **PASS** |
| A new skill can be hand-authored into both managed roots | LOW | file inspection | `.claude/` and `.agents/` copies of `bee-hive/SKILL.md` identical at head | **PASS** |
| The dispatcher fails closed on a hard-gate or unclassifiable row | **HIGH** | dry-run transcript | cell 4 case (c) — **the gating proof for this slice** | **PENDING** (execution proof) |
| Cell graph is executable | LOW | command output | `bee cells schedule`: `cycles: []`, `unsatisfiable_deps: []`, waves `[1,2] → [3] → [4]` | **PASS** |
| Advisor consult (AO2b) | — | config read | `.bee/config.json` `advisor: null` — unconfigured; recorded and proceeding, per the validating contract | **RECORDED** |

## Spikes

None run. The two open assumptions are not spike-shaped: neither can be answered by a throwaway proof cheaper than the cell that must produce it anyway. Both are attached to cells as their `verify`, so execution cannot cap without them.

## Notes carried to execution

- All four cells are `change_class: behavior` and the CLI advised that each needs `red_failure_evidence` at cap time: the verify must be observed **failing before the work** and passing after. Capping will refuse otherwise.
- Cell 3's verify creates and closes a throwaway herdr workspace. It must never target a workspace the human is using.

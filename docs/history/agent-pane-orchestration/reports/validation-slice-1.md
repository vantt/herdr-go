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

## Verdict — NOT READY, RETURN TO PLANNING

The high-risk persona panel returned 7 BLOCKERs. Three were re-verified directly and hold:

1. **Every file slice 1 writes is gitignored.** `git check-ignore -v` resolves `.claude/skills/bee-orchestrating/SKILL.md` to `.gitignore:45` and the `.agents/` copy to `:46`. Line 43 states the intent: *"Agent skills = installed tooling, regenerable from their sources. Do not track."* So no cell can produce a commit (AGENTS.md rule 8), `cells cap --files` would record paths git cannot see, and CONTEXT.md's claim that git history protects the work against an onboarding `remove_skill` is false. This is not a stray glob — naming the skill `bee-*` places it inside both the untracked set and the sync's managed namespace at once.
2. **Cell 4's verify can never pass.** `git status --porcelain` is never empty: ` M .bee/logs/tools.jsonl` is force-tracked (`.gitignore:51`) and rewritten by every bee invocation, including the `cells cap` that runs the verify.
3. **Cell 2's verify passes vacuously.** `A && B && !C || D && E` is left-associative at equal precedence, so any SKILL.md containing a line matching `never.*agent_status` — which every correct one will — satisfies the right operand and prints `SKILL_OK` even when `diff -q` failed and the D-id loop never ran. Byte-identity and all seven citations were unenforced.

Further BLOCKERs accepted without independent re-verification (the panel cited command output for each):

4. **The headless agent's `--permission-mode` is chosen nowhere.** This, not D6, is the actual blast-radius control for an unattended loop. At the default it stalls on a permission prompt with no TTY; at `bypassPermissions` it has unrestricted tool access while holding `gate_bypass: full`.
5. **No stop gesture exists.** `plan.md` left it open, yet cell 1 ships the unbounded loop and cell 3 starts it — the only halt available is killing the pane.
6. **The cockpit's cwd is unspecified and `bee worktree new` refuses outside the MAIN checkout**, so as written every dispatch iteration would fail forever while the loop dutifully continues — the silent stall D6's rationale exists to prevent.
7. **D6's classification mechanism still has no owning cell**, and the real spawn path (`pane split --cwd` into a real worktree, agent started) ships with dry-run evidence only.

Also accepted: cell 3's asserted 2 tabs / 4 panes contradicts its own action (a fresh workspace already carries a root tab and pane), cell 3 leaks a workspace into the human's live session on any failure, and cell 3 should depend on cell 2.

Slice 1 returns to planning. Two of these are user decisions, not repairs.

## Second structural pass — after the reshape

Iteration 2 of a maximum 3. Of the seven original blockers: **five RESOLVED** (gitignored paths, the impossible verify, the vacuous verify, the permission mode, and the two unowned owners — D6's classifier and the real spawn path), **two PARTIAL**, plus **two new blockers** the reshape itself introduced.

Both PARTIALs were the same mistake in two places — repairing the mechanism without asserting it:

- The stop file existed and was tested, but the SKILL.md verify never required it to be **documented**, so the human's only halt gesture lived in a shell script; and nothing cleared a stale stop file at bootstrap, so one leftover file would silently no-op every future loop start — the same silent-stall class as the cwd bug it sat next to.
- `--main-root` became required and was passed to both tabs, but the verify asserted only tab and pane **counts** — never the cwd, which was the entire point of the repair.

New blockers, both repaired:

- **Cell 9 depended on an input contract no cell created.** Its fixture runs needed a backlog override that cell 6 did not offer, and D1 dispatchability also needs a history root and a cells store no fixture can provide. Runs (b) and (c) are now scoped to the classifier level — which *is* the D6 mechanism — while run (a) remains the full dispatch role against real state.
- **Cell 10's cleanliness check was keyed to a name the cell never fixed.** `grep -i throwaway` would have passed silently had the worker chosen any other slug. The slug is now literal: `apo-throwaway-spawn`.

Eleven warnings were also applied. The ones that mattered: a stop-file assertion that passed for a nonexistent file (a positive control now distinguishes "stopped" from "never ran"); greps that would have fired on a *comment* the cell's own prohibitions invite the worker to write; a `trap` installed one statement too late, leaking a workspace into the live session; a bash-only herestring where bee's runner uses `/bin/sh` (dash); hardcoded grant counts that would go red when an unrelated sibling worktree merges; and transcript assertions satisfiable by typing the words — cell 9 now re-runs the classifier and diffs against the recorded JSON.

Confirmed sound and deliberately left alone: cell 6's shell function and its PBI-044→false / PBI-038→true expectations (both checked against the real rows), cell 8's JSON extraction and its 3-tab/5-pane arithmetic (checked against `herdr api schema --json`), and the dependency graph.

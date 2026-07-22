---
name: herdr-orchestrating
description: >-
  Drives the unattended dispatch control pane of the agent-pane-orchestration loop: picks the highest-impact ready backlog item, refuses unsafe or unclassifiable work, and starts a working agent in a fresh worktree via the herdr CLI. Use for exactly one dispatch iteration at a time — the control-loop runner invokes this skill fresh, every 60 seconds, with no memory of any earlier iteration. Slice 1 (dispatch) only; the merge role is a separate future slice and is out of scope here.
metadata:
  version: '0.1'
  ecosystem: bee
  dependencies:
    nodejs-runtime:
      kind: command
      command: node
      missing_effect: unavailable
      reason: The dispatchable-set build and lane classification run through the vendored .bee/bin helpers and scripts/classify-lane.mjs.
    herdr-cli:
      kind: command
      command: herdr
      missing_effect: unavailable
      reason: Every pane/tab/agent action in this role goes through the herdr binary directly (D8) — there is no other way to reach a pane.
---

# herdr-orchestrating — dispatch role

You are the **dispatch** control pane of the agent-pane-orchestration loop (`docs/history/agent-pane-orchestration/CONTEXT.md`, all decisions below cite that document by id). This skill lives at `.claude/skills/herdr-orchestrating/` and `.agents/skills/herdr-orchestrating/` — named `herdr-orchestrating`, deliberately not `bee-*` (D21), because the repo's `.gitignore` untracks `bee-*` skill directories in both roots and this skill must stay ordinary tracked content. Read this whole file before doing anything: **you have no memory of any earlier iteration.** `control-loop.sh` invokes you as a brand-new, cold `claude -p` process every 60 seconds (D4) — nothing carries over between iterations except what is durably recorded in bee state, git, and the herdr workspace itself. Every fact this role needs is either written in this file or read live, right now, from bee/herdr/git. Never assume "I already checked that" — you didn't; a different process did, or nobody did.

**Slice boundary.** This role only dispatches. It never finishes, retires, or reclaims a worktree or a pane — that is a separate future role (the merge side of this same feature, its own slice). If you find yourself about to run a command that merges a branch back into main, deletes a worktree, or closes a pane, stop: that action belongs to the other role, not this one.

## 0. Where you are running

This role assumes its own cwd is the **MAIN checkout** — never a worktree (D14 creates worktrees FROM main; it does not run inside one). If `git rev-parse --show-toplevel` resolves to a path containing `--wt--`, that is a fatal misconfiguration: report it as an anomaly (see §5) and stop this iteration without dispatching anything.

The human's stop gesture is `.bee/tmp/herdr-orchestrating.stop`: `control-loop.sh` already checks for that file before it ever starts an iteration, so by the time this role is running the loop has not been asked to stop. Nothing in this file needs to check it again; it exists purely so you understand why the loop might simply never invoke you again — removing the file is what lets a human resume it.

## 1. Learn who you are, and self-name (D17)

herdr assigns no name of its own to a pane — an unnamed pane has no `label` field at all. The first act of every agent in this system, every iteration, is:

```
herdr pane current --current
```

This returns your own `pane_id`, `tab_id`, `workspace_id`, and `label` (absent if unset). If `label` is not exactly `dispatch`, claim it now: `herdr pane rename <pane_id> dispatch`. If it already reads `dispatch` — which it will on every iteration after the first, since a label is pane metadata that outlives the cold process that set it — do nothing; do not re-rename. Record `tab_id` and `workspace_id`: everything below scopes its herdr calls to this workspace, and your `tab_id` is the **cockpit** tab (D13) — you are physically running inside it.

## 2. Refuse to operate below `gate_bypass: full` (D6)

```
node .bee/bin/bee.mjs status --json
```

Read `gate_bypass_level`. This role may only pick up work when it is exactly `full` or `total`. At `off` or `normal`, do not build a dispatchable set, do not classify anything, do not spawn anything — announce the refusal into the chat pane (§3) with the actual level and why (an auto-created worktree inherits this repo's `gate_bypass` level, D6, and an unattended agent must never inherit `normal`'s latitude for hard-gate work), then end the iteration. This is a live check, every iteration — the level can change between polls.

## 3. Find the chat pane (nothing labels it)

The human's pane carries no label — it is identified structurally, not by name. Because your own pane lives in the cockpit tab, calling pane layout on yourself returns exactly the cockpit tab's panes with their screen geometry:

```
herdr pane layout --current
```

Among the panes in that layout, the chat pane is the one with the smallest `rect.x` (leftmost; break ties on the smallest `rect.y`), excluding your own `pane_id`. Per D13's layout — chat left, dispatch top-right, merge bottom-right — that leftmost pane is chat. Use its `pane_id` as the target of every `herdr pane send-text <chat_pane_id> "..."` call in the rest of this document. Resolve this once per iteration; do not assume yesterday's pane_id is still valid — panes can be closed and recreated by the human.

## 4. Count occupied runtime slots, and report anomalies once (D5, D18, D20)

Resolve the **runtime** tab: `herdr tab list --workspace <workspace_id>`, the tab whose `label` is `runtime` (set by the cockpit bootstrap per D13). If no tab carries that label, fall back to "the one tab in this workspace that is not your own `tab_id`" — D13 fixes exactly two tabs, so exclusion is unambiguous.

List its panes: `herdr pane list --workspace <workspace_id>`, filtered to that `tab_id`. For each pane:

- **Unlabelled** → anomaly candidate (below). It cannot be counted as occupying a slot for a specific PBI because nothing says which one it is.
- **Labelled with slug `S`** → this pane's worktree needs the D2/D20 "finished" test before it can be counted. Resolve the worktree's absolute path from the pane's `cwd`/`foreground_cwd`, then check, against **that worktree's own bee store** (each worktree has its own `.bee/`, so run these with that path, e.g. `(cd <path> && node .bee/bin/bee.mjs status --json)`):
  1. `phase` is `compounding-complete`;
  2. zero cells in `open` or `claimed` for that worktree's feature (`(cd <path> && node .bee/bin/bee.mjs cells list --feature <S> --json)`);
  3. `git -C <path> status --porcelain` is empty (clean tree);
  4. `git -C <path> rev-parse --abbrev-ref HEAD` is exactly `wt/<S>`.

  If all four hold, that worktree is **finished** (D2) — per D18 it does **not** count as an occupied slot, even though its pane still physically exists; this role never closes it (slice boundary, §"Slice boundary" above). If any of the four fails, the pane **counts as occupied**.

**`agent_status`/`agent_session` from `herdr pane list` are read for exactly one purpose in this entire role: spotting an anomaly** — a labelled pane whose worktree is not finished by the test above, yet whose agent session has died (`agent_status` idle/unknown with no live `agent_session`, or a `foreground_cwd` that no longer matches the worktree) — and it is never read as proof that a working agent, or the item it is running, has finished (D18, D20). A merely-idle agent mid-item is expected and is not an anomaly; only a dead session on unfinished work is.

`occupied_count` = the number of labelled, not-yet-finished runtime panes. D5's cap is 4. If `occupied_count >= 4`, no slot is free this iteration — still run the anomaly check below, but do not build or announce a dispatch decision (§6-7).

**Anomalies are reported exactly once, never once per poll** — a report repeated every 60 seconds for the rest of the day is a report nobody reads. There is no state file or registry to remember what was already said (D18 forbids one); instead, before sending a new anomaly report, read the chat pane's own recent scrollback —

```
herdr pane read <chat_pane_id> --source recent --lines 200
```

— and check whether it already names this exact `pane_id` with this exact reason. If it does, say nothing. If it does not, send exactly one line naming the `pane_id`, the slug (if labelled), and the reason, and take no other action: do not relabel, close, or reclaim the pane. Reporting is the whole of this role's response to an anomaly.

## 5. Build the dispatchable set (D1)

A PBI is dispatchable **iff all four of D1's conditions hold** — build the reverse index and check every condition fresh, every iteration:

- **(a) Ready.** A `docs/history/<slug>/CONTEXT.md` exists whose `**Backlog:**` line names the PBI. There is no PBI→slug column anywhere; the map only goes slug→PBI, so build it by reading that line out of every `docs/history/*/CONTEXT.md` (e.g. `grep -rn '^\*\*Backlog:\*\*' docs/history/*/CONTEXT.md`), then invert it.
- **(b) `in-flight`.** The PBI's row in `docs/backlog.md` (`| ID | PBI | Status | Ghi chú |`) has Status exactly `in-flight` — not `proposed`, not `done`.
- **(c) No worktree grant.** `node .bee/bin/bee.mjs worktree list --json` → its `grants` object. A grant exists for `<slug>` when any key ends with `--wt--<slug>` (grant keys are `<main-checkout-basename>--wt--<slug>`, e.g. `herdr-gateway--wt--<slug>`) — if one does, this PBI is already under way; skip it.
- **(d) Zero cells.** `node .bee/bin/bee.mjs cells list --feature <slug> --json` returns an empty array.

Only rows passing all four go forward to §6.

## 6. Lane-safety filter (D6) — and what `lane_safe` is not

For every candidate from §5, run the classifier already built for this purpose (cell 6 of this feature, do not modify it):

```
node .claude/skills/herdr-orchestrating/scripts/classify-lane.mjs <PBI-ID>
```

(run the copy under whichever skill root your runtime reads — `.claude/` for Claude Code, `.agents/` for Codex; both are byte-identical). It emits one JSON object: `{pbi, lane, hard_gate_flags[], lane_safe, reason}`, fail-closed — anything it cannot classify with confidence comes back `lane_safe:false`. Drop every candidate whose `lane_safe` is `false`.

**`lane_safe` is only ONE of D1's four dispatchability conditions — it is not a synonym for "dispatchable."** It answers a narrower question than D1 does: "does this row's backlog text look safe for an unattended agent to pick up unsupervised" (no hard-gate flag, not 4+ mode-gate risk flags). It says nothing about whether the row is `in-flight`, already has a worktree, or still has open cells — those are §5's job. A row can be `lane_safe: true` and still be completely ineligible because it failed §5; conversely, passing §5 alone never makes a row eligible — §5 and this filter are both required, and neither one substitutes for the other. Never widen "passed lane classification" into "should be dispatched": that conflation is exactly what would let an unattended loop start picking up work it has no business touching.

## 7. Rank and announce before acting (D16)

"Highest impact" is this agent's own judgement over the surviving rows from §6 — there is no stored priority field, and none should be added (the backlog table has no priority column by design). Read the surviving rows' full text (description + notes) and choose. Before taking any action, send the choice and the reason for it into the chat pane found in §3:

```
herdr pane send-text <chat_pane_id> "dispatch: picking <PBI-ID> (<slug>) because <reason>"
```

If nothing survives §5/§6, or no slot is free (§4), there is nothing to announce or dispatch — end the iteration quietly (an empty runtime tab poll is normal, not an anomaly).

## 8. Spawn the working agent (D14, D9, D22, D4)

In order, all from the MAIN checkout:

1. `node .bee/bin/bee.mjs worktree new --feature <slug> --json` — creates and registers the worktree in one move; read the resulting path from its output.
2. Open a fresh pane at that path in the **runtime** tab. Pick a pane already in that tab to split from (any one is fine to start; to keep the tab roughly balanced as it fills, split whichever existing runtime pane currently has the larger `rect` area from `herdr pane layout`, and choose `--direction right` if that pane is wider than it is tall, otherwise `--direction down`):
   ```
   herdr pane split <target_pane_id> --direction right|down --ratio 0.5 --cwd <worktree_path> --no-focus --json
   ```
   Capture the new pane's id from the result.
3. Start the working agent there:
   ```
   herdr agent start <slug> --cwd <worktree_path> --workspace <workspace_id> --tab <runtime_tab_id> -- claude --model sonnet --permission-mode bypassPermissions
   ```
   `--model sonnet` is D4's fixed model for every agent in this system, control and working alike. `--permission-mode bypassPermissions`, with no tool allowlist narrowing it, is D22's explicit, accepted-risk choice — it is the only mode that does not stall forever on a permission prompt with no TTY attached; do not add flags that narrow it. herdr-go's own config is untouched (D9) — the model and permission flags travel as argv at spawn time, never as a new `agent_presets` entry.

   The exact wiring between step 2's freshly split pane and step 3's `agent start` call has not been proven live by this document; if `docs/history/agent-pane-orchestration/references/spawn-proof.md` exists and has content, it is the authoritative worked example for this exact sequence — follow it. If it does not exist yet or is empty, you are doing this for the first time: after step 3, run `herdr pane list --workspace <workspace_id>` filtered to the runtime tab and confirm exactly **one** new pane appeared with a live agent at the right cwd, not two panes or zero. If anything looks wrong, report it into the chat pane (§3's pane, one line, plain description) and do not repeat the spawn blindly on the next iteration — a repeated blind retry against an unproven integration point is how a cold loop turns one mistake into 1440 a day.

The working agent that starts here is on its own from that point — it runs the ordinary bee chain inside its own worktree until its item is finished (D2). This role does not watch it, does not wait on it, and does not act on it again; the next iteration's occupancy count (§4) is how its progress is next observed.

## `--dry-run`: report the whole decision, change nothing

There is no CLI to parse for this role — recognize `--dry-run` from the instruction you were given for this iteration (verbatim in the prompt, or an explicit note in the task). It is for manual verification of the decision logic, never something `control-loop.sh` passes on its own unbounded loop.

Under `--dry-run`, run every read in §1-§7 exactly as written — self-identification, the `gate_bypass` check, chat-pane resolution, occupancy counting, the dispatchable-set build, lane classification, ranking — and produce the same decision you would otherwise announce and act on. The difference is entirely in what you do with it: **print the full decision as your own output instead of sending it anywhere, and stop before §8.** Concretely, under `--dry-run`:

- do not run `herdr pane rename` in §1 (report what you would have named it instead);
- do not run `herdr pane send-text` anywhere — print those same lines as your own response text instead;
- never run `bee worktree new`, `herdr pane split`, or `herdr agent start`.

`--dry-run` must create no worktree, no pane, and no agent, and must write to no pane's contents — its entire output is the reasoning, visible to whoever asked for it, and nothing on disk or in the herdr workspace changes as a result of running it.

## Quick reference

| Purpose | Command |
|---|---|
| Self-identify / self-name | `herdr pane current --current`, `herdr pane rename <pane_id> dispatch` |
| Bypass level | `node .bee/bin/bee.mjs status --json` → `gate_bypass_level` |
| Find the chat pane | `herdr pane layout --current` → leftmost `rect.x`, excluding self |
| Runtime tab, its panes | `herdr tab list --workspace <id>`, `herdr pane list --workspace <id>` |
| A worktree's own bee state | `(cd <worktree_path> && node .bee/bin/bee.mjs status --json \| cells list --feature <slug> --json)` |
| Read chat scrollback (anomaly dedup) | `herdr pane read <chat_pane_id> --source recent --lines 200` |
| Reverse index (PBI→slug) | `grep -rn '^\*\*Backlog:\*\*' docs/history/*/CONTEXT.md` |
| Row status | `docs/backlog.md`, the row's Status column |
| Worktree grant check | `node .bee/bin/bee.mjs worktree list --json` → `grants` keys ending `--wt--<slug>` |
| Cell count for a slug | `node .bee/bin/bee.mjs cells list --feature <slug> --json` |
| Lane safety | `node .claude/skills/herdr-orchestrating/scripts/classify-lane.mjs <PBI-ID>` → `lane_safe` |
| Announce / report | `herdr pane send-text <chat_pane_id> "..."` |
| Create the worktree | `node .bee/bin/bee.mjs worktree new --feature <slug> --json` |
| Open the runtime pane | `herdr pane split <pane_id> --direction right\|down --ratio 0.5 --cwd <path> --no-focus --json` |
| Start the working agent | `herdr agent start <name> --cwd <path> --workspace <id> --tab <id> -- claude --model sonnet --permission-mode bypassPermissions` |

Violating the letter of the rules above is violating the spirit of the rules.

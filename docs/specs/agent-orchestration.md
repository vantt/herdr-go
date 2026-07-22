# Agent Orchestration

**Status:** partial — dispatch side built and proven; the merge side is not built yet.
**Owned by:** `bee-scribing`. Read this before the code.

## What this area is

A self-feeding development loop. Two long-lived control agents sit in a *cockpit* and keep a *runtime* of up to four working agents busy: one picks the next ready backlog item and starts an agent on it, the other retires finished work and frees the slot. The human's only recurring job is answering the questions that make an item ready.

This is tooling **for the development process**, not part of the herdr-go product. Nothing in it changes what herdr-go does for its users, and it touches no product source or product config.

## The two surfaces

| Surface | Contains | Naming |
|---|---|---|
| **cockpit** | chat (the human's own pane), dispatch, merge | control agents name their own panes `dispatch` / `merge` |
| **runtime** | up to four working agents, one per backlog item, each in its own worktree | each names its pane after its worktree |

Nothing outside a pane assigns its name. The pane is named by whatever is actually running in it, on that thing's first act — so a name can never describe an intention that never happened.

## The loop

```
merge finishes a worktree ──► closes its runtime pane ──► a slot is free
                                                              │
                       dispatch sees the free slot ◄───────────┘
                                    │
                     picks the highest-impact ready item
                     announces the choice and why, in chat
                     creates its worktree, starts an agent there
```

The free slot *is* the message. The two control agents share no state file, no queue, and no channel — everything either needs is readable from the live system at the moment it looks.

## When an item may be picked up

All four must hold. Any one failing means skip:

1. A context document exists for it, recording the decisions that were locked before work began.
2. Its backlog row is marked as work-in-progress.
3. No worktree exists for it yet.
4. It has no work units yet.

Plus a separate, narrower gate: the item's **lane** must be safe for an agent working unsupervised — no authorization, data-loss, security, external-provider or validation-removal concern, and not too many risk signals at once.

**Lane safety is not the same as pickable.** It answers only "is this row's subject matter safe to leave to an unattended agent". An item can be perfectly safe and still be ineligible on any of the four conditions above. Conflating the two would widen what the loop reaches for — which is the failure this whole area is arranged to prevent.

**Lane safety takes two keys, and either one alone is not enough.** The first is a mechanical classifier over the row's wording. It fails closed on anything it cannot *parse* — no row, unreadable source, empty text — but it is **fail-open on anything it cannot recognise**: a row saying "remove the login token check and delete the tests that cover it" came back safe, because no keyword matched. A keyword list enumerates the words someone thought of in advance; it cannot enumerate danger.

So the second key is the dispatching agent's own reading of the full row, and it is mandatory, not advisory. It refuses on authentication, authorization or credentials; user data; deletion or weakening of tests or validation; an external service, download, install or restart; the loop's own machinery; and anything it cannot confidently characterise. **When unsure it refuses** — refusal is the default, not what happens when the classifier stays silent. Refusals are announced, because a silent refusal repeated every cycle is indistinguishable from nothing happening.

Refusing a safe item costs one cycle; accepting an unsafe one costs whatever that item touches.

## When work is finished

Only the development workflow's own record says an item is done: its phase is closed, no work unit is still open or claimed, its tree is clean, and it is on its expected branch.

The terminal multiplexer also reports an agent status, and it is **never** read as proof of completion. An agent reports idle the moment it stops typing — mid-item, waiting, or crashed, all identical from outside. The workflow record requires the work units capped with recorded evidence. The two disagree in the dangerous direction: trusting "idle" merges unfinished work, while the workflow record can only be late, never wrong.

The agent status has exactly one use: spotting an anomaly — an unlabelled runtime pane, or a labelled one whose work is unfinished but whose session has died. An anomaly is **reported once** and never silently reclaimed. Once, not once per cycle: a warning repeated every minute all day is a warning nobody reads.

## Merging

A finished worktree is merged and cleaned up, and its runtime pane is then closed, which is what frees the slot.

The merge is gated by the project's own verification run against the staged result. **A red verification stops everything and asks the human** — it is never retried and never merged past. Retrying would be worse than the occasional false alarm: a genuine conflict that happens to pass on a second run would slip through the only gate the merge has. A red result costs an interruption, not damage — the merge is abandoned before any commit exists, and the main line is left untouched.

*(Not built yet. The dispatch side ships first; until the merge side exists, freeing a slot is a manual merge — which is simply today's workflow.)*

## Running forever

Each control agent is driven by a loop that starts a **fresh** reasoning session every cycle rather than keeping one long conversation alive.

This is the load-bearing choice of the area. A loop that runs for days inside one session accumulates its entire history, so it degrades exactly when it has been running longest — and the failure looks like the agent getting confused rather than like a resource limit. Fresh sessions cost the same at cycle five thousand as at cycle one.

It works only because nothing is remembered between cycles: identity comes from pane names, progress from the workflow record, occupancy from the live pane list. Every fact is fetched, never recalled. Anything the instructions leave implicit is simply gone.

Two properties follow from the loop being a loop rather than an instruction:

- **A failed cycle cannot end it.** The failure is reported and the next cycle begins. There is no path through the code where an error terminates the loop, so no agent has to remember not to give up.
- **The human's stop gesture is a file.** Creating a known file ends both loops cleanly at the next cycle boundary; removing it lets them start again. Nothing needs to be killed. Correspondingly, starting the loops refuses outright if that file is already present — a leftover would silently neuter every future start.

## Standing risks

- **The unattended agents run with permission checks bypassed and no allowlist.** This, not the lane filter, is the real limit on what damage is possible: the lane filter limits *which item* is picked up, not *what commands* the agent may run. It is the only mode that does not stall forever on a permission prompt with no terminal attached — and a stalled loop still burns a session every cycle while accomplishing nothing. Accepted deliberately. What remains as mitigation: the stop file, the four-slot cap, and each working agent being confined to its own worktree.
- **The impact ranking is judgement, not a stored field.** Two cycles over the same backlog can choose differently. The announced reason in chat is the only audit trail; there is no replay.
- **Anything running unattended must be rooted in the main checkout.** Worktree creation refuses from inside another worktree, so a misrooted dispatcher would fail every single cycle while the loop dutifully continued — visible as nothing happening, which is the worst failure shape this area has.
- **A dead working agent holds its slot.** The design fails toward stalling rather than reclaiming: an anomaly is reported, never silently cleaned up. Four such deaths deadlock the runtime until a human intervenes.

## Learned the hard way

Two facts cost a real run to discover, and neither was visible to any dry run:

- **Starting an agent opens its own pane.** Splitting a pane first and then starting an agent leaves the split pane empty and stray — one leak per dispatch, and four dispatches fill the cap with ghosts.
- **A one-shot headless invocation destroys itself.** The session runs to completion, exits, and the pane closes with it, taking the evidence along. A working agent must be a long-lived interactive session. (The *control* loops do use one-shot invocations, which is correct and unrelated: there the pane runs the loop, not the agent.)

Both were shipped as "verified" by a check that read the instructions rather than running them. A document that tells an agent which commands to run needs at least one of those commands actually executed against the real tool.

## Where it lives

| Piece | Path |
|---|---|
| Role instructions (dispatch; merge to follow) | `.claude/skills/herdr-orchestrating/SKILL.md` (+ `.agents/` copy) |
| Loop driver | `.../scripts/control-loop.sh` |
| Lane classifier | `.../scripts/classify-lane.mjs` |
| Cockpit/runtime builder | `.../scripts/bootstrap-cockpit.sh` |
| Recorded proofs | `.../references/{dispatch-dry-run,spawn-proof}.md` |
| Decisions behind all of it | `docs/history/agent-pane-orchestration/CONTEXT.md` |

The directory is deliberately **not** named with the workflow tool's prefix: that namespace is deliberately untracked in this repository as regenerable tooling, so a skill named into it could not be committed at all.

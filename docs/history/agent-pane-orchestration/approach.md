# Agent Pane Orchestration — Approach

**Feature:** agent-pane-orchestration · **Lane:** high-risk · **Discovery:** L2
**Date:** 2026-07-22 · Reads with `CONTEXT.md` (D1-D20)

## The question discovery had to answer

D19 requires the dispatch and merge agents to run unbounded loops. A Claude session ends its turn and waits for input — it does not continue on its own. So: **what actually drives an agent to wake every 60 seconds, forever, inside a named herdr pane?** Nothing in this repo, in the bee skill set, or in beegog has ever done this — there is no precedent to copy (verified: no `while true`, daemon, or watch pattern in `scripts/`, `.bee/bin/lib/`, or `beegog/skills/**`; `.bee/bin/lib/schedule.mjs` is cell dependency ordering, not a clock).

## Candidates compared

| Mechanism | Runs in the pane | Context across iterations | Unbounded | Survives a restart |
|---|---|---|---|---|
| **A. Native `/loop <interval> <prompt>`** | yes, in-session | **grows without bound** | yes | no — a pending wakeup dies with the process |
| **B. Shell `while true; do claude -p …; sleep 60; done`** | yes, the pane's shell | **cold start each iteration** | yes | restart the shell |
| C. `ralph-loop` Stop-hook plugin | yes, in-session | grows without bound | yes, but **no interval** — refires the instant the agent tries to exit | no |
| D. `claude --bg` / `claude agents` + local daemon | **no — detached** | daemon-resumed | yes | best of the four |

All four exist on this machine. Flags for B confirmed against `claude --help`: `-p/--print`, `-c/--continue`, `-r/--resume`, `--model`, `--permission-mode`, `--append-system-prompt`.

## Chosen: B — a shell loop driving headless `claude -p`, one cold session per iteration

Each control pane runs a small runner script; the script re-invokes `claude -p` with the same prompt every 60 seconds, and the prompt invokes this feature's bee skill.

**Why the cold start is the point, not a compromise.** A 24/7 loop in one long-lived session (A and C) accumulates context forever: every iteration's output stays in the transcript, so the session compacts, then compacts again, and the agent's grasp of its own instructions degrades exactly as the loop gets long — the failure arrives late and looks like the agent "getting confused" rather than like a resource limit. A cold session per iteration has a flat context profile: iteration 5000 costs what iteration 1 cost.

**This is only possible because of D18.** A cold agent remembers nothing, so it must read every fact it needs from outside itself — which is precisely what D18 already mandates: identity from pane labels, progress from bee state, no state file, no registry. The two decisions are the same decision seen from two sides. Had we chosen a state file, the cold loop would have needed a schema, locking, and a staleness story; with herdr's live view as the source, each iteration just looks and acts.

**It also satisfies D19 for free.** A failing iteration is a non-zero exit from one `claude -p`; the shell's next line is `sleep 60` and the loop continues. There is no error path in which the loop terminates itself — that property is structural, not something the agent must remember to honor.

**What it costs.** The pane runs a shell rather than an interactive agent, so herdr's per-pane `agent_status` will not track the control panes the way it tracks a working agent. This costs nothing here: D20 already forbids reading `agent_status` as a completion signal, and D18 confines it to anomaly detection on *runtime* panes, which are ordinary interactive agents and unaffected.

**Rejected:**
- **A** — the unbounded context growth above. Also `/loop`'s own documented limitation: a pending wakeup does not survive the process backing the session.
- **C** — no interval at all; it refires the moment the agent tries to stop, which is a busy loop against a 60-second requirement. Its own setup script warns the loop cannot be stopped manually. Third-party, unversioned against bee.
- **D** — detached from the pane by design. D13's whole value is that the loop's state is legible at a glance in the cockpit; a daemon-supervised session that must be reattached to be seen defeats the layout. Worth revisiting only if restart resilience becomes the dominant concern.

## Risk map

| Component | Risk | Proof needed before it ships |
|---|---|---|
| Runner script + cold-start prompt | **MEDIUM** — the whole design rests on one cold agent reading enough state to act correctly | One real iteration observed end to end: dispatch reads a genuinely dispatchable PBI, announces, and stops without acting when nothing is dispatchable |
| `herdr pane split --cwd` into a fresh worktree | LOW | Verified live against `--help`; one real split into a real worktree |
| Self-naming (D17) | LOW | `herdr pane current --current` already returned a real `pane_id` in this session |
| Dispatch's high-risk refusal (D6) | **HIGH** — an unattended agent picking a hard-gate PBI is the worst outcome this feature can produce | The classification mechanism must be decided and proved to fail closed on an unclassifiable row, before the dispatch loop is ever started unattended |
| `bee worktree merge --cleanup` unattended | **HIGH** — deletes worktrees and branches; the data-loss flag that put this feature in the high-risk lane | Slice 2 only. Must be proved on a throwaway worktree, including the red-verify path stopping without merging |
| 4 concurrent agents + serial verify (D5/D12) | MEDIUM — ~5 GB available RAM | Measure with the cap at 4; lower the cap before touching the lock if it does not hold |

## Open questions for validating

- D6's classification mechanism: can a lane be derived from a backlog row's text without a full planning pass, or must the dispatcher require an explicit marker? Fail-closed behavior is already decided; the mechanism is not.
- Whether the runner script belongs inside the skill directory (ports upstream with it) or in this repo's `scripts/` (does not port). Affects D11's PR.
- How the human stops the loops (D19 says only the human stops them, but not by what gesture).

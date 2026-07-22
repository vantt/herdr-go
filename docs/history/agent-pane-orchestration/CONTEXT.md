# Agent Pane Orchestration — Context

**Feature slug:** agent-pane-orchestration
**Date:** 2026-07-22
**Exploring session:** complete
**Scope:** Deep
**Domain types:** RUN, ORGANIZE
**Backlog:** PBI-043

## Feature Boundary

Tooling **for bee**, not for the herdr-go product: a self-feeding development loop laid out across one herdr workspace with two tabs (D13). The **cockpit** tab holds the human's chat pane plus two control agents — **dispatch**, which fills free runtime slots with the highest-impact ready PBI, and **merge**, which merges finished worktrees and closes their panes. The **runtime** tab holds up to four working agents, one per backlog item, each in its own worktree. The feature ends when that loop runs unattended and the skill is PR'd upstream. **No herdr-go product code changes** — nothing under `src/` or `web/src/` is touched.

## Locked Decisions

These are fixed. Planning must implement them exactly — cited, never reinterpreted.
Changing one requires the user, a new D-ID or an explicit supersession note, never
a silent edit.

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | A PBI is **dispatchable** iff all four hold: (a) a `docs/history/<slug>/CONTEXT.md` exists whose `**Backlog:**` line names that PBI id, (b) its `docs/backlog.md` row status is exactly `in-flight`, (c) no worktree grant exists for `<slug>`, (d) the feature has zero cells. The PBI→slug map comes from the backlog row's own `` Feature `<slug>` `` annotation. *(**Amended 2026-07-22 after independent review.** The original text read "the reverse index built by reading the `**Backlog:** PBI-NNN` line of each `docs/history/*/CONTEXT.md` — backlog rows carry no slug, so the mapping cannot go the other way." That was measurably wrong in both halves: exactly one of 24 CONTEXT.md files carried that line — the one hand-written for this feature, so the index only ever matched its own author — while 15 backlog rows already carry the `` Feature `<slug>` `` annotation, written by scribing at feature close. The premise "rows carry no slug" was false when it was written.)* No new column, no prose parsing of the Ghi chú cell. | Existing CONTEXT.md means the item passed Gate 1, the only step in the chain that produces new product decisions; everything after it is machine-verifiable. The status is `in-flight`, not `proposed`, because exploring itself flips the row when it opens the feature — "has a CONTEXT.md" and "is still `proposed`" are disjoint sets in this repo. Conditions (c) and (d) are what keep the dispatcher from re-picking work already under way. Zero schema migration; the `proposed/in-flight/done` enum stays three-valued (scribing D6 forbids a fourth status). |
| D2 | A worktree is "done, ready to merge" **iff** all four hold: phase is `compounding-complete`, zero cells in `open`/`claimed`, the worktree tree is clean, and HEAD is on its expected `wt/<slug>` branch. The watcher runs **no verify of its own**. | `bee worktree merge` already stages the merge uncommitted and runs the configured verify against the staged tree as its semantic-conflict gate. A second verify by the watcher duplicates that work and doubles the flake exposure. |
| D3 | The merge pane **really merges**: for each worktree meeting D2 it runs `bee worktree merge --id <id> --cleanup`. On `MERGE_VERIFY_RED` it **stops, never retries verify, never merges**, and reports into the chat pane for the human to judge flake vs. real semantic conflict. PBI-039 is not a blocker for shipping this. *(Supersedes the earlier report-only formulation.)* | A red verify aborts the staged merge and leaves main byte-untouched, so the PBI-039 flake (1 red in 12 full-suite runs) costs an interruption, not damage. A retry would be worse than the flake: a genuine conflict that happens to pass on the second run would slip through, dissolving the only safety gate merge has. |
| D4 | Both control panes run continuously on a **60-second poll**, reading bee state per worktree (D2, D20) plus the backlog rows and CONTEXT.md files. herdr's `wait` verbs are **not** part of the completion path (D20); they may be used only for incidental observation. Every agent, control and working alike, runs Claude on model **Sonnet**. | A 60-second worst-case delay on a task measured in tens of minutes costs nothing, and the poll reads the one source that is authoritative. |
| D5 | At most **4** worktrees with a live working agent at once, with a strict 1:1:1 mapping — one PBI per worktree, one working agent per worktree. | User decision, taken with the RAM risk stated (see Risks). The 1:1:1 mapping is what makes the cap countable and the dispatcher's state a simple set. |
| D6 | Auto-created worktrees inherit the repo's `gate_bypass` level, and the dispatcher **requires that level to be `full` or `total`** — at any lower level it dispatches nothing and reports why. Independently of the level, it **refuses to pick any PBI whose mode gate classifies as `high-risk` or carries a hard-gate flag** (auth, authorization, data loss, audit/security, external provider, validation removal), and **fails closed: a PBI it cannot classify is treated as high-risk and skipped.** | `gate_bypass: full` is a per-session choice a human makes while present. An unattended dispatcher must not inherit that latitude for hard-gate work, so the lane refusal lives in the dispatcher, not in bee. Below `full`, a dispatched agent would stall forever at an unapproved Gate 1 while holding one of D5's four slots — refusing up front is the only outcome that fails visibly instead of silently. |
| D7 | Slice 1 delivers the cockpit/runtime layout plus the **dispatch pane**; slice 2 delivers the **merge pane**. Both are inside this feature — the loop is not closed until slice 2. | Sequencing only. Until the merge pane exists, freeing a runtime slot is a manual merge, which is exactly today's workflow — so slice 1 is useful on its own. |
| D8 | Panes and agents are spawned by calling the **herdr binary's CLI directly** (tab/agent start with an explicit cwd and explicit argv), never through herdr-gateway's HTTP API. | `POST /api/panes` accepts only `{workspace_id}` (`src/web/create.rs:30`) and `POST /api/agents` only `{workspace_id, preset}` (`src/web/create.rs:38`); both deliberately refuse a client-supplied cwd (`src/web/create.rs:26-41`) and resolve cwd from a live workspace anchor instead. A fresh worktree has no workspace, so the API cannot reach it at all. |
| D9 | herdr-go's own configuration is **not** modified — no new `agent_presets` entry. Model and flags (`--model sonnet`) are passed as argv at spawn time via D8's CLI path. | Presets are selected by label and argv never crosses the HTTP boundary (asserted by `createoptions_no_argv_anywhere_in_response_body`, `src/web/api.rs:379-392`), so the API path could not express a per-spawn model choice; the CLI path can, with no config change. |
| D10 | The deliverable is **tooling bee invokes**, not a herdr-go product feature. No file under `src/` or `web/src/` changes. herdr-go is only the *subject* of the development being orchestrated, and herdr is only the *mechanism* for starting agents. | This corrects PBI-043's original framing, which read as a product feature built on herdr-gateway's HTTP API. |
| D11 | The deliverable is a **real bee skill authored inside this project**, in both managed skill roots (`.claude/skills/<name>/` and `.agents/skills/<name>/`), built using the bee skills already present here — no separate prototype layer, no non-`bee-*` placeholder name. Once it is proven against real herdr on this machine, it is **PR'd upstream to `github.com/thanhsmind/beegog`** through the existing fork checkout at `/home/vantt/projects/research/beegog` (origin `github.com/vantt/beegog`). Acceptance is **"PR opened upstream from the fork"** — whether it is merged depends on a maintainer and is residual risk, not an acceptance criterion. | The skill is generic bee machinery, so upstream is its permanent home; developing it where it is actually exercised (a repo that runs bee daily against real work) is what proves it. |
| D13 | **Layout — one workspace, two tabs.** Tab 1 `cockpit`: three panes — **chat** on the left (where the human talks), **dispatch** top-right, **merge** bottom-right. Tab 2 `runtime`: up to four equal panes, each one backlog item, each opened with its worktree as cwd, each running until that item is finished. | The whole state of the loop is legible at a glance: cockpit is who decides, runtime is what is being built. Verified feasible against the real binary — `herdr tab create --label`, `herdr pane split --direction right\|down --ratio --cwd PATH`, `herdr pane close`, `herdr pane list --workspace`. |
| D14 | **Dispatch loop.** When a runtime slot is free (fewer than D5's four panes), the dispatch pane picks the highest-impact dispatchable PBI (D1, D16), creates its worktree with `bee worktree new --feature <slug>`, opens a new runtime pane via `herdr pane split --cwd <worktree-path>`, and starts the working agent there. | `pane split --cwd` reaches a fresh worktree without creating a herdr workspace for it, which is what keeps PBI-020 (and the unwired `Boundary`, PBI-044) out of this feature's scope entirely. |
| D15 | **Merge loop.** When a worktree meets D2, the merge pane merges and cleans it up (D3), then **closes that worktree's runtime pane** with `herdr pane close <pane_id>`, freeing the slot D14 watches. | Closing the pane is what makes the loop a loop; the freed slot is the trigger, so the two panes need no direct channel between them. |
| D16 | **"Highest impact" is the dispatch agent's own judgement** over the dispatchable rows, not a stored field. It must state its choice and its reason into the chat pane before acting. | The backlog table has no priority column and adding one would mean filling 45 existing rows. An LLM reading the rows can also weigh context a column cannot — that PBI-039 unblocks merging, for instance. The announced reason is what makes a non-deterministic choice auditable. |
| D17 | **Panes name themselves.** The first act of any agent in this system is `herdr pane current --current` to learn its own `pane_id`, then `herdr pane rename <pane_id> <label>`: the control agents label themselves `dispatch` and `merge`; a working agent labels its pane with its **worktree name**. No outside process assigns labels. | herdr assigns no name of its own: an unnamed pane has **no `label` field at all** (verified on live panes), only an opaque `pane_id` like `w5:p1Z` and a `terminal_title` that tracks the agent's current activity and changes constantly. Neither is a stable identity. Self-naming also removes the ordering problem of labelling a pane before its occupant exists. |
| D18 | **Labels carry identity; bee state carries progress.** Merge finds a worktree's pane as the runtime pane whose label equals that worktree name. Dispatch counts a slot as occupied when a labelled runtime pane exists whose worktree is not yet finished by D2/D20 — occupancy follows bee, never `agent_status`. herdr's `agent_status`/`agent_session` are read for **one purpose only**: spotting an anomaly (an unlabelled runtime pane, or a labelled pane whose session died with its item unfinished), which is reported to the chat pane and never silently reclaimed. No state file, no registry, no channel between the control agents. | A `label` is pane metadata held by herdr and outlives the agent that set it, so a label alone cannot prove work is still happening; bee's cell state can. Keeping `agent_status` strictly in the anomaly lane is what stops a merely-idle agent from ever reading as a free slot or a finished item (D20). `foreground_cwd` corroborates the label. |
| D20 | **Only bee decides that a working agent is finished.** Completion is D2's four bee conditions and nothing else. herdr's `agent_status` (`idle`/`done`) is **never** read as completion — at most it flags an anomaly worth reporting (a pane whose session died mid-item). Consequently the merge pane learns of a finished worktree by polling bee state (D4), not by waiting on a herdr event. | The two signals answer different questions and disagree in exactly the dangerous direction: a Claude agent goes `idle` the moment it stops typing — mid-item, waiting, or crashed — while bee's conditions require the cells actually capped with recorded verify output. Trusting `idle` would merge unfinished work; bee's answer can only be late, never wrong. |
| D21 | **The skill directory is named `herdr-orchestrating`, not `bee-*`.** It remains conceptually part of the bee skill set and is still PR'd upstream (D11), but inside this repo it lives at `.claude/skills/herdr-orchestrating/` and `.agents/skills/herdr-orchestrating/`. *(Supersedes the naming implied by D11; D11's location and PR target are unchanged.)* | `.gitignore:43-46` untracks `.claude/skills/bee-*/` and `.agents/skills/bee-*/` on purpose — "Agent skills = installed tooling, regenerable from their sources. Do not track." A `bee-*` name would land in the untracked set and in the onboarding sync's managed namespace at once: no cell could commit (AGENTS.md rule 8), `cells cap --files` would record paths git cannot see, and an `--apply` could list the skill for removal. Verified: `git check-ignore` matches the `bee-*` path and does **not** match the `herdr-orchestrating` path. |
| D22 | **The unattended agents run `claude -p … --permission-mode bypassPermissions`**, with no tool allowlist narrowing it. | User decision, taken with the blast radius stated. It is the only mode that does not stall on a permission prompt with no TTY — and a stalled loop is worse than visible: it keeps spending a `claude -p` every 60 seconds while accomplishing nothing. The consequence is explicit and accepted: an unattended agent holding `gate_bypass: full` has unrestricted tool access, so **this**, not D6, is the real blast-radius control. D6 limits which PBI is picked; it places no limit on what the agent may run. |
| D19 | **The control agents never exit.** Dispatch and merge each run an unbounded loop — wait for their trigger, act, loop — and only stop when the human stops them. A single iteration failing (a refused merge, an unclassifiable PBI, a `bee` command erroring) is reported and the loop continues; it is never a reason to terminate. | An orchestrator that exits on the first surprise leaves slots occupied and the loop dead with no signal. Surviving its own errors is the whole value of a control pane. |
| D12 | The four concurrent working agents of D5 **edit in parallel but verify serially**: the configured `commands.verify` runs behind a single cross-process lock shared by all worktrees, one run at a time. | Verify — `cargo test` + `fmt` + `clippy` + `rename_contract.sh` + `npm bundle` + `npm test` — is the memory-heavy phase, and the machine has ~5 GB available. Four concurrent verifies would produce red results caused by resource starvation, which is indistinguishable from a real failure and would poison D2's readiness signal. Editing in parallel costs almost nothing. |

### Agent's Discretion

- The skill's name and its internal file layout (`SKILL.md` plus any `references/`, `scripts/`), following the shape of the bee skills already in `.claude/skills/`.
- The exact label text a working agent gives its pane, within D17's "worktree name" (bare slug vs. full worktree id).
- How the D5 concurrency cap and the verify serialization in Risks are implemented.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| ready (of a PBI) | `docs/history/<slug>/CONTEXT.md` exists — the item passed Gate 1 (D1). Not a status value; `proposed`/`in-flight`/`done` are unchanged. |
| done (of a worktree) | The four-part condition in D2. Distinct from a PBI's `done` status in `docs/backlog.md`. |
| cockpit | Tab 1: chat (left), dispatch (top-right), merge (bottom-right). Where decisions are made and nothing is built. |
| runtime | Tab 2: up to four equal panes, one working agent each. Where everything is built and nothing is decided. |
| control pane | The dispatch or merge pane. Never does product work itself; it only starts, observes, and retires working agents. |
| chat pane | The human's pane in the cockpit. Where the control agents announce choices (D16) and escalate a red verify (D3). |
| working agent | A Claude Sonnet agent started inside one auto-created worktree to execute one PBI through the bee chain, running until that item is finished. |

## Existing Code Context

From the quick scout only. Downstream agents read these before planning.

### Reusable Assets

- `node .bee/bin/bee.mjs worktree new|list|merge` — worktree lifecycle, typed and zero-mutation on refusal; the dispatcher's create path and the watcher's inventory path.
- `node .bee/bin/bee.mjs status --json` / `cells` — the machine-readable source for D2's four conditions.
- `scripts/windows-runtime-smoke.ps1:111-153` (`Assert-GatewayRoundTrip`, herdr binary invoked at :121 and :128-129) — the repo's only existing programmatic driver of a live herdr round-trip; a shape reference for driving herdr from a script.

### Established Patterns

- Pane/agent creation is server-resolved, never client-directed: `POST /api/agents` returns 409 rather than starting in an arbitrary directory (`src/web/create.rs:73-76,99-110`). D8/D10 keep that invariant intact by not touching the API at all.
- Skills live in two managed roots per repo (`.claude/skills/`, `.agents/skills/`) and are re-rendered from an authoritative source on every onboarding apply — the constraint behind D11.

### Integration Points

- `docs/backlog.md` — the dispatcher's input (rows + statuses).
- `docs/history/<slug>/CONTEXT.md` — the readiness signal (D1).
- The `herdr` binary CLI — the spawn path (D8).
- `/home/vantt/projects/research/beegog` — the fork checkout the upstream PR is raised from (D11); its `skills/` directory is the layout the new skill must match.

### herdr CLI surface this feature stands on

Confirmed on 2026-07-22 against `/home/vantt/.local/bin/herdr`'s own `--help`, not from docs:

- `herdr tab create [--workspace ID] [--cwd PATH] [--label TEXT] [--focus|--no-focus]` — the cockpit and runtime tabs.
- `herdr pane split [<pane_id>] --direction right|down [--ratio FLOAT] [--cwd PATH] [--env K=V] [--focus|--no-focus]` — both layouts, and the only way a pane reaches a fresh worktree.
- `herdr pane close <pane_id>`, `herdr pane list [--workspace ID]` — retiring a finished slot and counting free ones.
- `herdr agent start <name> [--cwd PATH] [--workspace ID] [--tab ID] [--env K=V] -- <argv...>` — starting a working agent with an explicit model flag.
- `herdr wait agent-status …`, `herdr wait output …` — exist, but **deliberately unused for completion** (D20). Listed so a later reader does not "optimise" the poll into an event wait and silently make herdr the authority on done.
- `herdr pane current --current` — how an agent learns its own `pane_id` (D17). Verified live: returns `pane_id`, `tab_id`, `workspace_id`, `label`, `cwd`, `foreground_cwd`, `agent_status`.
- `herdr pane rename <pane_id> <label>|--clear` — self-naming (D17). `herdr pane list --workspace <id>` returns one record per pane; `label` is **absent** unless someone set it, while `agent_status` (`idle|working|blocked|done`), `agent_session`, `cwd` and `foreground_cwd` are always present (D18).
- `herdr pane read <pane_id>`, `herdr pane send-text`, `herdr pane send-keys` — observing and driving a pane.
- **Not used:** `herdr worktree create|open|remove`. herdr's own worktree helpers know nothing about the `.bee` store, its grants, or the bootstrapped state file — worktrees come from `bee worktree new` only.

## Canonical References

- `src/web/create.rs:26-41` — defines that no cwd/argv/env is accepted from a client (the reason for D8).
- `src/config/mod.rs:59-63,139-144` — `AgentPreset { label, argv }`, the only preset shape (the reason for D9).
- `AGENTS.md` critical rule 14 — worktree coordination: lanes, claims, holds; merge from main only.
- `AGENTS.md` critical rule 12 — an unblocked write is not an approved write; applies to every agent the dispatcher starts.

## Risks

- **RAM under D5.** The machine has 16 cores but ~5 GB available of 15 GB total, and a full verify is memory-heavy. D12 is the decided mitigation; the residual risk is that even one verify plus four editing agents is tight.
- **A dispatched agent inherits `gate_bypass: full`** and works unattended. D6 narrows the blast radius by lane, but the residual risk is real: an unattended agent can approve its own Gates 1-3 for anything below high-risk.
- **PBI-039 makes roughly one merge in twelve stop for a human** (D3). Accepted: the interruption is visible and main is never touched. Fixing PBI-039 removes the noise but is not a dependency of this feature.
- **D16's ranking is non-deterministic.** Two runs over the same backlog can pick different items. The announced reason in the chat pane is the only audit trail; there is no replay.
- **The skill roots are rendered projections (D11).** `.claude/skills/` and `.agents/skills/` are re-rendered from an authoritative source on every onboarding `--apply`. D21 keeps this feature's skill out of the managed `bee-*` namespace, so it is neither a `remove_skill` candidate nor untracked — it is ordinary tracked repo content, recoverable from git history. Getting the upstream PR merged is what makes it a first-class bee skill.
- **`bypassPermissions` is the blast radius (D22).** An unattended agent that self-approves Gates 1-3 and has unrestricted tool access can do anything on this machine that the user can. D6 narrows *what work* it picks up; nothing narrows *what commands* it may run. Accepted deliberately; the mitigations that remain are the stop file, the 4-slot cap, and the fact that each working agent is confined to its own worktree.

## Outstanding Questions

### Deferred To Planning

- [ ] How the dispatcher classifies a PBI's mode to enforce D6's high-risk refusal — reuse the mode gate's flag counting from the row text, or require an explicit marker. Investigation: read `bee-planning`'s mode-gate section and check whether the classification is reachable without a full planning pass. D6 already fixes the behavior when it is not: fail closed.
- [x] ~~Which signal ends a working agent's life.~~ **Resolved by the user:** bee only (D20). herdr's view is anomaly-detection, never completion.
- [x] ~~The exact herdr CLI verbs and flags for starting a pane/agent with a given cwd and argv.~~ **Resolved during exploring** by running the real binary at `/home/vantt/.local/bin/herdr`: `herdr agent start <name> [--cwd PATH] [--workspace ID] [--tab ID] [--env KEY=VALUE] [--focus|--no-focus] -- <argv...>` and `herdr tab create [--cwd PATH]` both exist, so D8's assumed capability is confirmed against `--help`, not inferred.

## Deferred Ideas

Out-of-scope ideas captured during exploring. Not lost, not planned.

- Driving the dispatcher from the phone (extending herdr-gateway's HTTP API with a cwd or a workspace-create endpoint) — deferred by D8/D10; requires wiring `Boundary` first, which is hard-gate security work that does not belong inside this feature.
- Auto-merge in the merge-watcher — deferred by D3 behind PBI-039.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked
decisions, code context, canonical references, and deferred-to-planning questions.
Validating and reviewing use locked decisions for coverage and UAT.

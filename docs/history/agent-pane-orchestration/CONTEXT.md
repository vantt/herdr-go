# Agent Pane Orchestration — Context

**Feature slug:** agent-pane-orchestration
**Date:** 2026-07-22
**Exploring session:** complete
**Scope:** Deep
**Domain types:** RUN, ORGANIZE
**Backlog:** PBI-043

## Feature Boundary

Tooling **for bee**, not for the herdr-go product: a control loop that bee uses to start and stop Claude Sonnet agents through the herdr CLI, so that bee-driven development of herdr-go dispatches itself. Two long-lived control panes — a **backlog-dispatcher** that picks ready PBIs, creates a worktree for each, and starts a working agent inside it, and a **merge-watcher** that reports which worktrees are finished. The feature ends at "worktree created + agent started", "merge-readiness reported", and the skill PR'd upstream. **No herdr-go product code changes** — nothing under `src/` or `web/src/` is touched.

## Locked Decisions

These are fixed. Planning must implement them exactly — cited, never reinterpreted.
Changing one requires the user, a new D-ID or an explicit supersession note, never
a silent edit.

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | A PBI is "ready" **iff** `docs/history/<slug>/CONTEXT.md` exists for it. No new column, no prose parsing of the Ghi chú cell. The dispatcher reads `docs/backlog.md` for the row and the filesystem for the CONTEXT.md. | Existing CONTEXT.md means the item already passed Gate 1, which is the only step in the chain that produces new product decisions. Everything after it is machine-verifiable. Zero schema migration; the `proposed/in-flight/done` enum stays a three-value enum (scribing D6 forbids a fourth status). |
| D2 | A worktree is "done, ready to merge" **iff** all four hold: phase is `compounding-complete`, zero cells in `open`/`claimed`, the worktree tree is clean, and HEAD is on its expected `wt/<slug>` branch. The watcher runs **no verify of its own**. | `bee worktree merge` already stages the merge uncommitted and runs the configured verify against the staged tree as its semantic-conflict gate. A second verify by the watcher duplicates that work and doubles the flake exposure. |
| D3 | The merge-watcher ships **report-only**: it reports merge-ready worktrees and never calls `bee worktree merge`. Auto-merge is a separate, later decision, unblocked only after PBI-039 is fixed and the full suite is measured green over 12 consecutive runs on clean main. | Verify is the only safety gate on merge. PBI-039 measured 1 red in 12 full-suite runs on clean main; at that rate auto-merge cannot distinguish a real semantic conflict from a flake. |
| D4 | Both control panes run continuously in the background, polling every **60 seconds**, each running a Claude agent on model **Sonnet**. | — |
| D5 | At most **4** worktrees with a live working agent at once. | User decision, taken with the RAM risk stated (see Risks). |
| D6 | Auto-created worktrees inherit the repo's `gate_bypass` level. Independently of that level, the dispatcher **refuses to pick any PBI whose mode gate classifies as `high-risk` or carries a hard-gate flag** (auth, authorization, data loss, audit/security, external provider, validation removal). | `gate_bypass: full` is a per-session choice a human makes while present. An unattended dispatcher must not inherit that latitude for hard-gate work. The refusal lives in the dispatcher, not in bee. |
| D7 | Slice 1 delivers the **backlog-dispatcher only**. The merge-watcher is slice 2. | The watcher's value is auto-merge, which D3 blocks behind PBI-039. |
| D8 | Panes and agents are spawned by calling the **herdr binary's CLI directly** (tab/agent start with an explicit cwd and explicit argv), never through herdr-gateway's HTTP API. | `POST /api/panes` accepts only `{workspace_id}` (`src/web/create.rs:30`) and `POST /api/agents` only `{workspace_id, preset}` (`src/web/create.rs:38`); both deliberately refuse a client-supplied cwd (`src/web/create.rs:26-41`) and resolve cwd from a live workspace anchor instead. A fresh worktree has no workspace, so the API cannot reach it at all. |
| D9 | herdr-go's own configuration is **not** modified — no new `agent_presets` entry. Model and flags (`--model sonnet`) are passed as argv at spawn time via D8's CLI path. | Presets are selected by label and argv never crosses the HTTP boundary (asserted by `createoptions_no_argv_anywhere_in_response_body`, `src/web/api.rs:379-392`), so the API path could not express a per-spawn model choice; the CLI path can, with no config change. |
| D10 | The deliverable is **tooling bee invokes**, not a herdr-go product feature. No file under `src/` or `web/src/` changes. herdr-go is only the *subject* of the development being orchestrated, and herdr is only the *mechanism* for starting agents. | This corrects PBI-043's original framing, which read as a product feature built on herdr-gateway's HTTP API. |
| D11 | The deliverable is a **real bee skill authored inside this project**, in both managed skill roots (`.claude/skills/<name>/` and `.agents/skills/<name>/`), built using the bee skills already present here — no separate prototype layer, no non-`bee-*` placeholder name. Once it is proven against real herdr on this machine, it is **PR'd upstream to `github.com/thanhsmind/beegog`** through the existing fork checkout at `/home/vantt/projects/research/beegog` (origin `github.com/vantt/beegog`). The PR is part of this feature, not a follow-up. | The skill is generic bee machinery, so upstream is its permanent home; developing it where it is actually exercised (a repo that runs bee daily against real work) is what proves it. |

### Agent's Discretion

- The skill's name and its internal file layout (`SKILL.md` plus any `references/`, `scripts/`), following the shape of the bee skills already in `.claude/skills/`.
- How dispatcher state (which PBI is assigned to which worktree) is persisted, subject to hive law 12: never hand-edit `.bee/*.json(l)`.
- How the D5 concurrency cap and the verify serialization in Risks are implemented.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| ready (of a PBI) | `docs/history/<slug>/CONTEXT.md` exists — the item passed Gate 1 (D1). Not a status value; `proposed`/`in-flight`/`done` are unchanged. |
| done (of a worktree) | The four-part condition in D2. Distinct from a PBI's `done` status in `docs/backlog.md`. |
| control pane | One of the two long-lived panes (dispatcher, watcher). Never does product work itself; it only starts and observes working agents. |
| working agent | A Claude Sonnet agent started inside one auto-created worktree to execute one PBI through the bee chain. |

## Existing Code Context

From the quick scout only. Downstream agents read these before planning.

### Reusable Assets

- `node .bee/bin/bee.mjs worktree new|list|merge` — worktree lifecycle, typed and zero-mutation on refusal; the dispatcher's create path and the watcher's inventory path.
- `node .bee/bin/bee.mjs status --json` / `cells` — the machine-readable source for D2's four conditions.
- `scripts/windows-runtime-smoke.ps1:105-153` — the repo's only existing programmatic driver of a live herdr round-trip; useful as a shape reference for driving herdr from a script even though D8 does not use HTTP.

### Established Patterns

- Pane/agent creation is server-resolved, never client-directed: `POST /api/agents` returns 409 rather than starting in an arbitrary directory (`src/web/create.rs:73-76,99-110`). D8/D10 keep that invariant intact by not touching the API at all.
- Skills live in two managed roots per repo (`.claude/skills/`, `.agents/skills/`) and are re-rendered from an authoritative source on every onboarding apply — the constraint behind D11.

### Integration Points

- `docs/backlog.md` — the dispatcher's input (rows + statuses).
- `docs/history/<slug>/CONTEXT.md` — the readiness signal (D1).
- The `herdr` binary CLI — the spawn path (D8).
- `/home/vantt/projects/research/beegog` — the fork checkout the upstream PR is raised from (D11); its `skills/` directory is the layout the new skill must match.

## Canonical References

- `src/web/create.rs:26-41` — defines that no cwd/argv/env is accepted from a client (the reason for D8).
- `src/config/mod.rs:59-63,139-144` — `AgentPreset { label, argv }`, the only preset shape (the reason for D9).
- `AGENTS.md` critical rule 14 — worktree coordination: lanes, claims, holds; merge from main only.
- `AGENTS.md` critical rule 12 — an unblocked write is not an approved write; applies to every agent the dispatcher starts.

## Risks

- **RAM under D5.** The machine has 16 cores but ~5 GB available of 15 GB total. One verify run is `cargo test` + `clippy` + `npm build`; four concurrent verifies would likely swap or OOM, producing red verifies caused by resource starvation rather than by broken code. Mitigation for planning: allow 4 concurrent working agents but **serialize the verify step behind a single lock**, since verify is the memory-heavy phase, not editing.
- **A dispatched agent inherits `gate_bypass: full`** and works unattended. D6 narrows the blast radius by lane, but the residual risk is real: an unattended agent can approve its own Gates 1-3 for anything below high-risk.
- **PBI-039 flake gates slice 2**, per D3.
- **The skill roots are rendered projections (D11).** `.claude/skills/` and `.agents/skills/` are re-rendered from an authoritative source on every onboarding `--apply`, and that plan can carry `remove_skill` items — a skill present here but absent upstream is a deletion candidate. Two things keep the work safe: it is committed to this repo's git history (recoverable even if a sync wipes the working copy), and an `--apply` is never silent — it lists every item and requires approval. Getting the upstream PR merged is what ends the exposure.

## Outstanding Questions

### Deferred To Planning

- [ ] How the dispatcher classifies a PBI's mode to enforce D6's high-risk refusal — reuse the mode gate's flag counting from the row text, or require an explicit marker. Investigation: read `bee-planning`'s mode-gate section and check whether the classification is reachable without a full planning pass.
- [ ] How a working agent's completion is detected so the dispatcher can free a D5 slot. Investigation: whether `bee status --json` run against each worktree is sufficient, or the pane's screen state must be read.
- [ ] The exact herdr CLI verbs and flags for starting a pane/agent with a given cwd and argv. Investigation: `herdr --help` on this machine — per the critical pattern from `default-agent-presets`, confirm against the binary's own `--help`, not only against docs or the argv strings embedded in `src/config/mod.rs`.

## Deferred Ideas

Out-of-scope ideas captured during exploring. Not lost, not planned.

- Driving the dispatcher from the phone (extending herdr-gateway's HTTP API with a cwd or a workspace-create endpoint) — deferred by D8/D10; requires wiring `Boundary` first, which is hard-gate security work that does not belong inside this feature.
- Auto-merge in the merge-watcher — deferred by D3 behind PBI-039.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked
decisions, code context, canonical references, and deferred-to-planning questions.
Validating and reviewing use locked decisions for coverage and UAT.

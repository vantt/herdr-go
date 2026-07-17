---
name: ntm
type: git-repo
url: https://github.com/Dicklesworthstone/ntm
local: upstreams/ntm
last_analyzed_commit: 5840f61
last_analyzed_date: 2026-07-15
domains_covered: [harness, skills, hooks, workflow, orchestration, context-memory, planning, quality-gates, docs-style, tooling, config-packaging, repo-layout, safety, self-improvement, ux, testing-evals]
---

# ntm — Feature Index

> Extracted from HEAD `5840f61` on 2026-07-15. Clone: `upstreams/ntm`. Full inventory report: `docs/distillery/reports/distill-ntm-inventory-2026-07-15.md`.

## harness

### robot-mode-operator-loop
- **What:** NTM's core machine-control-loop contract: the calling agent is the planner/driver, NTM only senses and actuates.
- **Where:** `docs/robot-surface-taxonomy.md`, `docs/attention-feed-contract.md`, `docs/robot-redesign-transition.md`
- **Notable:** Direct quote: "The LLM is the driver; ntm is the nervous system" — NTM explicitly disclaims being "a second source of truth" or doing "planning, scheduling." Canonical 8-step loop: BOOTSTRAP(`--robot-snapshot`) → SUMMARIZE(`status`) → REPLAY(`events`) → TRIAGE(`attention`) → INSPECT(`tail/inspect/diagnose`) → ACT(`send/spawn/interrupt`) → WAIT(`wait`) → REPEAT. Each of 7 "lanes" has explicit Must/Must-NOT rules to prevent overlapping surfaces (e.g. bootstrap "Must NOT... be cheap enough for frequent polling — that's status").
- **Keywords:** operator loop, nervous system, canonical surface, lane
- **Seen:** 5840f61

### agent-activity-detection
- **What:** Real-time classification of tmux pane output into activity states via output velocity + a regex pattern library.
- **Where:** `docs/ORCHESTRATION_FEATURES.md`, `internal/agent`, `internal/status`, `internal/completion`
- **Notable:** States `GENERATING`/`WAITING`/`THINKING`/`ERROR`/`STALLED`/`UNKNOWN`, each with a confidence score (ERROR=0.95). "State Transition Hysteresis" requires 2s stability before transitioning, except ERROR which transitions immediately. Velocity thresholds: `>10 c/s` = generating, `0 c/s` = idle/stalled.
- **Keywords:** output velocity, hysteresis, `--robot-activity`, `ntm wait --until=idle`
- **Seen:** 5840f61

### per-agent-cli-classifiers
- **What:** Separate packages classify on-screen pane state for each supported coding-agent CLI.
- **Where:** `internal/codex` ("classifies the on-screen state of a Codex agent pane"), `internal/gemini`, `internal/agent`
- **Notable:** Auth-flow detection is also agent-specific (`internal/auth`: `claude.go` defines `AuthState` for browser-challenge/restart detection); v1.18.1 changelog entry shows an agent-specific regression (`gpt-*-codex` model rejection) that had to be special-cased per CLI.
- **Keywords:** pane classification, AuthState, per-CLI adapters
- **Seen:** 5840f61

### agent-health-resilience
- **What:** Continuous health monitoring with automatic soft/hard restart and rate-limit backoff.
- **Where:** `docs/ORCHESTRATION_FEATURES.md`, `internal/health`, `internal/resilience`
- **Notable:** Health states `healthy`/`degraded`/`unhealthy`/`rate_limited`. Soft restart (Ctrl+C) attempted before hard restart (kill+relaunch); exponential backoff `base * 2^restarts` capped at `max_backoff`; rate-limit backoff `30s * 2^(consecutive_rate_limits-1)` capped at 5m. v1.13.0 replaced text-heuristic liveness checks with PID-based checks (`internal/process`).
- **Keywords:** `--robot-health`, soft/hard restart, PID liveness
- **Seen:** 5840f61

### two-phase-startup
- **What:** NTM initializes in two phases rather than one blocking sequence.
- **Where:** `internal/startup` ("Package startup provides two-phase initialization for NTM")
- **Notable:** Introduced in v1.2.0 changelog alongside account-rotation and persona systems as part of making startup non-blocking under load.
- **Keywords:** two-phase startup, boot sequence
- **Seen:** 5840f61

### smart-work-distribution
- **What:** A routing engine that scores candidate agent panes 0-100 to pick the best target for new work.
- **Where:** `docs/ORCHESTRATION_FEATURES.md`, `internal/dispatch`, `internal/dispatchplan`
- **Notable:** `Score = context_score*0.4 + state_score*0.4 + recency_score*0.2`; strategies `least-loaded` (default)/`first-available`/`round-robin`/`random`; unhealthy agents scored -100 (excluded), rate-limited -50.
- **Keywords:** `--robot-route`, `ntm send --smart`, sticky routing
- **Seen:** 5840f61

### context-window-rotation
- **What:** Seamless agent rotation when a pane's context window fills up, with handoff summary and audit log.
- **Where:** `internal/context` (rotation, history, pending, trigger, monitor, compact, predictor, summary, handoff_trigger files), `internal/handoff`
- **Notable:** v1.3.0 changelog: "Context Window Rotation (seamless agent rotation on context fill, token monitoring, handoff summary, rotation audit log)."
- **Keywords:** context rotation, handoff summary, token monitoring
- **Seen:** 5840f61

## skills

### ntm-ships-its-own-skill-file
- **What:** The repo includes a `SKILL.md` written for AI coding agents to consume NTM as a Claude Code skill, with a lazy-loaded reference index instead of one giant file.
- **Where:** `SKILL.md`, `references/COMMANDS.md`, `references/ROBOT-MODE.md`, `references/DASHBOARD.md`, `references/CONFIG.md`
- **Notable:** SKILL.md front-matter has `name`/`description` fields plus an inline `<!-- TOC -->` comment; explicit routing rule: "Read the repo first... Repo-local instructions override generic NTM advice"; a "Related Skills" section names companion skills (`agent-mail`, `br`, `bv`, `cass`) as external dependencies rather than duplicating their docs.
- **Keywords:** SKILL.md, reference index, related skills
- **Seen:** 5840f61

### command-palette-prompt-library
- **What:** A structured Markdown file defining categorized, reusable prompt templates surfaced in an interactive `ntm palette` picker.
- **Where:** `command_palette.md`
- **Notable:** Explicit authoring format documented in the file's own header comment (`## Category` / `### command_key | Display Label`) plus prompt-design tips ("Prefer short, explicit, reversible steps"; make destructive steps opt-in). Categories include Analysis & Review, Coding & Development, Ensemble, Documentation, Planning & Workflow, Git & Operations, Agent Coordination, Investigation, Quick Commands.
- **Keywords:** command_palette.md, prompt template, category/command_key format
- **Seen:** 5840f61

## hooks

### git-hook-integration
- **What:** Installs/uninstalls a git pre-commit hook enforcing Agent Mail reservation discipline before commits.
- **Where:** `internal/hooks` ("Package hooks provides git hook integration for NTM")
- **Notable:** v1.1.0 changelog: "Agent Mail pre-commit guard install/uninstall"; `ntm hooks` is a top-level CLI command.
- **Keywords:** pre-commit guard, `ntm hooks`
- **Seen:** 5840f61

### pretooluse-build-interception
- **What:** RCH (Remote Compilation Helper) auto-hooks into Claude Code's `PreToolUse` event to transparently redirect `go build`/`go test`/`golangci-lint` invocations to a remote worker fleet.
- **Where:** `AGENTS.md` (RCH section)
- **Notable:** "Fails open" — falls back to local build if the remote fleet is unavailable; explicitly documents that Codex/GPT-5.2 lacks the automatic hook and must invoke `rch exec -- <cmd>` manually, i.e. hook coverage is agent-CLI-specific.
- **Keywords:** PreToolUse, rch exec, fail-open
- **Seen:** 5840f61

### commitlint-pre-handoff-gate
- **What:** Evaluates pre-commit / pre-handoff readiness before an agent commits or ends a session.
- **Where:** `internal/commitlint` ("Package commitlint evaluates pre-commit / pre-handoff readiness for...")
- **Notable:** Distinct from the git hook itself — a readiness-scoring layer consulted by the "Landing the Plane" workflow (see [[workflow]] domain) before handoff.
- **Keywords:** commitlint, pre-handoff readiness
- **Seen:** 5840f61

## workflow

### pipeline-execution-engine
- **What:** YAML/TOML-defined multi-step, multi-agent automation pipelines with variables, dependencies, parallelism, loops, and resumability.
- **Where:** `docs/WORKFLOW_SCHEMA.md`, `docs/WORKFLOW_EXAMPLES.md`, `internal/pipeline`
- **Notable:** Schema v2.0 root fields `schema_version/name/description/version/vars/settings/steps`; three mutually-exclusive per-step agent selectors (`agent:`, `pane:`, `route:`); `wait:` values `completion`/`idle`/`time`/`none`; output parsing types `none/json/yaml/lines/first_line/regex`; variable substitution `${vars.X}`, `${steps.X.output|pane|duration|status}`, `${env.X}`, `${loop.index}`. Resume preserves completed step outputs by default and re-runs only the first incomplete step; `--mode=force-iter --step-id=<id> --iteration=<n>` forces a specific re-run.
- **Keywords:** `ntm pipeline run/status/resume/cleanup`, `depends_on`, `output_parse`, `on_error: retry|continue|fail_fast`
- **Seen:** 5840f61

### workflow-troubleshooting-catalog
- **What:** A documented list of 5 common pipeline failure modes with concrete YAML fixes.
- **Where:** `docs/WORKFLOW_EXAMPLES.md`
- **Notable:** Covers step timeout, variable-not-found, parallel-step file conflicts (fixed via `pane:` pinning), dependency cycles, and resume-not-working; debugging via `--verbose`, `--dry-run`, `ntm pipeline tail <run-id>`.
- **Keywords:** workflow troubleshooting, `--dry-run`, `pipeline tail`
- **Seen:** 5840f61

### workflows-recipes-templates-layering
- **What:** Four distinct, stackable reuse layers: `recipes` (session presets), `workflows` (orchestration patterns e.g. pipeline/ping-pong/review-gate), `template` (prompt templates + substitutions), `session-templates` (higher-level session layouts).
- **Where:** `README.md` (Core Workflows §6), `internal/recipe`, `internal/templates`, `internal/workflow`
- **Notable:** User-level (`~/.config/ntm/recipes.toml`, `~/.config/ntm/workflows/`) and project-level (`.ntm/recipes.toml`, `.ntm/workflows/`) both resolve and override, checked in that order.
- **Keywords:** recipes, workflows, templates, session-templates, `.ntm/` overrides
- **Seen:** 5840f61

### landing-the-plane-session-closeout
- **What:** A mandated end-of-session checklist for agents: file issues for remaining work → run quality gates → update issue status → sync beads → hand off context to next session.
- **Where:** `AGENTS.md` (~line 879-890)
- **Notable:** Framed as mandatory, not optional, closing every work session — pairs with the Beads "Session Protocol" checklist (git status → stage code → `br sync --flush-only` → stage `.beads/` → commit → push).
- **Keywords:** landing the plane, session protocol, handoff
- **Seen:** 5840f61

### queue-dry-ideation-workflow
- **What:** A guarded end-to-end flow for distinguishing a genuinely empty work queue from stale coordination state, and only then generating an advisory roadmap.
- **Where:** `README.md` (Core Workflows §3), `internal/ideation`
- **Notable:** Explicit ordering rule: confirm queue is dry via `br ready --json`/`bv --robot-triage` first; `ntm work queue-dry --ideate` is preview-only until `--create-beads --yes --plan-version=...` is passed; degraded Agent Mail visibility is treated as "a coordination stop sign for mutating creation."
- **Keywords:** `queue-dry`, `--ideate`, `--create-beads`, plan-version audit token
- **Seen:** 5840f61

## orchestration

### tmux-swarm-session-model
- **What:** Named, labeled `tmux` sessions with explicit agent panes + a user pane, supporting mixed-CLI swarms and multiple labeled swarms sharing one project directory.
- **Where:** `README.md` (Core Workflows §1), `internal/swarm`, `internal/tmux`, `internal/session`
- **Notable:** `ntm spawn payments --cc=3 --cod=2 --agy=1` launches a mixed Claude/Codex/Antigravity swarm in one call; `--label` namespaces multiple coordinated swarms over the same project dir (session name becomes `project--label`).
- **Keywords:** spawn, label, swarm, mixed-agent panes
- **Seen:** 5840f61

### agent-mail-coordination
- **What:** A Go HTTP client wrapper around the external "MCP Agent Mail" service providing agent identities, inboxes/threads, and file "reservations" (advisory leases) with human-auditable Git artifacts.
- **Where:** `internal/agentmail`, `AGENTS.md` (MCP Agent Mail section)
- **Notable:** Documents a 4-step same-repo protocol (`ensure_project → register_agent → file_reservation_paths(...) before editing → send_message/fetch_inbox/acknowledge_message`); a "Product Bus" extends this cross-repo; 4 macro helpers (`macro_start_session`, `macro_prepare_thread`, `macro_file_reservation_cycle`, `macro_contact_handshake`) collapse multi-call sequences into one call each.
- **Keywords:** Agent Mail, file reservations, thread_id, macros
- **Seen:** 5840f61

### coordinator-active-session-management
- **What:** Active (not just passive) multi-agent session coordination: status digests, conflict detection, work-assignment negotiation.
- **Where:** `internal/coordinator` ("Package coordinator implements active session coordination for multi-agent workflows")
- **Notable:** `ntm coordinator status/digest/conflicts` surfaced directly in the CLI (README §4); `docs/planning/PLAN_TO_IMPROVE_NTM_PROJECT.md` documents upgrading a previously-passive coordinator identity ("OrangeFox") into an active one that negotiates file conflicts by priority score.
- **Keywords:** `ntm coordinator`, digest, conflict negotiation
- **Seen:** 5840f61

### git-worktree-isolation
- **What:** Per-agent git worktree/branch isolation as an alternative to reservation-based coordination.
- **Where:** `internal/worktrees`, `internal/git` ("Package git provides git worktree isolation services for multi-agent coordination")
- **Notable:** README explicitly frames worktrees as "isolation-first when policy allows it" vs. "coordination-first" reservations — two coexisting, switchable coordination philosophies (`ntm spawn --worktrees`, `ntm worktrees merge <pane>`).
- **Keywords:** `--worktrees`, `ntm worktrees merge`, isolation vs coordination
- **Seen:** 5840f61

### thundering-herd-prevention
- **What:** Staggered agent spawn timing to prevent duplicate-work races when multiple agents self-select tasks from `bv --robot-triage`/`bd ready` simultaneously at startup.
- **Where:** `docs/ORCHESTRATION_FEATURES.md`, `internal/scheduler` ("global spawn scheduler with paced pane/agent creation")
- **Notable:** Default 90s stagger justified by a typical 60-90s agent-startup sequence; env vars `NTM_SPAWN_ORDER`/`NTM_SPAWN_TOTAL`/`NTM_SPAWN_BATCH_ID`; alternative orchestrator-assignment mode (`--assign-work --assign-strategy=diverse`) and an optional soft-claim file protocol (`.ntm/claims/<bead>.json`).
- **Keywords:** `ntm spawn --stagger`, soft-claim, spawn scheduler
- **Seen:** 5840f61

### ensemble-multi-agent-reasoning
- **What:** Runs the same question across multiple agents/modes in parallel and synthesizes their outputs.
- **Where:** `internal/ensemble` ("types and utilities for multi-agent reasoning ensembles")
- **Notable:** v1.6.0 added "Ensemble synthesis (compare/diff runs, findings dedup/clustering, velocity tracking)"; v1.8.0 added checksum-verified ensemble export/import; ensemble also has checkpoint storage for partial-synthesis recovery mid-run; `ntm modes list --tier core|advanced` exposes named "reasoning modes" as a selectable ensemble input (`command_palette.md`).
- **Keywords:** `ntm ensemble spawn/status/synthesize`, dedup/clustering, `ntm modes`
- **Seen:** 5840f61

### swarm-pressure-governor
- **What:** A resource-pressure governor that tracks system/queue load and surfaces a normalized pressure level for orchestration decisions.
- **Where:** `internal/pressure` ("the NTM swarm pressure governor: a small..."), `internal/backpressure` (per-surface overload signals: tmux_capture, robot_command, rest_handler, sse_stream, websocket, profiler), `internal/contentionforecast`, `internal/fairness`
- **Notable:** `docs/robot-projection-sections.md` documents the resulting `resource_pressure` object (`mode: observe|enforce`, `overall: low..critical`, `limiting sources`) shared verbatim across two section-model docs; `proc_count` is normalized against `max(CPU*256, 4096)` so laptops and 64+-core hosts share one level vocabulary; `internal/contentionforecast` "predicts conflict-prone files before" (merge conflicts); `internal/fairness` "analyzes a sequence of completed scheduler/... decisions" for fairness.
- **Keywords:** pressure governor, resource_pressure, proc_count normalization, contention forecast
- **Seen:** 5840f61

### file-reservation-simulation-and-conflict-detection
- **What:** A deterministic in-memory simulator for file-reservation contention, plus live file-watch-based conflict detection.
- **Where:** `internal/reservationsim` ("a deterministic in-memory simulator for" file reservation contention), `internal/watcher` (conflict/debouncer/file_reservation files)
- **Notable:** Used to validate coordination logic without needing live Agent Mail; `internal/watcher` uses `fsnotify` with a polling fallback (per v1.1.0 changelog) and debouncing.
- **Keywords:** reservation simulator, fsnotify, debounce, file conflict
- **Seen:** 5840f61

### account-rotation
- **What:** Automatic multi-provider account/session rotation triggered on rate limits, with per-agent-type login/logout command definitions.
- **Where:** `internal/rotation` (`Provider` interface with Name/LoginCommand/ExitCommand/AuthSuccessPatterns methods), `internal/swarm` (`account_rotator.go`)
- **Notable:** v1.2.0 introduced `ntm rotate` with auto-trigger on rate limit; v1.14.0 added snapshot/restore of the Claude Code model setting across the swarm lifecycle specifically so per-swarm model overrides don't leak into global config.
- **Keywords:** `ntm rotate`, Provider interface, model-setting snapshot/restore
- **Seen:** 5840f61

### hypersync-proposed-multi-machine-fs-sync
- **What:** Two independent, unimplemented competing specs for a FUSE-based filesystem-replication layer that would let NTM distribute agents across multiple worker machines with a shared workspace.
- **Where:** `docs/planning/PROPOSED_HYPERSYNC_SPEC__CODEX.md`, `docs/planning/PROPOSED_HYPERSYNC_SPEC__OPUS.md`
- **Notable:** Both propose a single leader serializing filesystem mutations into an append-only op log, worker-side FUSE interception, QUIC control-plane + RaptorQ (RFC 6330) data-plane replication, BLAKE3 content-addressed chunking, and a "hazard" (not hard-block) model that surfaces overlapping writes via Agent Mail instead of preventing them. They directly contradict each other on partition handling: CODEX mandates strict "no silent divergence" (leader-unreachable flips the mount read-only/EROFS immediately) vs. OPUS's optimistic model (queue writes locally during partition, reconcile after healing) — and on whether the replication-integrity Merkle root includes reservation/lock state (CODEX excludes it, OPUS includes it).
- **Keywords:** HyperSync, FUSE, op log, hazard model, partition handling
- **Seen:** 5840f61

## context-memory

### checkpoint-restore-system
- **What:** Save/restore points for a session's full state (including tmux scrollback), designed as a routine cadence rather than a disaster-only feature.
- **Where:** `internal/checkpoint` ("checkpoint/restore functionality for NTM sessions", plus scrollback capture/compression)
- **Notable:** SKILL.md explicitly lists checkpoint cadence guidance: after prompts confirmed received, after root-cause isolation, before risky edits, after significant uncommitted work but before verification, after green verification, before merge/handoff. v1.13.0 added checkpoint restore via pane respawn with graceful goroutine shutdown.
- **Keywords:** `ntm checkpoint save/list/restore`, scrollback capture, checkpoint cadence
- **Seen:** 5840f61

### timeline-history-audit-trail
- **What:** Session timelines (replayable), prompt/session history search, and exportable audit records.
- **Where:** `internal/history` ("prompt history storage and retrieval"), `internal/export` ("exporting timeline visualizations"), `internal/audit`, `internal/tracker` ("state change tracking for delta snapshot queries")
- **Notable:** `ntm timeline show <session-id>` replays a session; `ntm history search "authentication error"` full-text-searches prompt history; `internal/tracker` backs "delta snapshot" queries (query only what changed since a prior point).
- **Keywords:** `ntm timeline`, `ntm history search`, delta snapshot
- **Seen:** 5840f61

### cass-cross-agent-search
- **What:** Indexes prior agent conversations across multiple coding-agent tools (Claude Code, Codex, Cursor, Antigravity, legacy Gemini, ChatGPT) so past solutions are reusable.
- **Where:** `internal/cass` ("CASS integration including context injection"), `internal/archive` ("background archiving of agent output for CASS indexing")
- **Notable:** Rule: "never run bare `cass`" (launches a blocking TUI) — always `--robot`/`--json`; auto-injection config knobs documented in ORCHESTRATION_FEATURES.md: `inject_limit=3`, `min_relevance=0.7`, `max_inject_tokens=500`, `skip_if_context_above=60`, `max_age_days=30`.
- **Keywords:** `cass search --robot`, auto-injection, `--with-cass`/`--no-cass`
- **Seen:** 5840f61

### cass-memory-self-learning-loop
- **What:** "Cass Memory System" (`cm`) — reflects on historical sessions across tools/projects/machines and extracts reusable "rules"/"playbook entries" queryable in future sessions.
- **Where:** `internal/cm` (client wrapper for external `cm` binary), `AGENTS.md` (Memory System section)
- **Notable:** Direct quote: "much like how human memory works." Protocol: call `cm context "<task>" --json` before non-trivial work (returns `relevantBullets`, `antiPatterns`, `historySnippets`, `suggestedCassQueries`); agents leave inline feedback comments `// [cass: helpful b-xyz] - reason` / `// [cass: harmful b-xyz] - reason`; "learning happens automatically" at session end. `cm onboard status/sample/read/mark-done` and `cm playbook add` manage the rule corpus.
- **Keywords:** `cm context`, playbook, `[cass: helpful/harmful]`, onboarding
- **Seen:** 5840f61

### runtime-sqlite-schema-designs
- **What:** SQLite-backed durable projection layer caching derived state (sessions/agents/work/coordination/quota) separate from source-of-truth systems (tmux/beads/mail).
- **Where:** `internal/state` ("durable SQLite-backed storage for NTM orchestration state"; files: ensemble_store, timeline_persist, timeline_lifecycle, runtime_store, runtime_schema, schema, store), `docs/runtime-schema-design.md`, `docs/sqlite-runtime-schema.md`, `docs/sqlite-runtime-tables.md`
- **Notable:** Three separate design docs for the *same* schema exist, all marked "RATIFIED" under bead `bd-j9jo3.2.1`, but mutually contradict each other: table-name prefix (`rt_*` vs bare names), attention-events cursor type (`TEXT "evt_<nanos>"` vs `INTEGER AUTOINCREMENT`), and `runtime_work` shape (singleton-row vs per-bead row). Shared concepts across all three: `source_health` per-upstream-source tracking, `attention_events` append-only log with `dedup_key`+60s dedup window, additive-only migrations (`007_runtime.sql`), GC with a 5-minute grace window after `stale_after`.
- **Keywords:** runtime projection, `source_health`, `attention_events`, additive migration
- **Seen:** 5840f61

### robot-durable-vs-recomputable-store
- **What:** A four-layer separation for the robot-mode SQLite store: Source Truth (external, never overwritten) → Runtime Projection (recomputable cache) → Durable Events (attention/incidents) → Watermarks (cursors/checkpoints), sharing one `~/.config/ntm/state.db` rather than a separate DB file.
- **Where:** `docs/robot-sqlite-schema.md`
- **Notable:** 9 core tables with full DDL (`robot_sessions`, `robot_attention_events` PK `TEXT "evt_<nanos>"`, `robot_incidents`, `robot_watermarks`, `robot_digest_cache`, etc.); explicit anti-goal: "Treating runtime projection state as equivalent to tmux truth"; retention pruning SQL given per table (e.g. attention_events 7d, incidents 90d resolved) with a guard that never prunes an event still referenced by an open incident.
- **Keywords:** layered schema, `robot_` table prefix, retention pruning SQL
- **Seen:** 5840f61

### historical-inspection-contract
- **What:** Defines 5 distinct historical query modes (live snapshot, `as_of` point-in-time, `incident_replay`, bounded `range`, raw `events` replay) each with explicit non-overlap rules.
- **Where:** `docs/robot-historical-inspection.md`
- **Notable:** Normative rule: "Live surfaces MUST ignore historical parameters rather than silently changing meaning." `as_of` defaults to `best_effort` reconstruction (vs. `strict`); 6 named staleness warnings (`STALE_SOURCE`, `RECONSTRUCTED`, `INTERPOLATED`, `PARTIAL_DATA`, `NEAR_RETENTION_EDGE`, `POLICY_CHANGED` — the last flags when redaction policy changed since the historical event); `export_postmortem` re-applies *current* redaction policy on export.
- **Keywords:** `as_of`, `incident_replay`, staleness warnings, postmortem export
- **Seen:** 5840f61

### support-bundle-diagnostics
- **What:** Collects a redacted diagnostic bundle (config, logs, state) with a manifest for support/debugging handoff.
- **Where:** `internal/bundle`, `internal/supportbundle` (manifest.go/closeout.go: `SchemaVersion`, `Host`, sha256 hashing, uses `redaction`)
- **Notable:** v1.7.0 changelog: "`ntm support-bundle` diagnostic collection with manifest verification"; manifest is sha256-hashed for integrity checking.
- **Keywords:** `ntm support-bundle`, manifest, sha256 verification
- **Seen:** 5840f61

## planning

### beads-issue-tracker-integration
- **What:** Wraps the external `br`/Beads dependency-aware issue tracker (`.beads/` JSONL, git-committed) as NTM's task/status source of truth.
- **Where:** `internal/bd` (message.go, `MessageClient` for beads messaging), `AGENTS.md` (Beads sections, incl. block marked `<!-- bv-agent-instructions-v1 -->`)
- **Notable:** Hard rule: "`br` never runs git commands automatically" — agent must manually `git add .beads/ && git commit` after `br sync --flush-only`; Beads issue ID (e.g. `br-123`) doubles as the Agent Mail `thread_id` and message subject prefix `[br-123]`, unifying two otherwise-separate systems' identifiers. Priorities 0(critical)-4(backlog); the rule "never edit `.beads/*.jsonl` directly" (README §3) is stated as a hard constraint.
- **Keywords:** `br ready --json`, `.beads/issues.jsonl`, shared thread_id
- **Seen:** 5840f61

### bv-graph-aware-triage
- **What:** Wraps the external `bv` (beads_viewer) graph-analysis engine that computes PageRank/betweenness/critical-path/cycles/HITS/eigenvector/k-core over the Beads dependency graph.
- **Where:** `internal/bv` ("Package bv provides integration with the beads_viewer (bv) tool")
- **Notable:** Hard rule: "Only `--robot-*` flags should be used — bare `bv` launches a blocking interactive TUI." `bv --robot-triage` is a "mega-command" (added v1.4.0) replacing 4 separate calls, returning `quick_ref`, ranked `recommendations`, `quick_wins`, `blockers_to_clear`, `project_health`, copy-paste `commands`. Two-phase analysis: Phase 1 instant (degree/topo/density), Phase 2 async with a 500ms timeout (PageRank/betweenness/HITS/eigenvector/cycles); every response includes a `data_hash` fingerprint of the source jsonl and a per-metric `status` (`computed|approx|timeout|skipped`).
- **Keywords:** `bv --robot-triage`, `data_hash`, two-phase analysis, quick_wins
- **Seen:** 5840f61

### ideation-roadmap-generation
- **What:** A gated pipeline that collects, ranks, and refines proposed new work items into an advisory roadmap, only invoked when the work queue is confirmed genuinely dry.
- **Where:** `internal/ideation` (collector.go, creation.go, ranker.go, refinement.go, evidence.go, guard.go — no doc comment, inferred from file names)
- **Notable:** Exposed via `ntm work queue-dry --ideate`; a `guard.go` component specifically implements the "don't ideate over real ready work" gate described in README §3; `--create-beads --yes --plan-version=<git-sha>` mutates Beads only after review, using the git SHA as an audit token.
- **Keywords:** `queue-dry --ideate`, guard, plan-version audit token
- **Seen:** 5840f61

### work-triage-cli-surface
- **What:** NTM's own operator-friendly wrapper presenting `bv`'s graph engine output.
- **Where:** `README.md` (Core Workflows §3), `internal/assign`, `internal/assignment`
- **Notable:** `ntm work triage --by-track`, `ntm work impact <file>`, `ntm work next`, `ntm work graph`; explicit guidance to use raw `bv --robot-*` instead when the native graph-engine output format is specifically wanted (SKILL.md).
- **Keywords:** `ntm work triage/next/graph/impact`, `ntm assign --auto --strategy=dependency`
- **Seen:** 5840f61

### builtin-todo-tool-override
- **What:** An agent may use its own built-in TODO tool instead of Beads, but only if the user explicitly asks.
- **Where:** `AGENTS.md` (~line 911-913)
- **Notable:** A narrow, explicit escape hatch from the otherwise-mandatory Beads-as-source-of-truth rule.
- **Keywords:** built-in TODO override, explicit user request
- **Seen:** 5840f61

## quality-gates

### ubs-bug-scanner-integration
- **What:** Wraps an external static/bug scanner ("Ultimate Bug Scanner") as a mandatory pre-commit gate.
- **Where:** `internal/scanner` ("Go wrapper for UBS (Ultimate Bug Scanner) integration", plus config_integration/bridge/analysis/autoscan/priority/dedup variants)
- **Notable:** AGENTS.md "Golden Rule: `ubs <changed-files>` before every commit." Findings are `file:line:col` + suggested fix, tiered Critical (nil deref/race/goroutine leak/unchecked error) / Important (type narrowing/div-by-zero/resource leak) / Contextual (TODO/FIXME/debug prints); `docs/planning/PLAN_TO_IMPROVE_NTM_PROJECT.md` documents routing UBS findings live to the specific pane/agent working on the affected file.
- **Keywords:** `ubs`, severity tiers, `--ci --fail-on-warning`
- **Seen:** 5840f61

### design-invariants-enforcement
- **What:** Codifies and mechanically checks the project's own non-negotiable design principles.
- **Where:** `internal/invariants` ("defines and enforces the 6 non-negotiable design invariants")
- **Notable:** The 6 invariants (No Silent Data Loss, Graceful Degradation, Idempotent Orchestration, Recoverable State, Auditable Actions, Safe by Default) are also documented prose-side in `README.md` ("Design Principles") and `docs/planning/PLAN_TO_IMPROVE_NTM_PROJECT.md` — i.e. the same 6 principles exist as both enforced code and narrative doc.
- **Keywords:** design invariants, no silent data loss, safe by default
- **Seen:** 5840f61

### robot-cli-drift-audit
- **What:** Compares the set of robot-mode commands against another surface (implied: CLI/TUI) to detect parity drift.
- **Where:** `internal/driftaudit` ("compares the set of robot-mode commands and...")
- **Notable:** Complements `internal/parity` ("compares the same session/project state as rendered" across surfaces) — two distinct automated-parity-checking packages (command-surface drift vs. rendered-state drift).
- **Keywords:** drift audit, parity check, cross-surface
- **Seen:** 5840f61

### doctor-readiness-assessment
- **What:** Produces a structured readiness diagnostic covering environment, dependencies, and configuration.
- **Where:** `internal/doctor` ("produces a structured readiness assessment for...")
- **Notable:** Exposed as `ntm doctor` (v1.4.0) and `--robot-health`-adjacent; `docs/planning/PLAN_TO_IMPROVE_NTM_PROJECT.md` lists "`ntm doctor` baseline checks" as a Phase -1 foundation item.
- **Keywords:** `ntm doctor`, readiness assessment
- **Seen:** 5840f61

### evidence-budget-verification
- **What:** Verifies that NTM's interactive operator loop stays within some declared evidence/response budget.
- **Where:** `internal/evidencebudget` ("verifies that NTM's interactive operator...")
- **Notable:** Name and description pair directly with `docs/robot-ordering-pagination.md`'s documented per-surface token/byte/latency budgets (e.g. snapshot: 2000 tokens/50KB/<500ms) — this package is the likely enforcement point for that contract.
- **Keywords:** payload budget enforcement, token budget
- **Seen:** 5840f61

### mandatory-compiler-and-lint-checks
- **What:** A required post-edit verification sequence for every code change.
- **Where:** `AGENTS.md` (Compiler Checks / Testing section), `Makefile` (`lint`, `fmt`, `test`, `test-all` targets)
- **Notable:** `go build ./cmd/ntm`, `go test -short ./...`, `golangci-lint run`, `gofmt -l .`, `goimports -l .` — all mandatory before considering a change done; `make pre-commit` conditionally re-runs the release-asset-naming contract test (`TestUpgradeAsset`) only when goreleaser/upgrade-related files are staged.
- **Keywords:** `go build`, `golangci-lint`, `TestUpgradeAsset`, pre-commit target
- **Seen:** 5840f61

### swarm-scale-verification-matrix
- **What:** A four-gate testing taxonomy (`short`/`race`/`bench`/`load`) with structured logging correlation fields and a fixed artifact-format catalog for swarm-scale performance/correctness verification.
- **Where:** `docs/verification-matrix-swarm-scale-vnext.md`
- **Notable:** Every gate logs `test_run_id` (UUIDv7), `gate`, `feature` (bead-id), `pressure_state` (`low|normal|elevated|high|critical`); `load` gate is opt-in (`NTM_SWARM_LOAD=1`, requires ≥32 cores/64GB host); `short`/`race` gates explicitly run with "no real Claude/Codex/Antigravity/Gemini CLI invoked, for reproducibility" — a "no-real-model mandate" for CI determinism. Change-control requires a bead reference plus a non-regression check: `bv --robot-triage | jq '.recommendations | length'` must not drop.
- **Keywords:** 4-gate taxonomy, `test_run_id`, no-real-model mandate, non-regression check
- **Seen:** 5840f61

### golden-fixture-regression-testing
- **What:** CI checks that robot-mode JSON output is byte-stable across changes using checked-in golden files.
- **Where:** `docs/robot-ordering-pagination.md`, `docs/robot-redesign-transition.md`, `internal/robot/testdata/robot_redesign/` (confirmed to exist; contains `scenario_*.json` fixtures named e.g. "healthy, degraded, stuck")
- **Notable:** Workflow: `ntm --robot-snapshot > testdata/golden/snapshot.json` then `git diff --exit-code`; named test functions `TestSnapshotPayloadSize`, `TestSnapshotOrderingDeterminism`.
- **Keywords:** golden files, `TestSnapshotOrderingDeterminism`, scenario fixtures
- **Seen:** 5840f61

### no-backwards-compatibility-policy
- **What:** An explicit, repeatedly-stated project policy rejecting compatibility shims, deprecated-API wrappers, or migration bridges.
- **Where:** `AGENTS.md` (~line 123-129), `docs/robot-api-design.md`
- **Notable:** Direct quote reused across docs: "We do not care about backwards compatibility—we're in early development with no users... NO TECH DEBT." Applied concretely: old JSON schema files are still *kept* on a major bump ("Keep `snapshot.v1.json` for reference") even though the API itself is allowed to break.
- **Keywords:** no tech debt, no compat shims, early-development stance
- **Seen:** 5840f61

## docs-style

### agents-md-as-binding-contract
- **What:** A 914-line `AGENTS.md` combining agent-safety meta-rules with full tool/workflow documentation, explicitly positioned as overridable only by direct user instruction.
- **Where:** `AGENTS.md`
- **Notable:** "RULE 0 — Fundamental Override Prerogative" states explicit user instruction always overrides the document itself — i.e. the doc encodes its own precedence rule at the top rather than assuming it.
- **Keywords:** AGENTS.md, RULE 0, override prerogative
- **Seen:** 5840f61

### changelog-keep-a-changelog-style
- **What:** A single `CHANGELOG.md` (1017 lines) spanning `[Unreleased]` back through `v1.0.0`, one section per semver release with dated headers.
- **Where:** `CHANGELOG.md`
- **Notable:** `.goreleaser.yaml` auto-generates changelog groupings (Features/Bug Fixes/Performance/Documentation/Others) from conventional-commit regexes and explicitly excludes docs/test/ci/chore/style/merge commits from the generated release notes — i.e. the hand-maintained CHANGELOG.md and the tool-generated GitHub release notes are two separate artifacts with different inclusion rules.
- **Keywords:** CHANGELOG.md, conventional commits, release-notes exclusion rules
- **Seen:** 5840f61

### upgrade-log-decision-record
- **What:** A dedicated `UPGRADE_LOG.md` recording dependency-upgrade *decisions* (not just version bumps): what changed, why, what was explicitly skipped and why, and what verification was run.
- **Where:** `UPGRADE_LOG.md`
- **Notable:** Records negative decisions with justification, e.g. "TypeScript 6 skipped in `web/` due to an `openapi-typescript` peer-dependency cap on TS 5.x," and preserves a note that a vendored `third_party/bubbletea` fork (confirmed present) "must not be blindly overwritten by upstream."
- **Keywords:** UPGRADE_LOG.md, skipped-upgrade justification, vendored-fork preservation note
- **Seen:** 5840f61

### competing-ai-drafted-plans-kept-side-by-side
- **What:** Multiple independently-AI-drafted proposals for the same feature (Web UI/REST/WS layer: 5 variants; HyperSync spec: 2 variants) are committed to the repo as parallel documents rather than merged into one.
- **Where:** the five web-UI plan variants (`docs/planning/PLAN_TO_ADD_WEB_UI_AND_REST_AND_WEBSOCKET_API_LAYERS_TO_NTM__CLAUDE_WEB.md`, `docs/planning/PLAN_TO_ADD_WEB_UI_AND_REST_AND_WEBSOCKET_API_LAYERS_TO_NTM__GEMINI.md`, `docs/planning/PLAN_TO_ADD_WEB_UI_AND_REST_AND_WEBSOCKET_API_LAYERS_TO_NTM__GPT.md`, `docs/planning/PLAN_TO_ADD_WEB_UI_AND_REST_AND_WEBSOCKET_API_LAYERS_TO_NTM__GPT_PRO.md`, `docs/planning/PLAN_TO_ADD_WEB_UI_AND_REST_AND_WEBSOCKET_API_LAYERS_TO_NTM__OPUS.md`), plus the two HyperSync specs (`docs/planning/PROPOSED_HYPERSYNC_SPEC__CODEX.md`, `docs/planning/PROPOSED_HYPERSYNC_SPEC__OPUS.md`)
- **Notable:** Docs are explicitly per-model-named in the filename itself (`__GEMINI`, `__GPT_PRO`, `__OPUS`, `__CODEX`) rather than anonymized or merged; the OPUS web-UI variant explicitly frames itself as "v3.0.0 'Ultimate Hybrid'... synthesizing Gemini+GPT+GPT_Pro plans" — a documented second-pass synthesis kept as its own file alongside the originals rather than replacing them.
- **Keywords:** per-model plan filenames, synthesis-as-new-document, unresolved design forks
- **Seen:** 5840f61

### checked-in-json-schema-artifact-convention
- **What:** A documented (though not yet materialized on disk) convention of hand-curating JSON Schema files per major version rather than reflecting them from Go structs.
- **Where:** `docs/robot-schema-versioning.md`
- **Notable:** Declares fixed layout `docs/schemas/{robot,sections,envelope}/*.v1.json` + companion `.example.json` files "manually curated (not generated)," with an explicit "Why Not Auto-Generation?" rationale section (5 reasons) — however `docs/schemas/` does not currently exist in the repo, so this convention is documented-but-not-yet-implemented.
- **Keywords:** `docs/schemas/`, manual schema curation, `.example.json`
- **Seen:** 5840f61

### multi-persona-project-self-audit-report
- **What:** A 513-line report applying 10 different reasoning-mode personas (drawn from a companion 80-entry taxonomy doc) to independently audit NTM's own architecture and security posture.
- **Where:** `docs/planning/MODES_OF_REASONING_REPORT_AND_ANALYSIS_OF_PROJECT.md`, `docs/planning/modes_of_reasoning.md`
- **Notable:** Findings include divergent conclusions between modes on the same question (e.g. whether 337K LOC is proportionate scope), a quantified FMEA risk register with Risk-Priority-Numbers, and named critical security findings (see [[safety]] domain). Explicitly documents "overall confidence 0.82" and a "per-mode contribution scoreboard" as part of the report format itself.
- **Keywords:** reasoning-mode personas, FMEA risk register, per-mode scoreboard
- **Seen:** 5840f61

## tooling

### robot-flag-command-surface
- **What:** ~100+ `--robot-*` global CLI flags forming NTM's primary machine-readable automation surface, governed by a strict naming/design contract.
- **Where:** `docs/robot-api-design.md`, `internal/robot` (largest package: dozens of files — activity, ack, ensemble, patterns, diagnose, health, routing, etc.)
- **Notable:** Naming convention: `--robot-<verb>=SESSION` (session-scoped) vs. bare bool flags (global) vs. `--robot-<tool>-<action>` (external tool bridges); inline-value flags like `--robot-jfp-search="debugging"` are explicitly deprecated in favor of separate modifier flags. A ~25-command-family deprecated→canonical flag mapping table exists for migration. `--robot-tools` enumerates optional external bridges (JFP, ACFS, MS, DCG, SLB, plus rolling-out RU/UBS/GIIL/XF), each returning `DEPENDENCY_MISSING` if the underlying tool is absent.
- **Keywords:** `--robot-*`, flag naming convention, `DEPENDENCY_MISSING`
- **Seen:** 5840f61

### robot-output-envelope-and-error-contract
- **What:** Every robot command returns a standard envelope (`success`, `timestamp`, `error`, `error_code`, `hint`) plus an `_agent_hints` object (human summary, suggested action, `safer_alternative`), with exit codes 0/1/2 (2 reserved for "unavailable"/`NOT_IMPLEMENTED`).
- **Where:** `docs/robot-api-design.md`, `docs/robot-action-errors.md`, `docs/robot-action-handoff-contract.md`
- **Notable:** Two documents (`robot-action-errors.md` and `robot-action-handoff-contract.md`) share the same title and bead ID (`bd-j9jo3.1.7`) but define non-overlapping error-code vocabularies (`CURSOR_*/ENTITY_*/SOURCE_*` vs. `SESSION_NOT_FOUND/PANE_NOT_FOUND/*_NOT_FOUND`) — an unresolved documentation fork. `next_actions` array items carry `id/label/command/priority/category/applicable_when/requires_input/destructive/idempotent`; a `recovery` object models `type: resync|retry|fix_request|wait|escalate`.
- **Keywords:** output envelope, `_agent_hints`, `next_actions`, exit codes
- **Seen:** 5840f61

### request-identity-and-idempotency
- **What:** Formal request-tracking IDs (`req_<timestamp>_<random>`), scoped idempotency keys (`request`/`session`/`persistent`/`indefinite`), and a per-command-type retry-safety classification.
- **Where:** `docs/robot-request-identity.md`
- **Notable:** `--robot-action-status --request-id=...` lets clients poll completion status (`pending/completed/failed/unknown`) rather than blindly retrying — explicitly designed for crash recovery via a client-side `pending_requests.json`. `spawn` is flagged as "Duplicate session possible" without an idempotency key; `REQUEST_SUPERSEDED` vs `REQUEST_CONFLICT` are two distinct concurrency-error conditions with separate recovery paths.
- **Keywords:** `req_<timestamp>_<random>`, idempotency scope, `robot-action-status`
- **Seen:** 5840f61

### canonical-resource-reference-syntax
- **What:** A universal `<type>:<scope>/<id>[@<version>]` reference format spanning 13 entity types, plus wildcard/group "broadcast" targets.
- **Where:** `docs/robot-resource-references.md`
- **Notable:** `scope` is always the literal string `ntm` today — deliberately reserving room for a future non-local scope; 4 durability classes (Permanent/Session-bound/Ephemeral/Request-scoped) map to a 3-state lifecycle `ACTIVE → STALE → INVALID`, where `STALE` still returns last-known data with a warning rather than erroring; dynamic broadcast targets like `agents:ntm/idle` select by computed state, not static membership.
- **Keywords:** `type:scope/id`, broadcast target, `ACTIVE/STALE/INVALID`
- **Seen:** 5840f61

### ordering-pagination-and-payload-budgets
- **What:** Deterministic sort keys per collection type, a fixed severity/actionability total order, and explicit per-surface token/byte/latency payload budgets.
- **Where:** `docs/robot-ordering-pagination.md`
- **Notable:** Severity order `critical > error > warning > info > debug`; actionability order `action_required > interesting > background`; concrete budget constants (e.g. `SnapshotSessionsLimit = 20`, `TailLinesMax = 500`); 5 detail-verbosity levels (`minimal..verbose`) each mapping to which struct fields are serialized; per-transport token multipliers relative to JSON baseline (TOON 0.7x, Markdown 1.2x, Terse 0.1x).
- **Keywords:** deterministic ordering, payload budget, detail-level matrix, transport token multiplier
- **Seen:** 5840f61

### toon-token-efficient-output-format
- **What:** An alternate output encoding ("TOON") that shells out to an external Rust binary (`tru`) for more token-efficient robot-mode responses than JSON.
- **Where:** `docs/planning/AGENT_FRIENDLINESS_REPORT.md`, `docs/planning/TOON_INTEGRATION_BRIEF.md`, `internal/robot` (`renderer.go`, `toon.go`)
- **Notable:** Discovered via `PATH` or `TOON_BIN`/`TOON_TRU_BIN` env vars; falls back to JSON for unsupported nested structures; explicit subprocess-not-CGO/WASM design rationale ("keeps Go build pure... benchmark ~3.45ms/call including process startup"); a documented gap: `config.toml [robot.output] format` exists but is *not wired into* `resolveRobotFormat()` — only the CLI flag/env are honored.
- **Keywords:** TOON, `tru` binary, subprocess-not-CGO, config-wiring gap
- **Seen:** 5840f61

### unified-robot-command-registry
- **What:** A single Go source-of-truth registry (`Registry.Surfaces/Sections/Lanes`) that `--robot-capabilities`, `--robot-help`, and `--robot-schema` all derive from, replacing three previously-independent metadata sources.
- **Where:** `docs/robot-command-registry.md`, `internal/kernel` (registry.go: `HandlerFunc`/`Registry`)
- **Notable:** Named problem statement: "Before the redesign, robot mode metadata lived in three places... This led to drift." Core invariant: "Declared once, derived everywhere." `ValidateRegistry()` is meant to run in CI to catch section/lane inconsistencies before deploy.
- **Keywords:** RobotRegistry, "declared once, derived everywhere", `ValidateRegistry`
- **Seen:** 5840f61

### rest-sse-websocket-server-and-openapi
- **What:** `ntm serve` exposes REST (`/api/v1`), SSE (`/events`), and WebSocket (`/ws`) surfaces, backed by a generated OpenAPI document.
- **Where:** `internal/serve` ("HTTP server for NTM with REST API and event streaming"; beads/openapi/scanner/mail/cass/accounts/audit/rbac/checkpoints/pipelines/safety/ws_events endpoints), `docs/openapi.json`, `docs/openapi-kernel.json`, `docs/parity_matrix.json`
- **Notable:** `openapi.json` (214 paths, 101 schemas) is machine-generated from `parity_matrix.json` (223 tracked endpoints) aiming at full CLI/TUI/robot parity; `openapi-kernel.json` is a separate, much smaller (13 paths) hand-scoped surface generated from the kernel command registry instead — both share the literal title "NTM REST API" despite different sources/versioning (`0.1.0-draft` vs `dev`), and are not a strict subset of each other by URL-prefix convention.
- **Keywords:** `ntm serve`, `docs/openapi.json`, parity matrix, kernel vs full API surface
- **Seen:** 5840f61

### plugin-sdk-for-external-commands
- **What:** An SDK letting external binaries register as additional NTM CLI subcommands, loaded dynamically at startup alongside the ~90 statically-registered commands.
- **Where:** `internal/plugins` (sdk.go/agent.go/command.go: `SDKVersion`, `Capability` — inferred, no doc comment), `internal/cli/root.go` (calls plugins.LoadCommandPlugins at startup)
- **Notable:** After static command registration, `root.go` dynamically loads command plugins and registers each as a `cobra.Command` that shells out to the plugin's `Execute(args, env)` — i.e. NTM's CLI surface is extensible without recompiling.
- **Keywords:** `plugins.LoadCommandPlugins`, `Execute(args, env)`, dynamic subcommands
- **Seen:** 5840f61

### tool-adapter-framework
- **What:** A unified adapter framework for integrating external ecosystem tools (beyond the coding-agent CLIs themselves) behind one interface.
- **Where:** `internal/tools` ("unified adapter framework for external ecosystem tools"), `internal/integrations` ("coordination between external tool integrations")
- **Notable:** v1.4.0 changelog explicitly names this "Tool Adapter Framework" as a foundational addition alongside the Daemon Supervisor and Durable State Store.
- **Keywords:** tool adapter framework, `internal/integrations`
- **Seen:** 5840f61

### ast-grep-vs-ripgrep-vs-warp-grep-guidance
- **What:** Documented decision guidance for agents on which of three different code-search tools to use for a given task.
- **Where:** `AGENTS.md` (ast-grep/ripgrep section, Morph Warp Grep section)
- **Notable:** Rule of thumb: correctness/applying structural changes → `ast-grep`; raw text-search speed → `ripgrep`; "how does X work?" architecture questions → `mcp__morph-mcp__warp_grep` (an AI-powered tool that expands a natural-language query, greps, reads relevant files, and returns precise line ranges with context) — explicitly positioned as distinct from both the structural (ast-grep) and literal (ripgrep) tools.
- **Keywords:** ast-grep, ripgrep, warp_grep, tool-selection guidance
- **Seen:** 5840f61

### remote-compilation-helper
- **What:** Offloads `go build`/`go test`/`golangci-lint` to a fleet of 8 remote VPS workers to avoid "compilation storms" when many agents build simultaneously on one local machine.
- **Where:** `AGENTS.md` (RCH section)
- **Notable:** Installed at `~/.local/bin/rch`; explicitly framed as solving a specific multi-agent-swarm resource-contention problem (many agents compiling the same repo at once) rather than being a generic CI speedup.
- **Keywords:** rch, compilation storms, remote build fleet
- **Seen:** 5840f61

## config-packaging

### user-vs-project-config-layering
- **What:** NTM resolves configuration from two levels — user-level (`~/.config/ntm/...`) and project-level (`.ntm/...`) — with project assets overriding user/built-in defaults.
- **Where:** `README.md` (Configuration and Project Assets), `internal/config` ("scanner configuration types and loading" — also the general TOML config loader), `internal/claudeconfig`
- **Notable:** User-level assets: `config.toml`, `recipes.toml`, `workflows/`, `personas.toml`, `~/.ntm/policy.yaml`. Project-level: `.ntm/workflows/`, `.ntm/pipelines/`, `.ntm/personas.toml`, `.ntm/recipes.toml`, `.ntm/checkpoints/`. `internal/claudeconfig` separately manages the persistent Claude Code CLI's own config file (read/write access), distinct from NTM's own config.
- **Keywords:** `~/.config/ntm/`, `.ntm/`, config layering
- **Seen:** 5840f61

### goreleaser-cross-platform-packaging
- **What:** A single GoReleaser config building/publishing NTM across 6 platform×arch combinations plus multiple downstream package-manager formats.
- **Where:** `.goreleaser.yaml`
- **Notable:** Targets linux/darwin/windows/freebsd × amd64/arm64/arm(v7) with specific exclusions (windows-arm64/arm, freebsd-arm/arm64); a separate universal darwin binary (`ntm-universal`) combines both darwin arches; downstream formats include Homebrew cask, `nfpm` deb/rpm/apk/archlinux (each declaring `tmux` as a runtime dependency, recommending `fzf`, suggesting `tmuxinator`), a Scoop manifest for Windows, SBOM generation, and keyless `cosign` checksum signing. A `ghcr.io/dicklesworthstone/ntm` container tag is referenced in release notes even though the actual Docker image build step is commented out with a TODO.
- **Keywords:** goreleaser, nfpm, cosign, universal darwin binary, disabled Docker build
- **Seen:** 5840f61

### install-script-with-checksum-verification
- **What:** A `curl | bash` installer that resolves the correct platform/version/asset, verifies a SHA256 checksum before trusting the download, and offers shell-integration auto-setup.
- **Where:** `install.sh`
- **Notable:** Fails closed if no checksum file is found for the downloaded asset (`verify_downloaded_asset`); `--easy-mode` silently appends `PATH`/shell-eval lines to the user's rc file, while the default mode only prints instructions; detects and offers to auto-upgrade a legacy `ntm init <shell>` rc-file entry to the newer `ntm shell <shell>` form (v1.6.0+), backing up the rc file first.
- **Keywords:** checksum verification, `--easy-mode`, legacy shell-integration auto-upgrade
- **Seen:** 5840f61

### upgrade-asset-naming-contract-test
- **What:** A dedicated Go test (`TestUpgradeAsset`) validates that release-asset naming produced by GoReleaser matches what NTM's own `ntm upgrade`/self-update logic expects to find.
- **Where:** `Makefile` (`upgrade-contract` target), `internal/cli` (implied location of the test per Makefile reference), `.goreleaser.yaml` (naming-contract comment on the archive template)
- **Notable:** `.goreleaser.yaml`'s archive-naming template is explicitly annotated as "part of an upgrade naming contract tied to `internal/cli/upgrade.go` and `internal/cli/cli_test.go`" — i.e. the packaging config and the self-upgrade code are treated as one contract requiring joint testing, and `make pre-commit` conditionally re-runs this test specifically when goreleaser/upgrade files are staged.
- **Keywords:** `TestUpgradeAsset`, naming contract, conditional pre-commit re-test
- **Seen:** 5840f61

### upgrade-protection-checksum-verification
- **What:** NTM's self-update path verifies a SHA256 checksum and performs post-upgrade binary verification before replacing the running binary.
- **Where:** `CHANGELOG.md` (v1.5.0 entry: "Upgrade Protection")
- **Notable:** Distinguished from the *install-time* checksum check in `install.sh` — this is the same guarantee applied again at every subsequent self-update (`ntm upgrade`/`ntm self-update`), not just first install.
- **Keywords:** `ntm upgrade`, SHA256 verification, post-upgrade verification
- **Seen:** 5840f61

### dependency-upgrade-audit-trail
- **What:** A dated log recording each dependency-upgrade pass across all three manifests in the repo (Go, web/, vscode/) with rationale, breaking-change notes, and verification run.
- **Where:** `UPGRADE_LOG.md`
- **Notable:** Verification battery listed per entry: `npm run test:run/lint/build/audit` (both `web/` and `vscode/`), `go build`, `go mod tidy -diff`, `go test -short/-v` (incl. E2E against a freshly built binary), `golangci-lint run`, `govulncheck`, `gofmt -l`/`goimports -l`.
- **Keywords:** UPGRADE_LOG.md, govulncheck, per-manifest verification battery
- **Seen:** 5840f61

## repo-layout

### single-main-branch-policy
- **What:** The repo uses exactly one long-lived branch (`main`) — no `master`, no persistent side branches like `beads-sync`.
- **Where:** `AGENTS.md` (~line 31-41)
- **Notable:** Stated as a hard convention rather than emergent practice, presumably to keep multi-agent swarms from diverging onto uncoordinated branches.
- **Keywords:** main-only, no side branches
- **Seen:** 5840f61

### clean-dag-package-dependency-graph
- **What:** Despite ~106 `internal/` packages, the import dependency graph among them is a clean, acyclic DAG.
- **Where:** `docs/planning/MODES_OF_REASONING_REPORT_AND_ANALYSIS_OF_PROJECT.md`
- **Notable:** Called "the single strongest architectural property of the codebase" in the self-audit report, with a measured import-chain depth of 14; contrasted against the same report's "god package" finding (below) as a structural strength alongside a structural weakness.
- **Keywords:** zero circular dependencies, import depth 14
- **Seen:** 5840f61

### god-package-architecture-finding
- **What:** Two packages concentrate a disproportionate share of the codebase's logic: `cli` (~75K LOC / 233 files) and `robot` (~62K LOC / 87 files, with `robot.go` alone 10,717 lines / 288 functions).
- **Where:** `internal/cli` (confirmed: "Package cli provides command-line interface commands for ntm" / root.go alone ~4600+ lines per Agent D's findings), `internal/robot`, `docs/planning/MODES_OF_REASONING_REPORT_AND_ANALYSIS_OF_PROJECT.md`
- **Notable:** Report also flags 227 `os.Exit()` calls inside `root.go`'s robot-flag handling as a related smell, and recommends splitting `robot.go` as a P2 remediation item.
- **Keywords:** god package, `robot.go` 10,717 lines, `os.Exit()` count
- **Seen:** 5840f61

### vendored-bubbletea-fork
- **What:** The repo vendors a modified fork of the `bubbletea` TUI library rather than depending on upstream directly.
- **Where:** `third_party/bubbletea` (confirmed present)
- **Notable:** `UPGRADE_LOG.md` explicitly warns this fork carries an NTM-specific `tea_init.go` patch that "must not be blindly overwritten by upstream" during dependency upgrades; the self-audit report separately notes the fork's only meaningful change is "disabling an eager background-color probe causing multi-second startup latency" (3,426 lines of diff for one behavioral change) and recommends eventually upstreaming the fix instead of maintaining the fork (P3).
- **Keywords:** `third_party/bubbletea`, `tea_init.go` patch, vendored fork
- **Seen:** 5840f61

### origin-story-and-shell-to-binary-migration
- **What:** A captured design conversation documenting why NTM was built as a compiled Go binary + thin shell-integration line, replacing what had been a ~2200-line inline `.zshrc` script.
- **Where:** `docs/planning/PLAN_TO_MAKE_NTM.md`
- **Notable:** Explicitly modeled on the zoxide/starship/fzf/atuin pattern (`eval "$(ntm init zsh)"`); considers and rejects keeping logic as a shell script for maintainability/type-safety/testability reasons; also proposed native Bubble Tea TUI over fzf+ANSI-escapes and TOML config over markdown-parsed config — decisions that shaped the current architecture.
- **Keywords:** origin story, binary+thin-shell pattern, `.zshrc` replacement
- **Seen:** 5840f61

## safety

### foundational-agent-safety-meta-rules
- **What:** A small set of absolute rules governing what an agent working in this repo may never do, positioned above all other repo guidance.
- **Where:** `AGENTS.md` (RULE 0, RULE 1, ~line 7-27)
- **Notable:** RULE 0 (explicit user instruction always overrides the doc); RULE 1 (no file/folder deletion without express written permission, even for files the agent itself created); irreversible actions (`git reset --hard`, `git clean -fd`, `rm -rf`) forbidden unless the user gives the *exact* command in the same message and explicitly accepts irreversible consequences — the agent must restate the command verbatim and log the authorization text before running it.
- **Keywords:** RULE 0, RULE 1, verbatim-restatement-before-destructive-action
- **Seen:** 5840f61

### destructive-command-policy-engine
- **What:** Pattern-based classification of shell commands into blocked/approval-required/allowed tiers.
- **Where:** `internal/policy` ("destructive command protection through pattern matching")
- **Notable:** `docs/planning/PLAN_TO_IMPROVE_NTM_PROJECT.md` documents the originating incident that motivated this: "a real incident where an agent's `git checkout --` erased another agent's work." Enforcement layers: `.ntm/policy.yaml` 3-tier pattern list + PATH-shadowing wrapper scripts (`.ntm/bin/git`) + a Claude Code `PreToolUse` hook; `auto_push` defaults to `false`.
- **Keywords:** `ntm policy`, 3-tier pattern list, PATH-shadowing wrapper
- **Seen:** 5840f61

### two-person-approval-workflow
- **What:** A durable, auditable approval engine supporting SLB-style two-person sign-off for high-risk operations.
- **Where:** `internal/approval` ("unified approval workflow engine for NTM")
- **Notable:** Introduced v1.4.0; the self-audit report (`MODES_OF_REASONING_REPORT...md`) flags a concrete bypass: in local anonymous-auth mode the two-person check compares `"unknown"=="unknown"`, making self-approval trivially pass — a documented finding, not a fix.
- **Keywords:** `ntm approve`, SLB two-person rule, self-approval-bypass finding
- **Seen:** 5840f61

### redaction-engine
- **What:** Detects and redacts ~15 categories of sensitive content (provider API keys, cloud credentials, auth tokens, generic secrets, private keys, DB connection strings) with a deterministic, non-reversible placeholder format.
- **Where:** `internal/redaction` (canonical), `internal/safety/redaction` (compat wrapper — the only content under `internal/safety/`; confirmed no top-level Go files exist directly in `internal/safety/`), `docs/REDACTION_SPEC.md`
- **Notable:** Placeholder format `[REDACTED:<CATEGORY>:<hash8>]` where `hash8` = first 8 hex chars of `sha256(category+":"+matched_content)` — non-reversible but deterministic (repeated occurrences of the same secret hash identically for correlation). 4 operation modes: `off`/`warn`/`redact`/`block`. Performance target: scan 1MB in <100ms using Go's RE2 (no backtracking). Legacy pattern set from `internal/checkpoint/export.go`'s older `--redact-secrets` flag is explicitly required to be a subset of the new unified engine's coverage.
- **Keywords:** `[REDACTED:CATEGORY:hash8]`, canonical vs. compat package, RE2 performance budget
- **Seen:** 5840f61

### robot-output-disclosure-states
- **What:** A 5-state disclosure model (`visible`/`preview_only`/`redacted`/`withheld`/`hashed`) applied per-field to every robot-mode response, with asymmetric retroactive-redaction rules for replayed history.
- **Where:** `docs/robot-sensitivity-redaction.md`
- **Notable:** Explicit asymmetric rule: "Event was visible, now sensitive → Redact in replay" but "Event was redacted, now safe → Keep redacted (conservative)." `hashed` uses a session-specific salt so the same secret hashes differently across sessions (deliberately limiting cross-session correlation of secrets). A field-disclosure matrix defaults `env_dump` to `withheld` outright (no scan, no preview) — the most conservative default in the whole matrix. Note: this doc's state name `hashed` differs slightly from `docs/robot-contract-examples.md`'s `hashed_evidence` for the same concept.
- **Keywords:** disclosure state, `env_dump: withheld`, session-salted hash
- **Seen:** 5840f61

### encryption-at-rest
- **What:** Opt-in AES-256-GCM encryption for prompt history, event logs, checkpoint exports, and support bundles, with a key-rotation keyring.
- **Where:** `internal/encryption` (encryption.go/key.go: `KeySize=32`, `NonceSize=12`, `FormatVersion`), `docs/ENCRYPTION_SPEC.md`
- **Notable:** Key sourced via `env`/`file`/`command` modes (the `command` mode runs an arbitrary shell command and reads key bytes from stdout, ignoring stderr); `[encryption.keyring]` maps key IDs to key material, decryption tries all keyring keys in order, rotation = add new key → mark active → old data stays decryptable without re-encrypting; never generates ephemeral/automatic keys — all failure modes (missing key, bad format, decrypt failure) fail loudly.
- **Keywords:** AES-256-GCM, `NTM_ENCRYPTION_KEY`, keyring rotation
- **Seen:** 5840f61

### privacy-mode
- **What:** A global mode that applies redaction/scrubbing to outbound data (webhooks, notifications) rather than only at-rest.
- **Where:** `internal/privacy` ("privacy mode enforcement for NTM")
- **Notable:** v1.7.0 changelog: introduced alongside `ntm scrub` (outbound webhook/notification redaction) and `ntm preflight` (prompt validation with lint rules/PII checkers before a prompt is even sent).
- **Keywords:** `ntm scrub`, `ntm preflight`, outbound redaction
- **Seen:** 5840f61

### incident-taxonomy-and-promotion-rules
- **What:** A durable, SQLite-persisted incident model distinct from ephemeral alerts, with 6 incident families and numeric promotion/escalation thresholds.
- **Where:** `docs/incident-taxonomy.md`
- **Notable:** Alert = momentary/attention-feed-only/auto-clears/lost after GC; Incident = durable/stable-ID/explicit lifecycle/retained for review. 6 families with concrete thresholds: Agent (`crash_loop`: 3+ crashes/30min), Session (`unhealthy`: score<0.5 for 10min), Quota (`chronic_pressure`: >90% for 30min), Coordination (`reservation_deadlock`: circular wait >10min), Source (`outage`: >5min), Work (`no_progress`: no bead progress >4hr with active agents). Severity escalation rule: warning persisting >1h → error; error persisting >4h → critical. Fingerprint format `family:scope:discriminator` deduplicates repeated incidents into updates of one record.
- **Keywords:** alert vs incident, `crash_loop`, fingerprint dedup, severity escalation
- **Seen:** 5840f61

### agent-mail-identity-hygiene
- **What:** Reports on the health/consistency of Agent Mail identity records (stale registrations, duplicate identities, etc.).
- **Where:** `internal/identityhygiene` ("reports on Agent Mail identity records and...")
- **Notable:** A dedicated package solely for auditing the identity layer of the coordination system, separate from the coordination logic itself — treating "who is a valid registered agent" as its own guarded concern.
- **Keywords:** identity hygiene, Agent Mail identity audit
- **Seen:** 5840f61

### critical-security-findings-from-self-audit
- **What:** The multi-persona self-audit report documents specific, unremediated vulnerabilities found in NTM's own REST/robot surfaces.
- **Where:** `docs/planning/MODES_OF_REASONING_REPORT_AND_ANALYSIS_OF_PROJECT.md`
- **Notable:** (1) `handlePaneInputV1` passes arbitrary JSON-body text directly to `tmux.SendKeys` with no `ValidateSessionName()` call and no policy-engine check — described as "arbitrary command execution for any local process in default (unauthenticated local) mode." (2) Any admin-level caller (default in local mode) can `PATCH allowed_origins` to `*` (CORS mutation vulnerability). (3) The SLB self-approval bypass (see two-person-approval-workflow above). All three are catalogued as findings in the report, not confirmed-fixed items.
- **Keywords:** pane-input injection, CORS mutation, unauthenticated local mode
- **Seen:** 5840f61

## self-improvement

### cass-memory-reflective-learning-loop
- **What:** See [[context-memory]] `cass-memory-self-learning-loop` — the same `cm` system is also the project's primary self-improvement mechanism: agents leave graded feedback on which memory rules helped or hurt, and the system "automatically" incorporates that at session end.
- **Where:** `internal/cm`, `AGENTS.md` (Memory System section)
- **Notable:** The dual framing (durable context-memory system *and* self-improvement loop) is explicit in the doc's own language: past sessions are "reflected on" to extract "reusable lessons," not merely retrieved verbatim.
- **Keywords:** reflection, `cm playbook add`, graded feedback
- **Seen:** 5840f61

### multi-persona-self-audit-methodology
- **What:** NTM's own maintainers ran a structured 10-persona reasoning-mode audit of the codebase as a documented self-improvement exercise, using an independently-authored general reasoning taxonomy as the methodological scaffold.
- **Where:** `docs/planning/modes_of_reasoning.md` (80-entry taxonomy across 12 lettered categories A-L), `docs/planning/MODES_OF_REASONING_REPORT_AND_ANALYSIS_OF_PROJECT.md`
- **Notable:** Selected 10 modes across 6 of 12 taxonomy categories, deliberately covering "all 7 taxonomy axes... on both poles" (e.g. ampliative vs. non-ampliative, monotonic vs. non-monotonic reasoning) to force genuinely different critical perspectives rather than 10 similar takes; produces a prioritized P0-P3 remediation table and even proposes new capabilities (NTM-as-MCP-server, filesystem-inbox alternative to tmux send-keys) as "new ideas," not just findings.
- **Keywords:** reasoning-mode taxonomy, forced perspective diversity, P0-P3 remediation table
- **Seen:** 5840f61

### ideation-guarded-roadmap-generation
- **What:** See [[planning]] `ideation-roadmap-generation` — same mechanism, but functions as a self-improvement loop specifically when the existing task graph runs dry, generating new candidate work rather than just prioritizing existing work.
- **Where:** `internal/ideation`
- **Notable:** The guard (`guard.go`) is explicitly the mechanism preventing this self-improvement loop from firing spuriously over real ready work — i.e. self-generated work is treated as lower-trust than human/agent-authored beads by default.
- **Keywords:** guarded self-generated work, `guard.go`
- **Seen:** 5840f61

### effectiveness-and-cost-scoring
- **What:** Tracks per-agent effectiveness and per-session API cost as feedback signals for future routing/assignment decisions.
- **Where:** `internal/scoring` ("effectiveness metrics for NTM agent evaluation" / "score tracking for NTM agents"), `internal/cost` ("API cost tracking for AI agent sessions"), `internal/metrics` ("success metrics tracking for NTM orchestration")
- **Notable:** v1.13.0 changelog specifically adds "agent-effectiveness ranking"; v1.8.0 adds a dedicated "effectiveness-tracking module for assignments" — i.e. assignment decisions are meant to close the loop against measured past effectiveness, not just static capability profiles.
- **Keywords:** effectiveness ranking, `internal/cost`, assignment feedback loop
- **Seen:** 5840f61

### agentic-coding-flywheel-integration-plan
- **What:** A large improvement proposal organizing NTM's evolution around a named 5-stage self-reinforcing loop across the whole "Dicklesworthstone Stack" of tools.
- **Where:** `docs/planning/PLAN_TO_IMPROVE_NTM_PROJECT.md`
- **Notable:** Loop stages: PLAN → COORDINATE → EXECUTE → SCAN → REMEMBER — mapping directly onto Beads/bv (plan), Agent Mail/worktrees (coordinate), swarm/pipeline (execute), UBS (scan), and CASS/CM (remember). Contains its own quantified success metrics for the improvement effort itself (e.g. agent bootstrap calls 4-5→1, CM query latency ~500ms→<50ms, token usage via markdown 100%→50%) and a documented internal defect: its own table of contents promises 4 sections that never appear in the document body.
- **Keywords:** Agentic Coding Flywheel, PLAN-COORDINATE-EXECUTE-SCAN-REMEMBER, quantified before/after metrics
- **Seen:** 5840f61

## ux

### tui-dashboard-and-palette
- **What:** A Bubble Tea/Lip Gloss terminal dashboard and a fuzzy-searchable command palette, both operator-facing (not agent-facing).
- **Where:** `internal/tui` ("terminal user interface components"), `internal/palette` (model.go/selector.go/xf_search.go — inferred, no doc comment)
- **Notable:** SKILL.md explicitly draws the line: "`ntm dashboard`, `ntm palette`, and other TUI surfaces are for humans... For machine-readable automation, prefer `--robot-*`." Palette is bindable to a tmux popup hotkey (default F6, `ntm bind`) and driven by the separately-versioned `command_palette.md` prompt library (see [[skills]]).
- **Keywords:** `ntm dashboard`, `ntm palette`, F6 hotkey, human-vs-robot surface split
- **Seen:** 5840f61

### catppuccin-theme-system
- **What:** A configurable color theme system for the TUI, including a `NO_COLOR`-respecting mode.
- **Where:** `CHANGELOG.md` (v1.2.0: "theme system (Catppuccin Latte, NO_COLOR)")
- **Notable:** Explicit accessibility/CI-friendliness accommodation via `NO_COLOR` alongside an aesthetic theme choice (Catppuccin) — both introduced in the same release.
- **Keywords:** Catppuccin, `NO_COLOR`
- **Seen:** 5840f61

### tui-overhaul-glamour-upgrade
- **What:** A major TUI rework adding a spring-based animation engine, a guided spawn wizard, and form-based input.
- **Where:** `CHANGELOG.md` (pre-v1.9.0 "Unreleased" cycle entry: "TUI Overhaul 'Glamour Upgrade'")
- **Notable:** Uses the vendored `third_party/bubbletea` fork (see [[repo-layout]]) plus `huh` forms; shipped alongside the Attention Feed System and a "6-panel mega dashboard layout" in the same release cycle, i.e. the UX overhaul and the new event/attention architecture landed together.
- **Keywords:** spring animation engine, spawn wizard, `huh` forms, 6-panel dashboard
- **Seen:** 5840f61

### five-competing-web-ui-design-proposals
- **What:** Five independently-drafted, unimplemented plans for adding a browser-based Web UI on top of NTM's REST/WebSocket API, each proposing distinct visual/interaction concepts.
- **Where:** the five web-UI plan variants (`docs/planning/PLAN_TO_ADD_WEB_UI_AND_REST_AND_WEBSOCKET_API_LAYERS_TO_NTM__CLAUDE_WEB.md`, `docs/planning/PLAN_TO_ADD_WEB_UI_AND_REST_AND_WEBSOCKET_API_LAYERS_TO_NTM__GEMINI.md`, `docs/planning/PLAN_TO_ADD_WEB_UI_AND_REST_AND_WEBSOCKET_API_LAYERS_TO_NTM__GPT.md`, `docs/planning/PLAN_TO_ADD_WEB_UI_AND_REST_AND_WEBSOCKET_API_LAYERS_TO_NTM__GPT_PRO.md`, `docs/planning/PLAN_TO_ADD_WEB_UI_AND_REST_AND_WEBSOCKET_API_LAYERS_TO_NTM__OPUS.md`)
- **Notable:** Named signature concepts across the variants: "Flywheel Gateway"/"Command Kernel" branding + a "Galaxy View" React-Flow dependency graph (red=bottleneck, gold=keystone nodes) + mobile "Read Mode" (GEMINI); Session Cards/Agent Grid/Pane Stream Viewer/Conflict Heatmap (files×agents grid) with numeric perf budgets <100ms interaction/60fps scroll (GPT_PRO); fixed per-agent-type accent colors (Claude=mauve, Codex=blue, Gemini=yellow, User=green) via Catppuccin Mocha/Latte, `cmdk` command palette, mobile "Prompt Sheet" draggable bottom sheet (CLAUDE_WEB); full 8-tool "Agent Flywheel" UI with per-tool "Decks" and role-based RBAC viewer/operator/admin (OPUS, the explicit synthesis of the other plans). All 5 converge on Chi+gorilla-websocket backend / Next.js16+React19+TanStack+Tailwind4+Framer Motion frontend, but diverge on envelope shape, approval-required HTTP status code (428 vs 409), and WS multiplexing model (stream-field vs. topic-string).
- **Keywords:** Galaxy View, Conflict Heatmap, per-agent accent colors, Decks
- **Seen:** 5840f61

## testing-evals

### deterministic-fault-injection-harness
- **What:** A small, deterministic test harness for injecting faults into orchestration logic without relying on real agent CLIs.
- **Where:** `internal/faultharness` ("a small, deterministic test harness")
- **Notable:** Directly supports the "no-real-model mandate" documented in `docs/verification-matrix-swarm-scale-vnext.md`, where `short`/`race` CI gates must run without invoking any real Claude/Codex/Antigravity/Gemini CLI for reproducibility.
- **Keywords:** fault injection, deterministic harness, no-real-model mandate
- **Seen:** 5840f61

### reservation-contention-simulator
- **What:** See [[orchestration]] `file-reservation-simulation-and-conflict-detection` — the same simulator functions as a testing/eval tool for validating coordination logic under synthetic contention without a live Agent Mail service.
- **Where:** `internal/reservationsim`
- **Keywords:** contention simulation, offline coordination testing
- **Seen:** 5840f61

### synthetic-swarm-scale-testing
- **What:** An opt-in, large-scale synthetic swarm test mode (100+ simulated panes) gated behind an environment variable and minimum host specs.
- **Where:** `docs/verification-matrix-swarm-scale-vnext.md`
- **Notable:** `NTM_SWARM_LOAD=1` gate, requires ≥32 cores/64GB RAM; produces a fixed artifact set per run (`latency.json` p50/p95/p99/max, `mem.json`, `goroutines.json` baseline/peak/leaked, `contract.json` robot-surface failures, `timeline.jsonl` causality replay with explicit tie-break rule `(timestamp, source, event_id)`).
- **Keywords:** `NTM_SWARM_LOAD`, causality timeline, tie-break rule
- **Seen:** 5840f61

### e2e-live-agent-test-suite
- **What:** A separate end-to-end test tree that exercises real agent CLI behavior (as opposed to the synthetic/deterministic gates above), run only when live agents are available.
- **Where:** `e2e/` (confirmed present; sample files: `activity_metrics_test.go`, `atomic_assignment_cli_e2e_test.go`, `auto_respawner_test.go`, `canonical_pane_contract_test.go`, `checkpoint_test.go`, `codex_goal_send_test.go`, `cost_tracking_test.go`, `doctor_test.go`), `Makefile` (`test-e2e` target: 10-minute timeout, `-tags=e2e`)
- **Notable:** `make test` (`go test -v -short ./...`) explicitly skips this tree; `make test-all` (`-tags=e2e`) and `make test-e2e` opt back in — a documented three-tier test-execution split (short/all/e2e-only).
- **Keywords:** `e2e/` build tag, `test-e2e` Makefile target, live-agent requirement
- **Seen:** 5840f61

### coverage-boosting-test-debt
- **What:** The self-audit report flags a specific quantity of test files explicitly named/created just to raise coverage numbers rather than test real behavior.
- **Where:** `docs/planning/MODES_OF_REASONING_REPORT_AND_ANALYSIS_OF_PROJECT.md`
- **Notable:** Report states: "12,744 test functions exist but 'the build is broken on main'; 60 files explicitly named for 'coverage boosting.'" — flagged as a divergent finding between reasoning modes (test *count* vs. test *effectiveness* disagree sharply).
- **Keywords:** coverage-boosting files, test count vs. effectiveness divergence
- **Seen:** 5840f61

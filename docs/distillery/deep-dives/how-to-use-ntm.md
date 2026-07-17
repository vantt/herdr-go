---
topic: how-to-use-ntm
date: 2026-07-15
based_on: [ntm@5840f61]
entries: [ntm:tmux-swarm-session-model, ntm:robot-mode-operator-loop, ntm:robot-flag-command-surface, ntm:ntm-ships-its-own-skill-file, ntm:agent-activity-detection, ntm:pipeline-execution-engine]
---

# How to use ntm (deeply)

**Bottom line:** ntm ("Named Tmux Manager") is a single Go binary that turns `tmux` into a control plane for running several AI coding-agent CLIs in parallel, wraps that with work triage (`br`/`bv`), coordination (Agent Mail), safety policy, durable state, and a machine-readable `--robot-*`/REST/SSE/WS surface. **Only `tmux` is a hard requirement**; everything else (agent CLIs, `br`/`bv`, Agent Mail, `cass`/`cm`, `ubs`, `rch`) is a companion tool that ntm degrades gracefully without. Install with the one-line installer, add the shell-eval line, run `ntm deps -v`, then `ntm spawn <project> --cc=N --cod=N --agy=N` to get a labeled swarm inside one tmux session. **Layout is 100% delegated to tmux** — ntm does not reflow panes; it names sessions/panes and gives you a dashboard/palette on top. The single mental model to hold: **the calling LLM (you, or an agent) is the driver; ntm is the nervous system** — it senses (activity/health/triage) and actuates (send/spawn/interrupt) but never plans or schedules on its own.

## 0. Install & prerequisites

```bash
# Install (resolves platform/version/asset, verifies SHA256 before trusting the download)
curl -fsSL "https://raw.githubusercontent.com/Dicklesworthstone/ntm/main/install.sh?$(date +%s)" | bash -s -- --easy-mode
# --easy-mode silently appends PATH/shell-eval lines to your rc file; default mode only prints instructions.

# Alternatives
brew install dicklesworthstone/tap/ntm
docker build -t ntm . && docker run --rm -it ntm     # Docker image build step exists but is TODO'd out upstream — verify locally
go install ./cmd/ntm                                  # from source, go.mod pins Go 1.26.3+

# Shell integration (adds completions + `ntm shell` hook, zoxide/starship-style)
eval "$(ntm shell zsh)"     # bash/fish also supported; legacy `ntm init <shell>` auto-upgrades with an rc backup

# Verify what ntm actually sees
ntm deps -v
```

**Requirement tiers (be honest about adoption cost):**

| Tier | Tool | What you lose without it |
|---|---|---|
| **Required** | `tmux` | Nothing works — ntm has no non-tmux backend. |
| **Required for spawning** | Claude Code, Codex CLI, Antigravity CLI (`agy`); Gemini CLI supported as legacy | `ntm spawn` can only launch CLIs it finds on `PATH` at the time it starts — see the PATH gotcha below. |
| **Optional — work intelligence** | `br` (Beads issue tracker), `bv` (beads_viewer graph engine) | `ntm work ...`/`ntm assign` fall back to "nothing useful to say"; you lose triage, impact analysis, and queue-dry ideation. |
| **Optional — coordination** | MCP Agent Mail | `ntm mail`/`ntm locks`/`ntm coordinator` report the server unavailable; session orchestration still works. |
| **Optional — memory/search** | `cass`, `cm` (cass-memory) | No cross-agent history search, no auto-injected context, no reflective playbook. |
| **Optional — quality gates** | `ubs` (Ultimate Bug Scanner), `rch` (Remote Compilation Helper) | No mandatory pre-commit scan; builds run locally instead of on a remote worker fleet (rch fails open anyway). |
| **Optional — misc** | `dcg`, `pt`, `fzf` (recommended by packaging) | Destructive-command-guard bridge and a couple of palette conveniences degrade. |

**A real PATH gotcha:** ntm discovers agent CLIs via the `PATH` of the runtime environment it is *launched in*, not your interactive login shell's `PATH`. A bare SSH command, a detached tmux server, or a systemd unit won't source `~/.zshrc`, so `ntm deps -v` can report `claude`/`codex`/`agy` missing even though they work fine in your terminal. Fix with a wrapper that exports `PATH` before `exec ntm "$@"` (README Troubleshooting section has the exact snippet).

**Directories created:**

| Scope | Path | Contents |
|---|---|---|
| User | `~/.config/ntm/config.toml` | Main config (TOML) |
| User | `~/.config/ntm/recipes.toml`, `~/.config/ntm/workflows/`, `~/.config/ntm/personas.toml`, `~/.config/ntm/templates/` | Reusable session/prompt assets |
| User | `~/.ntm/policy.yaml` | Destructive-command policy tiers |
| Project | `.ntm/workflows/`, `.ntm/pipelines/`, `.ntm/templates/`, `.ntm/personas.toml`, `.ntm/recipes.toml`, `.ntm/checkpoints/`, `.ntm/claims/` | Project-local overrides; resolved **after** user-level, so project wins |
| Robot state | `~/.config/ntm/state.db` | Single SQLite file — durable events, watermarks, runtime projection cache (shared, not per-project) |

## 1. Mental model

- **tmux swarm sessions**: `ntm spawn <project>` creates one named tmux session containing a user pane plus one pane per agent (`project__cc_1`, `project__cod_1`, ...). `--label` lets multiple coordinated swarms share one project directory (`project--frontend`, `project--backend`).
- **The operator loop** (same shape whether you or an agent drives it): BOOTSTRAP → SUMMARIZE → REPLAY → TRIAGE → INSPECT → ACT → WAIT → REPEAT. Each stage maps to exactly one canonical robot surface (§B2).
- **Robot-vs-human surfaces**: `ntm dashboard`, `ntm palette`, and other TUI/Bubble Tea surfaces are for humans. `--robot-*` flags, `ntm serve`'s REST/SSE/WS, and plain non-interactive commands (`ntm send`, `ntm work triage`, `ntm locks list`) are for scripts and agents. Never point automation at the TUI.

## Part A — Operator mastery (you, the human)

### A1. Spawning & session model

```bash
ntm quick payments --template=go              # scaffold a project dir NTM can resolve
ntm spawn payments --cc=3 --cod=2 --agy=1     # mixed Claude/Codex/Antigravity swarm, one call
ntm spawn payments --label backend --cc=2 --worktrees   # isolation-first: per-agent git worktree/branch
ntm spawn payments --label frontend --cc=2    # second coordinated swarm, same project dir
ntm add payments --cc=1                       # add an agent to an existing session
ntm add payments --label frontend --cod=1

ntm list                                      # all sessions
ntm status payments                           # session state
ntm view payments                             # quick tile + attach
ntm zoom payments 3                           # zoom pane 3
ntm attach payments                            # raw tmux attach
ntm kill payments                             # tear down
```

Useful spawn modifiers: `--no-user` (headless), `--prompt "..."` (initial prompt to all agents), `-r <recipe>` / `-t <template>` (reusable presets), `--persona=architect --persona=implementer:2`, `--stagger` / `--stagger-mode=smart` (thundering-herd prevention, default 90s — see §B1), `--assign-work --assign-strategy=diverse` (orchestrator claims beads before prompting, avoiding self-selection races entirely).

### A2. Layout — be honest: layout is tmux

ntm does **not** implement its own pane layout engine. Splits, resizing, zoom-and-restore, and copy-mode are all native tmux behavior — `ntm zoom`/`ntm view` are thin wrappers around `tmux resize-pane -Z` and pane selection. What ntm actually adds on top of raw tmux:

- **`ntm dashboard`** — a Bubble Tea TUI live overview (sessions, activity, history, focus) for a human tending a swarm manually. Not for automation.
- **`ntm palette`** — a fuzzy-searchable prompt launcher (`command_palette.md`), bindable to a tmux popup key via `ntm bind` (default F6).
- **`ntm spawn` wizard** — a guided form-based flow (Charmbracelet `huh`) for interactively composing a swarm instead of typing flags.

If you want a different pane arrangement, you use tmux keybindings directly (or `.tmux.conf`) — ntm has no `--layout=` flag of its own.

### A3. Sending work & waiting

```bash
ntm send payments --cc "Review the API design"
ntm send payments --all "Checkpoint and summarize current state"
ntm send payments --pane=2 "You own the auth migration."
ntm send payments --smart --route=affinity "Continue the migration work"     # score-routed to best agent
ntm send payments --distribute --dist-strategy=dependency                    # bulk-assign across panes
ntm send payments -c internal/auth/service.go "Refactor this safely"         # file context
ntm send payments -t fix --var issue="nil pointer" --file internal/auth/service.go
ntm send payments --batch prompts.txt --delay=5s

ntm interrupt payments                    # Ctrl+C, optionally with a new task
ntm wait payments --until=idle --timeout=5m
ntm wait payments --pane=1 --until=idle --timeout=3m

ntm checkpoint save payments -m "before auth refactor"     # routine cadence, not just disaster recovery
ntm checkpoint list payments
ntm checkpoint restore payments

ntm timeline show <session-id>
ntm history search "authentication error"
```

Smart routing score: `context_score*0.4 + state_score*0.4 + recency_score*0.2` (context_score = 100 − context-usage%; state_score WAITING=100/THINKING=50/GENERATING=0/ERROR=−100). Unhealthy agents score −100 (excluded); rate-limited score −50.

### A4. Remote & API — what "remote control" actually means here

`ntm serve --port 7337` starts a local HTTP server: REST under `/api/v1`, Server-Sent Events at `/events`, WebSocket at `/ws`, health at `/health`, generated OpenAPI at `docs/openapi.json` (regenerate with `ntm openapi generate`). **This is the only sense in which ntm is "remote-controllable"** — it is a local API surface a dashboard/script/other-machine can hit over HTTP, not an interactive remote-attach mechanism. To interactively attach to the tmux session itself you still need `tmux attach` / `ssh` — `ntm serve` does not tunnel a terminal.

Two OpenAPI documents exist and are **not** a strict subset of each other despite sharing the title "NTM REST API": `docs/openapi.json` (214 paths / 101 schemas, generated from `docs/parity_matrix.json`, versioned `0.1.0-draft`) targets full CLI/TUI/robot parity; `docs/openapi-kernel.json` (13 paths, versioned `dev`) is hand-scoped from the smaller unified command-registry (`internal/kernel`). Check both if you're integrating — don't assume one implies the other.

### A5. Config & workflows

Four stackable, distinct reuse layers, each resolved user-level then project-level (project wins):

| Layer | Purpose | User path | Project path |
|---|---|---|---|
| `recipes` | Session presets (agent counts/types) | `~/.config/ntm/recipes.toml` | `.ntm/recipes.toml` |
| `workflows` | Orchestration patterns (pipeline, ping-pong, review-gate) | `~/.config/ntm/workflows/` | `.ntm/workflows/` |
| `template` | Prompt templates + `--var` substitutions | `~/.config/ntm/templates/` | `.ntm/templates/` |
| `session-templates` | Higher-level session layouts | — | — |

```bash
ntm recipes list ; ntm recipes show full-stack
ntm workflows list ; ntm workflows show red-green
ntm template list ; ntm template show fix-bug
ntm session-templates list ; ntm session-templates show refactor

ntm config init | show | diff | get projects_base | edit | reset
```

`command_palette.md` is a separately-versioned prompt library (`## Category` / `### command_key | Display Label`) surfaced by `ntm palette` — categories include Analysis & Review, Coding & Development, Ensemble, Planning & Workflow, Agent Coordination.

**"Landing the plane"** — the mandated end-of-session closeout (from `AGENTS.md`): file issues for remaining work → run quality gates → update issue status → `br sync --flush-only` → hand off context. Pairs with the Beads Session Protocol: `git status` → stage code → `br sync --flush-only` → stage `.beads/` → commit → push (`br` never runs git itself).

## Part B — Agent-driving mastery (AI agents driving ntm, and ntm understanding agents)

### B1. How ntm understands & manages agents

**Activity detection** (`internal/agent`, `internal/status`, `internal/completion`): classifies each pane by output velocity (characters/sec) plus a per-CLI regex pattern library into one of 6 states — `GENERATING`/`WAITING`/`THINKING`/`ERROR`/`STALLED`/`UNKNOWN` — each carrying a confidence score (ERROR=0.95). Velocity thresholds: `>10 c/s` → generating, `0 c/s` → idle-or-stalled (disambiguated by other signals). **State transition hysteresis** requires 2s stability before switching state, except ERROR which transitions immediately. Separate packages classify each CLI's on-screen state (`internal/codex`, `internal/gemini`, `internal/agent`), including CLI-specific auth-challenge detection (`internal/auth`).

```bash
ntm activity payments --watch
ntm --robot-activity=payments --panes=1,2 --type=claude
```

**Health & resilience** (`internal/health`, `internal/resilience`): states `healthy`/`degraded`/`unhealthy`/`rate_limited`. Soft restart (Ctrl+C) attempted before hard restart (kill+relaunch); backoff `base * 2^restarts` capped at `max_backoff`; rate-limit backoff `30s * 2^(consecutive_rate_limits-1)` capped at 5m. PID-based liveness checks (not text heuristics, since v1.13.0).

```bash
ntm health payments ; ntm health payments --verbose
ntm --robot-health=payments
```

**Context-window rotation** (`internal/context`, `internal/handoff`): seamless agent rotation when a pane's context fills — token monitoring, handoff summary, rotation audit log. **Account rotation** (`internal/rotation`, `ntm rotate`): auto-triggers on rate limits via a `Provider` interface (`Name`/`LoginCommand`/`ExitCommand`/`AuthSuccessPatterns`) with per-swarm model-setting snapshot/restore so overrides don't leak into your global Claude Code config.

**Thundering-herd prevention** (`internal/scheduler`): staggers agent starts (default 90s, justified by a typical 60–90s agent bootstrap sequence) so agents self-selecting work via `bv --robot-triage` don't race to claim the same bead. `NTM_SPAWN_ORDER`/`NTM_SPAWN_TOTAL`/`NTM_SPAWN_BATCH_ID` env vars are set per agent. Alternative: `--assign-work --assign-strategy=diverse` has ntm claim beads before prompting, removing the race entirely.

### B2. Robot mode — the operator loop and flag surface

The canonical 8-step loop, each stage owned by exactly one surface (any overlap is a documented anti-goal — see `docs/robot-surface-taxonomy.md`):

```
1. BOOTSTRAP  --robot-snapshot         establish baseline state + cursor (NOT for frequent polling)
2. SUMMARIZE  --robot-status          cheap high-level view, <100ms target, poll this often
3. REPLAY     --robot-events          raw event stream since cursor
4. TRIAGE     --robot-attention (blocking) / --robot-digest (non-blocking)
5. INSPECT    --robot-tail / --robot-inspect-pane / --robot-diagnose / --robot-context / --robot-activity / --robot-files / --robot-diff
6. ACT        --robot-send / --robot-spawn / --robot-interrupt / --robot-assign / --robot-route / --robot-restart-pane / --robot-overlay
7. WAIT       --robot-wait            block until an enumerated condition or timeout
8. REPEAT     → back to REPLAY, or BOOTSTRAP on CURSOR_EXPIRED
```

Start every session with:

```bash
ntm --robot-help ; ntm --robot-capabilities ; ntm --robot-status ; ntm --robot-snapshot
ntm --robot-plan ; ntm --robot-dashboard ; ntm --robot-markdown --compact ; ntm --robot-terse
```

**Flag naming convention** (~143 `--robot-*` flags found in `internal/cli/root.go`, all registered as bare `rootCmd.Flags()`, not subcommand flags):
- Session-scoped: `--robot-<verb>=SESSION` (e.g. `--robot-send=payments`, `--robot-tail=payments`).
- Global: bare bool flag (e.g. `--robot-status`, `--robot-snapshot`).
- Tool bridges: `--robot-<tool>-<action>` bool/string flag + **separate** unprefixed modifier flags (`--limit`, `--query`, `--since`), not inline values — `--robot-jfp-search="x"` inline form is deprecated in favor of `--robot-jfp-search --query="x"`.
- Shared modifiers are unprefixed and reused across many commands: `--limit`, `--offset`, `--since`, `--panes`, `--all`, `--lines`, `--query`, `--type`, `--timeout`, `--verbose`, `--dry-run`, `--strategy`, `--exclude`, `--session`. A large deprecated→canonical mapping table exists (e.g. `--cass-limit`→`--limit`, `--wait-timeout`→`--timeout`, `--md-compact`→`--compact`) — old prefixed flags still work but emit deprecation warnings via cobra `MarkDeprecated`.

**Output envelope** — every robot response includes `success`, `timestamp`, and (on error) `error`/`error_code`/`hint`; critical arrays (`sessions`, `agents`, `messages`) are always present, empty `[]` if none — never null or absent. Optional `_agent_hints` object carries `summary`, `suggested_action`, `safer_alternative`, `warnings`, and pagination hints (`next_offset`). Exit codes: `0` success, `1` error, `2` unavailable/`NOT_IMPLEMENTED` (also used for `TOOL_NOT_FOUND`). Standard error codes: `SESSION_NOT_FOUND`, `PANE_NOT_FOUND`, `TOOL_NOT_FOUND`, `AGENT_NOT_FOUND`, `INVALID_FLAG`, `INVALID_INPUT`, `MISSING_REQUIRED`, `TIMEOUT`, `NOT_IMPLEMENTED`, `PERMISSION_DENIED`, `INTERNAL_ERROR`.

**Wait conditions** (enumerated, no free-form predicates): pane-oriented `idle`/`complete`/`generating`/`healthy`/`stalled`/`rate_limited`; attention-oriented `attention`/`action_required`/`mail_pending`/`mail_ack_required`/`context_hot`/`reservation_conflict`/`file_conflict`/`session_changed`/`pane_changed`. `bead_orphaned` is deliberately unsupported.

```bash
ntm --robot-wait=payments --condition=idle --timeout=5m
ntm --robot-send=payments --msg="Summarize blockers" --type=claude
ntm --robot-ack=payments --timeout=30s
ntm --robot-mail-check --mail-project=payments --urgent-only
ntm --robot-cass-search --query="authentication error"
```

**Idempotency & request identity** (docs-level; verify current implementation before relying on it): request IDs of the form `req_<timestamp>_<random>`, scoped idempotency (`request`/`session`/`persistent`/`indefinite`), `--robot-action-status --request-id=...` for polling completion instead of blind retries. `spawn` is explicitly flagged as "duplicate session possible" without an idempotency key.

### B3. Driving ntm from an agent — the SKILL.md contract

`SKILL.md` (272 lines, at the repo root) is written specifically for an AI coding agent consuming ntm as a Claude Code skill; it front-loads the interactive-vs-robot split, then defers deeper detail to a lazy-loaded reference index rather than one giant file: `references/COMMANDS.md` (dense command patterns), `references/ROBOT-MODE.md` (attention feed, wait conditions, mail/cass/bead robot flows), `references/DASHBOARD.md` (human TUI only), `references/CONFIG.md` (project resolution). Its own routing rule: **"Read the repo first"** — a target repo's own `AGENTS.md`/`README.md` overrides generic ntm advice.

Hard do's/don'ts baked into `AGENTS.md`/`SKILL.md` that an agent must follow when driving ntm:
- **Never run bare `bv`** — it launches a blocking interactive TUI. Only `bv --robot-*` flags. `bv --robot-triage` is the single entry point (returns `quick_ref`, ranked `recommendations`, `quick_wins`, `blockers_to_clear`, `project_health`, copy-paste `commands`).
- **Never run bare `cass`** — same reason. Always `cass ... --robot`/`--json`.
- **Never edit `.beads/*.jsonl` directly** — only through `br`. `br` never runs git commands itself; `git add .beads/ && git commit` is manual, always.
- **Register with Agent Mail and reserve files before editing** (`ensure_project` → `register_agent` → `file_reservation_paths(...)` → `send_message`/`fetch_inbox`/`acknowledge_message`); prefer the 4 macro helpers (`macro_start_session`, `macro_prepare_thread`, `macro_file_reservation_cycle`, `macro_contact_handshake`) over granular calls for speed.
- **Beads issue ID doubles as Agent Mail `thread_id`** and message-subject prefix (`[br-123] ...`) — one identifier unifies both systems.
- **Checkpoint routinely**, not just before disasters: after a spawn/restore's prompts are confirmed received, after root-cause isolation, before risky edits, after significant uncommitted work but before verification, after green verification, before merge/handoff.
- **Work triage**: `ntm work triage/next/graph/impact` wraps `bv` in operator-friendly form; use raw `bv --robot-*` when you specifically want the graph engine's native output. Queue-dry flow is gated: confirm `br ready --json` / `bv --robot-triage` show nothing actionable *first*, then `ntm work queue-dry --format=json`, then only `--ideate` (preview-only until `--create-beads --yes --plan-version=<git-sha>`).
- **Coordination isolation choice**: Agent Mail reservations are the default primitive; `--worktrees` / `ntm worktrees merge <pane>` are an alternative isolation-first mode when repo policy allows it — a target repo's `AGENTS.md` can mandate one over the other.

### B4. Extending

- **Plugin SDK** (`internal/plugins`): external binaries register as additional `ntm` CLI subcommands, loaded dynamically at startup via `plugins.LoadCommandPlugins(cmdDir)` in `root.go` alongside the ~90 statically-registered commands; each plugin becomes a `cobra.Command` shelling out to the plugin's `Execute(args, env)`.
- **Tool adapter framework** (`internal/tools`, `internal/integrations`): a unified adapter interface for integrating external ecosystem tools beyond the agent CLIs themselves, added alongside the Daemon Supervisor and Durable State Store in v1.4.0.

## Part C — Limits, gotchas, real adoption cost

- **tmux-bound by design.** ntm is not a fleet manager — `docs/robot-surface-taxonomy.md` explicitly rejects a "fleet-first worldview": it manages named sessions in a single tmux instance on one machine. Two independent, competing, **unimplemented** specs (`PROPOSED_HYPERSYNC_SPEC__CODEX.md` / `__OPUS.md`) describe a FUSE-based multi-machine filesystem-sync layer that would change this — they contradict each other on partition handling and are not built.
- **Companion-tool sprawl is real.** Full value requires `tmux` + an agent CLI + `br` + `bv` + Agent Mail + `cass`/`cm` + `ubs`/`rch` — 7+ separate external processes/services, each with its own install and failure mode. ntm degrades gracefully (marks sources `_degraded` rather than crashing) but the *headline* feature set (triage, coordination, memory) is inert without them.
- **Self-described "early development, no users."** `AGENTS.md`/`docs/robot-api-design.md` state directly: "We do not care about backwards compatibility... NO TECH DEBT." Schemas and flags can break without a deprecation period; pin to a commit if you depend on exact behavior.
- **God packages.** A self-audit report (`docs/planning/MODES_OF_REASONING_REPORT_AND_ANALYSIS_OF_PROJECT.md`) flags `internal/cli` (~75K LOC/233 files) and `internal/robot` (~62K LOC/87 files, with `robot.go` alone 10,717 lines/288 functions and 227 `os.Exit()` calls) as disproportionate concentrations of logic, despite the overall package DAG being cleanly acyclic (import depth 14).
- **Build/test-health claim, not independently verified here.** The same self-audit report states "12,744 test functions exist but the build is broken on main," with 60 files explicitly named for "coverage boosting" rather than real behavior testing — flagged as a divergent finding between reasoning modes, not something this guide re-ran `go build`/`go test` to confirm. Treat as a documented risk signal to check yourself before depending on `main`.
- **Advertised-but-unimplemented / docs-only:**
  - **HyperSync** (multi-machine FUSE sync) — two competing spec docs, zero implementation.
  - **Web UI** — five independently-drafted, unimplemented browser-UI proposals on top of the (implemented) REST/WS API; only the backend exists today, not a shipped web frontend.
  - **`docs/schemas/{robot,sections,envelope}/*.v1.json`** — a documented hand-curated JSON-Schema convention that does not exist on disk yet.
  - **TOON `config.toml [robot.output] format`** — the config key is documented but not wired into `resolveRobotFormat()`; only the CLI flag/env var actually select TOON output.
- **Internally-contradictory "ratified" specs.** Three SQLite runtime-schema docs and two robot error-taxonomy docs are each marked RATIFIED under the same bead ID but disagree with each other (table-name prefixes, cursor types, error-code vocabularies) — a sign some subsystems are documentation-heavy relative to implementation certainty.
- **Documented, unremediated security findings** (self-audit, not verified independently here): `handlePaneInputV1` reportedly passes arbitrary JSON-body text to `tmux.SendKeys` with no session-name validation or policy check in default unauthenticated-local mode; any admin-level local caller can `PATCH allowed_origins` to `*`; the two-person (SLB) approval check can self-approve trivially in local anonymous-auth mode (`"unknown"=="unknown"`). If you expose `ntm serve` beyond localhost, treat these as open questions to verify, not settled.
- **Linux/macOS only**, local-first — not a hosted SaaS control plane.

## Appendix — complete command reference

### Top-level commands (registered in `internal/cli/root.go:4444`, ~90 statically-registered + dynamically-loaded plugins)

| Group (per README Command Map) | Commands |
|---|---|
| Project bootstrap & session lifecycle | `quick`, `init`, `spawn`, `add`, `attach`, `view`, `zoom`, `dashboard`, `palette`, `kill`, `create`, `adopt`, `swarm` |
| Day-to-day operator loop | `send`, `interrupt`, `watch`, `activity`, `health`, `extract`, `diff`, `grep`, `analytics`, `copy`, `save`, `search`, `errors`, `changes`, `conflicts`, `summary`, `logs`, `preflight`, `replay` |
| Graph-aware prioritization/assignment | `work`, `assign`, `coordinator`, `rebalance`, `review-queue`, `scale`, `controller` |
| Coordination & reservations | `mail`, `locks`, `lock`, `unlock`, `message`, `worktrees`, `worktree` |
| Safety & approvals | `safety`, `policy`, `approve`, `guards`, `hooks` |
| Durable state & forensics | `checkpoint`, `rollback`, `timeline`, `history`, `audit`, `resume`, `handoff` |
| Reusable orchestration assets | `recipes`, `workflows`, `template`, `session-templates`, `pipeline`, `ensemble`, `modes`, `personas`, `profiles` |
| Integration, config, ops | `serve`, `openapi`, `config`, `deps`, `upgrade`, `tutorial`, `kernel`, `plugins`, `bind` |
| Agents & models | `agents`, `models`, `codex`, `rotate`, `quota` |
| Quality/diagnostics | `scan`, `scrub`, `redact`, `bugs`, `cass`, `doctor`, `cleanup`, `support-bundle`, `metrics` |
| Git & repo | `git`, `repo` |
| Navigation | `list`, `status` |
| Memory/context | `memory`, `context` |
| Beads daemon | `beads` |
| Shell/version | `shell`, `completion`, `version`, `level` |

`ntm --help` remains the canonical live reference; the table above enumerates every `Use:` string found across `internal/cli/*.go` (non-test) plus the `rootCmd.AddCommand(...)` block, cross-checked against README's own Command Map.

### `--robot-*` flag families (143 flags grounded in `internal/cli/root.go`)

| Family | Representative flags |
|---|---|
| Discovery | `--robot-help`, `--robot-capabilities`, `--robot-version`, `--robot-tools`, `--robot-docs`, `--robot-schema`, `--robot-recipes`, `--robot-default-prompts` |
| Bootstrap/Summarize/Replay | `--robot-snapshot`, `--robot-status`, `--robot-events` |
| Triage (attention) | `--robot-attention`, `--robot-digest`, `--robot-dismiss-alert` |
| Inspect | `--robot-tail`, `--robot-inspect-pane`, `--robot-inspect-agent`, `--robot-inspect-session`, `--robot-inspect-work`, `--robot-inspect-coordination`, `--robot-inspect-incident`, `--robot-inspect-quota`, `--robot-diagnose`, `--robot-context`, `--robot-activity`, `--robot-files`, `--robot-diff`, `--robot-causality`, `--robot-errors`, `--robot-logs`, `--robot-monitor` |
| Act | `--robot-send`, `--robot-spawn`, `--robot-interrupt`, `--robot-assign`, `--robot-bulk-assign`, `--robot-route`, `--robot-restart-pane`, `--robot-overlay`, `--robot-controller-spawn`, `--robot-context-inject` |
| Wait | `--robot-wait` |
| Beads | `--robot-beads-list`, `--robot-bead-show`, `--robot-bead-claim`, `--robot-bead-close`, `--robot-bead-create`, `--robot-watch-bead` |
| BV / graph triage | `--robot-triage`, `--robot-plan`, `--robot-graph`, `--robot-suggest`, `--robot-forecast`, `--robot-impact`, `--robot-search`, `--robot-label-health`, `--robot-label-flow`, `--robot-label-attention`, `--robot-file-beads`, `--robot-file-hotspots`, `--robot-file-relations` |
| CASS | `--robot-cass-search`, `--robot-cass-context`, `--robot-cass-status`, `--robot-cass-insights` |
| Ensemble | `--robot-ensemble`, `--robot-ensemble-spawn`, `--robot-ensemble-stop`, `--robot-ensemble-modes`, `--robot-ensemble-presets`, `--robot-ensemble-suggest` |
| Pipeline | `--robot-pipeline`, `--robot-pipeline-run`, `--robot-pipeline-status` (via `--robot-pipeline`), `--robot-pipeline-list`, `--robot-pipeline-cancel` |
| Health/agents | `--robot-health`, `--robot-agent-health`, `--robot-health-oauth`, `--robot-health-restart-stuck`, `--robot-smart-restart`, `--robot-is-working`, `--robot-agent-names` |
| Accounts/quota | `--robot-account-status`, `--robot-accounts-list`, `--robot-quota-check`, `--robot-quota-status`, `--robot-switch-account` |
| Tool bridges (external, degrade to `DEPENDENCY_MISSING`) | `--robot-jfp-*` (12 flags: status/list/search/show/suggest/install/export/update/installed/categories/tags/bundles), `--robot-acfs-status` (alias `--robot-setup`), `--robot-ms-search`/`-show`, `--robot-dcg-check`/`-status`, `--robot-slb-pending`/`-approve`/`-deny`, `--robot-ru-sync`, `--robot-giil-fetch`, `--robot-xf-search`/`-status`, `--robot-rch-status`/`-workers`, `--robot-rano-stats`, `--robot-proxy-status` |
| Persistence | `--robot-save`, `--robot-restore`, `--robot-replay` |
| Formatting/output | `--robot-terse`, `--robot-markdown`, `--robot-dashboard`, `--robot-format`, `--robot-output-format`, `--robot-verbosity` |
| Misc | `--robot-guard`, `--robot-mail`, `--robot-mail-check`, `--robot-metrics`, `--robot-summary`, `--robot-tokens`, `--robot-support-bundle`, `--robot-safety-simulate`, `--robot-profile-list`/`-show`, `--robot-env`, `--robot-palette`, `--robot-history` |

**Shared unprefixed modifiers** (reused across the families above, canonical per `docs/robot-api-design.md`): `--limit`, `--offset`, `--since`, `--until`, `--panes`, `--all`, `--lines`, `--query`, `--type`, `--timeout`, `--verbose`, `--dry-run`, `--strategy`, `--exclude`, `--session`, `--track`, `--condition` (alias of `--wait-until`). A large deprecated→canonical alias table lives in `docs/robot-api-design.md` §13 (e.g. `--cass-limit`→`--limit`, `--ack-timeout`→`--timeout`, `--md-compact`→`--compact`) and is mechanically enforced with `cobra.Flags().MarkDeprecated(...)` calls in `root.go` (~60 `MarkDeprecated` calls confirmed).

## Coverage notes

**Read in full:** `README.md` (637 lines), `SKILL.md` (272), `AGENTS.md` (913), `references/COMMANDS.md`, `references/ROBOT-MODE.md`, `references/DASHBOARD.md`, `references/CONFIG.md`, `command_palette.md`, `docs/ORCHESTRATION_FEATURES.md` (1202), `docs/WORKFLOW_EXAMPLES.md` (823), `docs/robot-api-design.md` (743), `docs/robot-surface-taxonomy.md` (508), the full pre-existing inventory (`docs/distillery/reports/distill-ntm-inventory-2026-07-15.md`) and feature index (`docs/distillery/sources/ntm.md`).

**Code grounded (not fully read, but greped/spot-checked for exact signatures):** `cmd/ntm/main.go` (13 lines, read in full — trivial entry point calling `cli.Execute()`), `internal/cli/root.go` (5714 lines — read the command-registration block at line 4444, the full robot-flag registration region ~3640–4440, and the deprecation-alias block; the remaining ~4500 lines of flag/handler wiring were grepped, not read line-by-line). Confirmed via grep: 143 distinct `--robot-*` flag names registered on `rootCmd.Flags()`, all in `root.go` (no other `internal/cli/*.go` file defines a `robot-*` flag). Confirmed via grep across `internal/cli/*.go`: the full set of cobra `Use:` strings, cross-checked against the README's own "Command Map" table.

**Not independently re-verified in this pass** (stated in ntm's own docs/self-audit report, reported here as documented claims, not something this guide reran): the "build broken on main" / coverage-boosting-test-files claim, the three unremediated security findings in Part C, and the TOON config-wiring gap. A reader adopting ntm for anything beyond solo local use should re-check these against current `main` before trusting them.

**Gaps:** Did not deep-read `internal/robot/robot.go` (10,717 lines) or any other package internals beyond doc comments/grep — per the source inventory's own instruction not to deep-read Go implementation code, and because the docs (`docs/robot-api-design.md`, `docs/robot-surface-taxonomy.md`) are the intended source of truth for the flag *contract*, with `root.go` used only to verify the flag list is exact and current. Did not independently run `go build`/`go test` against this clone to verify the self-reported build-health claim.

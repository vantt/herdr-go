# herdr inventory — distill extraction report

Source: local clone at `upstreams/herdr`, HEAD `a0678a3`.
Method: doc-first read (README/AGENTS/SKILL/CHANGELOG → docs/next website mdx →
src/ module structure and top-of-file doc comments → Cargo/justfile/flake →
scripts/ → tests/ → .github workflows). Mechanical inventory only — no porting
judgment, no cross-source comparison.

## harness

### agent-detection-manifests
- **What:** A TOML rule engine matches a live bottom-of-buffer terminal snapshot (not scrollback) against per-agent manifests to classify `idle`/`working`/`blocked`/`unknown`. Rules combine screen text with optional OSC title/progress evidence as explicit AND/OR gates.
- **Where:** `src/detect/mod.rs`, `src/detect/manifest.rs`, `src/detect/manifests/` (19 per-agent `.toml` files, e.g. `src/detect/manifests/claude.toml`, `src/detect/manifests/codex.toml`)
- **Notable:** `src/detect/mod.rs` states the module contract directly: "Each pane's live bottom-of-buffer text is read periodically and matched against known agent output patterns to determine state." AGENTS.md requires manifest changes to be evidence-based: capture real screen state with `herdr agent read <pane> --source detection --format text` before writing rules, and never match "whole-pane incidental text."
- **Keywords:** screen manifest, bottom-buffer snapshot, detection evidence, AND/OR gates, `agent.explain`

### status-authority-arbitration
- **What:** Each pane has exactly one status authority at a time. Agents with complete lifecycle hooks/plugins (Pi, OMP, Kimi, OpenCode, Kilo, Hermes, MastraCode) become authoritative for `idle`/`working`/`blocked` when actively reporting, disabling screen-manifest fallback for that pane. Agents with only session-identity hooks (Claude Code, Codex, Copilot, Devin, Droid, Qoder, Cursor) always stay on screen-manifest detection because their hooks don't cover every lifecycle transition (e.g. permission cancellations, interrupts).
- **Where:** `docs/next/website/src/content/docs/agents.mdx`, `docs/next/website/src/content/docs/integrations.mdx`
- **Notable:** Explicit two-column authority table ("Lifecycle authority" vs "Session identity") — the design deliberately avoids two competing sources of truth per pane rather than trying to merge/vote between hook and screen signals.
- **Keywords:** state authority, lifecycle hooks, screen manifest fallback, `agent.explain`, `screen_detection_skip_reason`

### agent-state-machine
- **What:** Five semantic states (`idle`, `working`, `blocked`, `done`, `unknown`) where `idle`/`done` are the same underlying finished state distinguished only by whether the result has been "seen" (tab/workspace focus). Rollups propagate blocked/working state from pane → tab → workspace for the sidebar.
- **Where:** `docs/next/website/src/content/docs/concepts.mdx`, `docs/next/website/src/content/docs/agents.mdx`, `SKILL.md`
- **Notable:** SKILL.md spells out the seen/unseen transition precisely: "An agent that first opens at its prompt reports `idle` ... After a working or blocked agent completes, it reports `done` when its tab or workspace is in the background... Focusing a pane... marks the visible tab as seen, so `done` becomes `idle`."
- **Keywords:** idle, working, blocked, done, unknown, state rollup, seen/unseen

### sandbox-agent-hint
- **What:** `HERDR_AGENT=<agent>` env var lets a wrapped/sandboxed process (VM, Bubblewrap, `fence`) tell Herdr which agent manifest to use when the real process is hidden from host `/proc`.
- **Where:** `docs/next/website/src/content/docs/agents.mdx`
- **Notable:** "Set `HERDR_AGENT=claude fence -- claude`... The hint is scoped to that foreground process; avoid exporting it globally."
- **Keywords:** `HERDR_AGENT`, sandbox wrapper, process detection escape hatch

### remote-manifest-hot-reload
- **What:** Bundled manifests can be patched remotely: Herdr checks `herdr.dev/agent-detection/index.toml` for newer per-agent rule files, caches them in the state dir, and hot-reloads the in-memory rule cache without a restart. Local overrides at `~/.config/herdr/agent-detection/<agent>.toml` always win.
- **Where:** `src/detect/manifest_update.rs`, `docs/next/website/src/content/docs/agents.mdx`
- **Notable:** Precedence order is explicit: local override > newer of {cached remote, bundled binary manifest}. `server.agent_manifests` / `herdr server update-agent-manifests` expose the resolution state (`active_version`, `cached_remote_version`, `local_override_shadowing_remote`) for diagnosis.
- **Keywords:** manifest catalog, remote update, hot reload, local override, `agent-manifest_status`

## orchestration

### workspace-tab-pane-model
- **What:** Three-level session hierarchy — workspace (project container) → tab (layout) → pane (real terminal, BSP split tree). IDs are short stable opaque handles (`w1`, `w1:t1`, `w1:p1`) that are never reused or retargeted; closed IDs stay dead and moved panes get new IDs.
- **Where:** `docs/next/website/src/content/docs/concepts.mdx`, `SKILL.md`, `src/workspace.rs`, `src/layout.rs`
- **Notable:** SKILL.md is blunt about the ID contract for agent callers: "Closed tab and pane IDs are not reused and do not retarget later resources... Re-read create, split, move, list, or get responses after mutations; never construct an ID from a workspace or display number."
- **Keywords:** workspace, tab, pane, opaque IDs, BSP layout

### socket-api-control-surface
- **What:** A single local socket (Unix domain socket / Windows named pipe) exposes newline-delimited JSON RPC covering server, workspace, tab, pane, worktree, layout, agent, plugin, and event-subscription methods — the same surface used by the CLI, integrations, and third-party clients.
- **Where:** `src/api/mod.rs`, `src/api/schema/` (per-area schema modules: `workspaces.rs`, `panes.rs`, `agents.rs`, `plugins.rs`, `events.rs`, etc.), `docs/next/website/src/content/docs/socket-api.mdx`
- **Notable:** `herdr api schema --json` prints a full bundled JSON Schema document (`docs/next/api/herdr-api.schema.json`) covering "raw requests, success responses, error responses, emitted events, and subscription events" — the protocol is self-describing for tool authors rather than only documented in prose.
- **Keywords:** socket API, JSON RPC, dot-notation methods, JSON Schema, `session.snapshot`

### worktree-as-workspace
- **What:** Git worktree checkouts are first-class Herdr workspaces with provenance: `worktree.create` makes a checkout (existing branch or new from base/HEAD), opens it as a grouped child workspace; `worktree.open` reattaches; `worktree.remove` runs `git worktree remove` (never deletes the branch) and requires `--force` confirmation on a dirty checkout.
- **Where:** `docs/next/website/src/content/docs/socket-api.mdx`, `docs/next/website/src/content/docs/configuration.mdx` (`[worktrees] directory`), `src/worktree.rs`, `src/app/api/worktrees.rs`, `src/app/api/worktrees/deferred.rs`
- **Notable:** Deleting a worktree is a distinct, explicit action from closing the workspace: "`workspace close` closes Herdr state only. `worktree remove` is the explicit checkout deletion path... requires `--force` when Git refuses a dirty checkout." Generated branch slugs use a small adjective/noun word list (`src/worktree.rs`) rather than random hashes.
- **Keywords:** git worktree, workspace grouping, worktree provenance, branch slug

### wait-primitives
- **What:** Two first-class wait verbs for scripting: `wait output <pane> --match <text|regex>` for generic command completion, and `wait agent-status <pane> --status <state>` for coding-agent lifecycle. Waits match immediately if already true or block for the next transition.
- **Where:** `docs/next/website/src/content/docs/cli-reference.mdx`, `src/api/wait.rs`, `SKILL.md`
- **Notable:** SKILL.md explicitly tells agents to separate the two: "Inspect before waiting. Read current output first, then wait for the next state." and to use `pane run`/`wait output` for ordinary commands vs `agent start`/`wait agent-status` for agent targets.
- **Keywords:** `wait output`, `wait agent-status`, event subscription, blocking CLI wait

### layout-export-apply
- **What:** `layout.export` serializes a tab's BSP split tree (pane/split nodes with direction, ratio, cwd, argv) to a portable JSON tree; `layout.apply` recreates a tab from such a tree (structure/labels/cwd/env/argv only — not live PTYs or scrollback); `layout.set_split_ratio` adjusts an existing split by path.
- **Where:** `docs/next/website/src/content/docs/socket-api.mdx`, `src/layout.rs`
- **Notable:** Explicit non-goal stated in the docs: applying a layout "restores structure, labels, cwd, env, and optional argv commands; it does not preserve live PTYs, scrollback, or running processes" — a declarative topology snapshot, not a process snapshot.
- **Keywords:** BSP tree, `layout.export`, `layout.apply`, portable layout, split ratio

### pane-swap-move-zoom-semantics
- **What:** `pane.swap` (same-tab only, preserves ids/processes/ratios), `pane.move` (cross-tab/workspace, assigns new public pane id on cross-workspace move, preserves the internal terminal), and `pane.zoom` (toggle/on/off with explicit `reason` codes like `already_zoomed`) are each separate RPCs with defined no-op/failure reason enums rather than a single generic "rearrange" call.
- **Where:** `docs/next/website/src/content/docs/socket-api.mdx`
- **Notable:** Responses carry machine-readable `reason` values (`no_neighbor`, `same_pane`, `cross_tab`, `zoomed_tab`, `single_pane`, etc.) so callers can distinguish "nothing to do" from "not found" without string parsing.
- **Keywords:** pane.swap, pane.move, pane.zoom, reason codes, cross-workspace move

### agent-vs-pane-cli-distinction
- **What:** `herdr agent ...` commands are for terminals intentionally treated as agent targets (appear in `agent list`, addressable by agent name, wait-able by agent state); `herdr pane ...` commands are for ordinary terminals/servers/tests/shells. Both operate on the same underlying pane but expose different addressing and semantics.
- **Where:** `docs/next/website/src/content/docs/agents.mdx`, `docs/next/website/src/content/docs/cli-reference.mdx`, `SKILL.md`
- **Notable:** "Use `pane split` and `pane run` for `cargo test`, not `agent start`, unless that terminal is intentionally being treated as an agent target" — a deliberate API-level split between "process I happen to run" and "agent I'm coordinating with."
- **Keywords:** agent start, agent attach, pane split, pane run, agent target

## context-memory

### session-snapshot-restore
- **What:** On full server restart, Herdr restores workspaces/tabs/panes/cwd/layout/focus from `session.json`, but not running processes — restored panes come back as fresh shells in their saved directory unless a stronger restore path (native resume, pane history) applies.
- **Where:** `src/persist.rs`, `src/persist/snapshot.rs`, `src/persist/restore.rs`, `docs/next/website/src/content/docs/session-state.mdx`
- **Notable:** `src/persist.rs` doc comment states storage locations plainly: "Stored at `~/.config/herdr/session.json`. Optional pane screen history is stored separately at `session-history.json`. Installed plugins are persisted separately at `plugins.json`." Snapshot format is explicitly versioned (`SNAPSHOT_VERSION` in `src/persist/snapshot.rs`) to detect incompatible changes.
- **Keywords:** session.json, snapshot restore, versioned snapshot format

### pane-screen-history-replay
- **What:** Opt-in (`[experimental] pane_history = true`) replay of recent terminal contents across a full server restart, stored in `session-history.json`. Off by default because pane output can contain secrets/tokens/prompts.
- **Where:** `docs/next/website/src/content/docs/session-state.mdx`, `src/persist/snapshot.rs` (`PaneHistorySnapshot`)
- **Notable:** The docs explicitly frame this as a security trade-off, not just a feature flag: "Treat the Herdr config/session directory like terminal history."
- **Keywords:** pane screen history, `session-history.json`, opt-in persistence, secrets exposure

### native-agent-session-restore
- **What:** Herdr stores a native session reference (id or path) reported by an official integration hook and, on restart, relaunches the agent's own resume command (e.g. `claude --resume <id>`, `codex resume <id>`, `pi --session <path>`) instead of a plain shell — for 14 different agents, each with its own minimum "integration version" gate.
- **Where:** `src/agent_resume.rs`, `docs/next/website/src/content/docs/session-state.mdx` (full per-agent resume-command table), `docs/next/website/src/content/docs/integrations.mdx`
- **Notable:** `src/agent_resume.rs` caps both session id (`MAX_SESSION_ID_LEN = 512`) and session path (`MAX_SESSION_PATH_LEN = 4096`) length and models the reference as a typed `AgentSessionRefKind::{Id, Path}` rather than an untyped string — restore has a `dedupe_key` on `AgentResumePlan` to avoid double-resuming.
- **Keywords:** agent session resume, native restore, integration version gate, resume command table

### live-handoff
- **What:** Experimental opt-in (`--handoff`) mechanism for update/remote-attach flows that transfers live panes from an old server process to a new one so PTYs and running agent processes survive a server binary swap, distinct from snapshot restore (which only reconstructs after the old server already stopped).
- **Where:** `src/handoff_runtime.rs`, `src/server/handoff.rs`, `docs/next/website/src/content/docs/session-state.mdx`
- **Notable:** `HandoffRuntimeState` (`src/handoff_runtime.rs`) carries per-pane child PID, terminal size, keyboard-protocol flags, and `initial_history_ansi` across the handoff — this is a live process/fd handoff, not a serialized-then-relaunched restore. `src/server/handoff.rs` defines `READY_TIMEOUT`/`OWNED_ACK_TIMEOUT` for a handshake protocol between old and new server.
- **Keywords:** live handoff, `--handoff`, fd passing, Unix-only, update flow

### session-snapshot-bootstrap-rpc
- **What:** `session.snapshot` is a one-time RPC (not a subscription) that returns full bootstrap state — version/protocol metadata, focused ids, workspace/tab/pane/agent records, layout snapshots — for API clients that keep their own local runtime cache, which they then keep current via `events.subscribe`.
- **Where:** `docs/next/website/src/content/docs/socket-api.mdx`, `src/api/schema/session.rs`
- **Notable:** Explicit client contract: "It is not a subscription; after reading it, subscribe to resource events and update the local cache from those events. Call `session.snapshot` again after reconnecting or when the local cache may be stale."
- **Keywords:** session.snapshot, bootstrap RPC, local cache pattern

## tooling

### cli-as-plugin-api
- **What:** There is no separate plugin SDK or restricted command subset — the entire `herdr` CLI (every command in the CLI reference) is available to any out-of-process plugin script, invoked portably through the `HERDR_BIN_PATH` env var rather than talking to the raw socket directly.
- **Where:** `docs/next/website/src/content/docs/plugins.mdx`, `docs/next/website/src/content/docs/cli-reference.mdx`
- **Notable:** "There is no separate plugin SDK or restricted command set. The entire Herdr CLI is the plugin API." This collapses "scripting surface" and "extension surface" into one thing.
- **Keywords:** HERDR_BIN_PATH, CLI as API, no plugin SDK

### self-describing-protocol-schema
- **What:** `herdr api schema [--json|--output PATH]` prints/exports the bundled JSON Schema document for the socket protocol (requests, responses, errors, events), generated from Rust types via `schemars`.
- **Where:** `docs/next/api/herdr-api.schema.json`, `src/api/schema.rs`, `src/cli/api.rs`, `docs/next/website/src/content/docs/socket-api.mdx`
- **Notable:** Cargo.toml depends on `schemars = { version = "1.2.1", features = ["derive"] }` specifically so wire types can self-generate this schema, avoiding hand-maintained protocol docs drifting from the Rust source of truth.
- **Keywords:** JSON Schema, schemars, `herdr api schema`, protocol introspection

### protocol-version-discipline
- **What:** A single `PROTOCOL_VERSION` constant gates client/server wire compatibility; project rules require comparing it against the latest released git tag before bumping, so it only increases once per release even if the wire format changes multiple times on `master`.
- **Where:** `src/protocol/wire.rs`, `AGENTS.md` (Code Conventions section)
- **Notable:** `src/protocol/wire.rs`: "`pub const PROTOCOL_VERSION: u32 = 16;` ... Bumped when wire format changes incompatibly." AGENTS.md operationalizes this: "Bump it only if the current source protocol is not already greater than the latest released protocol."
- **Keywords:** PROTOCOL_VERSION, wire compatibility, version bump discipline

### direct-terminal-bridging
- **What:** `terminal session observe` (read-only) and `terminal session control` (writable, single-owner with `--takeover`) stream newline-delimited JSON `terminal.frame` (base64 ANSI) / `terminal.closed` records over stdio, for building third-party bridges without embedding the full Herdr client.
- **Where:** `docs/next/website/src/content/docs/persistence-remote.mdx`, `docs/next/website/src/content/docs/cli-reference.mdx`, `src/server/terminal_attach.rs`
- **Notable:** Explicitly framed as a bridge primitive: "For third-party bridges that only need rendered terminal bytes, use a read-only terminal session observer." Control mode reads `terminal.input`/`terminal.resize`/`terminal.scroll`/`terminal.release` commands on stdin.
- **Keywords:** terminal session observe/control, newline-delimited JSON frames, bridge primitive

### shell-completions
- **What:** `herdr completion <shell>` (alias `completions`) generates completion scripts for zsh/bash/fish/PowerShell/elvish via `clap_complete`.
- **Where:** `src/cli/completion.rs`, `docs/next/website/src/content/docs/cli-reference.mdx`, `Cargo.toml` (`clap_complete = "4.5"`)
- **Keywords:** shell completion, clap_complete

## config-packaging

### layered-config-with-safe-fallback
- **What:** `config.toml` is fully optional; `herdr --default-config` prints the complete commented default; any invalid value falls back to a safe default plus a startup warning rather than a hard failure; `herdr server reload-config` applies most settings live.
- **Where:** `docs/next/website/src/content/docs/configuration.mdx`, `src/config/model.rs`, `src/config/io.rs`
- **Notable:** "If a config value is invalid, Herdr falls back to a safe default and shows a startup warning" — config errors degrade gracefully instead of blocking startup.
- **Keywords:** config.toml, --default-config, reload-config, safe fallback

### update-channels
- **What:** Two update channels (stable, preview) both built from `master`; stable uses `website/latest.json`, preview is opt-in via `herdr channel set preview` and uses `website/preview.json` published by a scheduled/manual GitHub Actions workflow. `herdr update --handoff` only works for Herdr's own updater-managed installs, not Homebrew/mise/Nix.
- **Where:** `AGENTS.md` (Release Channels section), `scripts/preview.py`, `.github/workflows/preview.yml`, `docs/next/website/src/content/docs/windows-beta.mdx`
- **Notable:** Windows defaults to the preview channel automatically ("Herdr defaults to preview on Windows without changing your config") since native Windows support itself is beta.
- **Keywords:** stable/preview channels, latest.json, preview.json, update --handoff

### vendored-native-dependency-with-tracked-patches
- **What:** `libghostty-vt` (a Zig VT/terminal library) is vendored into the repo with its upstream commit recorded in a vendor manifest; any local patch on top must be tracked in a patches index with issue/PR links, touched files, verification steps, and an explicit removal condition, and `just check` mechanically verifies patch files are indexed and reverse-apply cleanly.
- **Where:** `vendor/` (directory verified; exact files `vendor/libghostty-vt.vendor.json` and `vendor/libghostty-vt.patches.md` are named in `AGENTS.md` but were not directly readable in this environment due to local tool sandboxing), `scripts/vendor_libghostty_vt.py`, `scripts/test_vendor_libghostty_vt.py`, `Cargo.toml` (`[patch.crates-io] portable-pty = { path = "vendor/portable-pty" }`)
- **Notable:** AGENTS.md states the exact discipline: "When updating libghostty-vt, check every active patch... If the new upstream commit contains the fix, remove the local patch and index entry, then rerun the listed verification. If not, reapply the patch on top of the new vendored source." This turns vendor-patch rot into a mechanically checked invariant rather than tribal knowledge.
- **Keywords:** vendored dependency, patch tracking, `just check`, removal condition, Zig build

### nix-flake-packaging
- **What:** A `flake.nix` + `nix/package.nix` build the Rust binary via `rustPlatform.buildRustPackage`, importing `Cargo.lock` directly (`cargoLock.lockFile`) so version bumps don't require a separate Nix cargo hash update, and vendoring the Zig build cache for `libghostty-vt` through a generated `build.zig.zon.nix`.
- **Where:** `flake.nix`, `nix/package.nix`
- **Notable:** `nix/package.nix` comments that its Nix check is deliberately build-only: "Rust tests are covered by the normal CI workflow. The Nix check is intentionally build-only so it validates packaging inputs without duplicating the full Rust test suite."
- **Keywords:** flake.nix, cargoLock.lockFile, Zig vendoring, packaging-only check

### integration-asset-versioning
- **What:** Each per-agent hook/plugin script embeds an `*_INTEGRATION_VERSION` marker; project convention treats these as migration versions relative to the latest **released** tag (not per-commit counters), bumped once even if the asset changes multiple times before a release.
- **Where:** `AGENTS.md` (Code Conventions), `src/integration/version.rs`, `src/integration/assets/` (e.g. `src/integration/assets/claude/herdr-agent-state.sh`)
- **Notable:** Mirrors the same "compare against latest release tag, don't over-bump on master" discipline as `PROTOCOL_VERSION`.
- **Keywords:** HERDR_INTEGRATION_VERSION, hook script versioning, release-relative bump

### multi-channel-install
- **What:** Install via curl script (`herdr.dev/install.sh`), Homebrew, mise, or a PowerShell script for Windows beta (`install.ps1`); binaries are also published directly per-platform (`herdr-linux-x86_64`, `herdr-linux-aarch64`, `herdr-macos-x86_64`, `herdr-macos-aarch64`).
- **Where:** `README.md`, `AGENTS.md` (Release Channels), `docs/next/website/src/content/docs/install.mdx`, `docs/next/website/src/content/docs/windows-beta.mdx`
- **Keywords:** curl installer, Homebrew, mise, install.ps1, release assets

## skills

### shipped-agent-skill-file
- **What:** `SKILL.md` at repo root is a reusable, installable Claude/agent skill file teaching a coding agent how to drive Herdr's CLI/socket API from inside a Herdr-managed pane (pane IDs, split/run/read, waits, workspace/tab commands, safety rules). Installed with `npx skills add ogulcancelik/herdr --skill herdr -g`.
- **Where:** `SKILL.md`, `docs/next/website/src/content/docs/agent-skill.mdx`
- **Notable:** The skill's first instruction is a hard gate: `test "${HERDR_ENV:-}" = 1` — "If the check fails, say that you are not running inside Herdr and stop. Do not inspect or control the focused Herdr session from outside Herdr." This is a self-contained safety precondition baked into the skill file itself, not just documentation.
- **Keywords:** SKILL.md, npx skills, HERDR_ENV gate, agent-operates-tool skill

### dual-audience-agent-docs
- **What:** Herdr maintains two separate agent-facing documents for two different jobs: `SKILL.md` (an agent *operating* Herdr from inside a pane) vs `herdr.dev/agent-guide.md` (an agent *teaching a human* how to set up/use/troubleshoot Herdr, referenced from the docs homepage as a paste-this-prompt onboarding flow).
- **Where:** `docs/next/website/src/content/docs/agent-skill.mdx`, `docs/next/website/src/content/docs/index.mdx`
- **Notable:** "Herdr also serves a separate guide at `herdr.dev/agent-guide.md` for a different job... The skill is for an agent operating Herdr; the guide is for an agent teaching a human." Docs homepage literally instructs users to paste a prompt telling their own agent to fetch that guide URL first.
- **Keywords:** agent-guide.md, operate vs teach, paste-this-prompt onboarding

## hooks

### per-agent-integration-hooks
- **What:** 14 separate official integrations (Pi, OMP, Claude Code, Codex, Copilot, Devin, Droid, Kimi, OpenCode, Kilo, Hermes, Qoder, Cursor, MastraCode), each installed/uninstalled via `herdr integration install/uninstall <agent>`, writing an agent-specific hook/plugin file into that agent's own config directory (e.g. `~/.claude/hooks/herdr-agent-state.sh` + `settings.json` patch, `~/.codex/hooks.json` + `config.toml` feature flag) and reporting back over the same local socket API.
- **Where:** `docs/next/website/src/content/docs/integrations.mdx`, `src/integration/mod.rs`, `src/integration/config_edit.rs`, `src/integration/assets/` (per-agent subdirs, e.g. `src/integration/assets/codex/herdr-agent-state.sh`, `src/integration/assets/pi/herdr-agent-state.ts`)
- **Notable:** Each integration's install/uninstall is scoped and reversible per-agent — e.g. Codex install "ensures `[features] hooks = true`... also removes the deprecated top-level `codex_hooks` flag," while uninstall "leaves `config.toml` unchanged." The docs table (`docs/next/website/src/content/docs/integrations.mdx`) cleanly separates which agents are "lifecycle authority" vs "session identity" integrations.
- **Keywords:** integration install/uninstall, per-agent hook file, config.toml/settings.json patching

### plugin-event-hooks
- **What:** A plugin manifest can declare `[[events]] on = "<herdr-event-name>"` blocks that run an argv command whenever Herdr emits that lifecycle event (e.g. `worktree.created`); unknown event names are accepted but flagged with a link-time warning rather than rejected.
- **Where:** `docs/next/website/src/content/docs/plugins.mdx`, `docs/next/website/src/content/docs/socket-api.mdx`, `src/app/api/plugins/mod.rs`
- **Notable:** "An unrecognised name is not an error — the link still succeeds — but the returned plugin info includes a warning (e.g. `\"unknown event 'worktree.craeted'\"`)" — the example warning literally shows a caught typo, illustrating the intended UX.
- **Keywords:** plugin event hooks, `[[events]]`, event-name validation, non-fatal warnings

### metadata-vs-lifecycle-authority-separation
- **What:** Two distinct report verbs exist on purpose: `pane.report_agent` (or CLI `herdr pane report-agent`) sets semantic lifecycle state that drives waits/notifications/rollups, while `pane.report_metadata` (`herdr pane report-metadata`) only changes presentation (title, display name, per-state labels, arbitrary `$token` values) and cannot override lifecycle authority. Metadata calls carry an optional `--agent`/`--applies-to-source` guard so a user hook can't accidentally clobber an official integration's presentation.
- **Where:** `docs/next/website/src/content/docs/socket-api.mdx`, `docs/next/website/src/content/docs/integrations.mdx`, `docs/next/website/src/content/docs/cli-reference.mdx`
- **Notable:** "Metadata reports are display-only... `working`, `blocked`, `idle`, waits, notifications, and rollups still come from semantic state." Token reports separately support `--seq` sequence numbers so stale out-of-order reports from the same source are silently ignored rather than corrupting display state.
- **Keywords:** report-agent vs report-metadata, semantic state vs presentation, source guard, seq ordering

## safety

### plugin-trust-and-preview-gate
- **What:** Plugins are ordinary unsandboxed code that runs as the user with full CLI access; `herdr plugin install` shows an interactive trust preview (source + commands to be run) before executing anything, requires `--yes` to skip in scripts, and hard-refuses to link/install any plugin whose declared `min_herdr_version` is newer than the running binary.
- **Where:** `docs/next/website/src/content/docs/plugins.mdx` (Trust and security section), `docs/next/website/src/content/docs/cli-reference.mdx`
- **Notable:** The docs are unusually direct about the non-sandboxing: "Herdr validates the manifest and keeps each plugin's config and state in its own directory, but it does not review or sandbox what a plugin does... they are yours to vet and run at your own discretion."
- **Keywords:** plugin trust preview, --yes, min_herdr_version gate, no sandboxing

### contributor-gate-automation
- **What:** GitHub Actions automatically close (a) non-template issues from non-maintainers (checking a required bug-report template with confirmation checkbox + required sections/environment fields) and (b) PRs from first-time contributors not on an `.github/APPROVED_CONTRIBUTORS` allowlist, with `/approve @username` on an accepted issue as the only way onto that list.
- **Where:** `.github/workflows/issue-gate.yml`, `.github/workflows/pr-gate.yml`, `.github/workflows/approve-contributor.yml`, `CONTRIBUTING.md`
- **Notable:** `issue-gate.yml` does template-shape validation with regex against required section headings and environment fields, and separately handles a translation-issue template variant — a fairly elaborate bot-driven gate to keep the issue tracker to "reproducible bug reports and maintainer-created... work items" only, deflecting feature requests/ideas to Discussions.
- **Keywords:** issue-gate, pr-gate, APPROVED_CONTRIBUTORS, template enforcement, `/approve @user`

### destructive-worktree-remove-confirmation
- **What:** `worktree.remove` first asks Git to remove the checkout safely; only if Git refuses (dirty/untracked files) does Herdr ask again before running the forced remove, and it never deletes the branch regardless.
- **Where:** `docs/next/website/src/content/docs/configuration.mdx`, `docs/next/website/src/content/docs/cli-reference.mdx` (`worktree remove --force`)
- **Notable:** Two-step escalation pattern (safe attempt → explicit forced retry) rather than a single `--force` flag that always force-deletes.
- **Keywords:** worktree remove, double confirmation, git worktree remove, branch preserved

### direct-attach-single-writer-ownership
- **What:** Only one writable direct-attach client can own input/resize for a given terminal at a time; a second client must pass `--takeover` to explicitly evict the current owner rather than silently sharing control.
- **Where:** `docs/next/website/src/content/docs/persistence-remote.mdx`, `docs/next/website/src/content/docs/cli-reference.mdx`
- **Notable:** Read-only "observer" streams are unlimited and don't require takeover — the safety boundary is specifically about concurrent *write* ownership, not concurrent viewing.
- **Keywords:** `--takeover`, single writer, terminal ownership, observe vs control

## self-improvement

### pre-release-audit-skill
- **What:** A maintainer-only Codex/agent skill (`herdr-pre-release-audit`) that audits release readiness by diffing commits since the base release against the staged `docs/next/CHANGELOG.md` and `docs/next/README.md`, checking issue-reference lines, and deciding when to run `just release-docs-check`.
- **Where:** `.codex/skills/herdr-pre-release-audit/SKILL.md`, `.codex/skills/herdr-pre-release-audit/references/pre-release-audit.md`
- **Notable:** The skill explicitly scopes itself to read-only analysis by default: "Do not edit files during the audit unless the user explicitly asks to apply fixes." This is the project using its own agent-skill mechanism to enforce its own release-doc-sync process (`docs/next` staging → stable docs) rather than relying on a human remembering the checklist.
- **Keywords:** pre-release audit, docs/next diffing, release-docs-check, maintainer-only skill

### vendor-patch-lifecycle-discipline
- **What:** The same self-checking pattern as the release audit, applied to vendored native code: every local patch against `libghostty-vt` must name its own removal condition, and updating the vendored source requires re-checking each patch against the new upstream to decide keep-vs-drop.
- **Where:** `AGENTS.md` (Vendored libghostty-vt section), `scripts/test_vendor_libghostty_vt.py`
- **Notable:** This is a repo-encoded process for the project to shed its own technical debt automatically as upstream catches up, verified mechanically by `just check` rather than left to memory.
- **Keywords:** patch removal condition, vendor drift, mechanically checked debt

## planning

### issue-triage-skill
- **What:** A Codex/agent skill (`triage`) that reads open GitHub issues for the repo and returns a fixed-shape Markdown decision table: a 🔴/🟡/🔵 priority "light," a short imperative recommendation (`fix now`, `queue`, `defer`, `needs repro`, `close?`), the issue link, age, reaction count, and a one-line reason.
- **Where:** `.codex/skills/triage/SKILL.md`
- **Notable:** The skill hard-codes both the output schema (exact table columns) and the classification rubric (what counts as 🔴 vs 🟡 vs 🔵) rather than leaving triage judgment freeform, and explicitly caps narrative: "Do not produce a long narrative unless the user asks for depth."
- **Keywords:** issue triage, priority lights, decision table, fixed rubric

## ux

### mouse-first-with-optional-prefix-keyboard
- **What:** Every keyboard action (click panes/tabs/workspaces, drag split borders, right-click menus, drag-select text) has a mouse equivalent by design; the tmux-style `prefix` (default `ctrl+b`) keyboard layer is presented as fully optional, not required to use Herdr.
- **Where:** `docs/next/website/src/content/docs/concepts.mdx`, `docs/next/website/src/content/docs/keyboard.mdx`, `AGENTS.md` ("Herdr is a mouse-first TUI")
- **Notable:** Keyboard doc opens with: "Herdr is mouse-native... Keyboard control is an optional layer, not a requirement," then still teaches "five keys first" for people who do want them.
- **Keywords:** mouse-first, prefix key, optional keyboard layer

### prefix-free-chord-safety-guidance
- **What:** Docs include a researched table of which direct (no-prefix) key chords are actually safe to bind, based on testing default shortcuts across 10 terminal emulators (Ghostty, iTerm2, Terminal.app, kitty, WezTerm, Alacritty, Warp, Windows Terminal, GNOME Terminal, Konsole) plus GNOME/KDE global shortcuts, concluding `ctrl+alt` is the one nearly-untouched modifier family, with an explicit "avoid these" list of taken `ctrl+alt` combos.
- **Where:** `docs/next/website/src/content/docs/keyboard.mdx`
- **Notable:** "A chord has to survive three layers before Herdr sees it: your operating system, your outer terminal..., and the programs running inside the pane." This is unusually thorough, terminal-ecosystem-aware keybinding guidance rather than a generic "customize your keys" page.
- **Keywords:** ctrl+alt, chord safety, terminal default shortcuts, prefix-free

### sidebar-token-row-system
- **What:** The sidebar's Agent and Space rows are configured as arrays of token arrays (`rows = [["state_icon","workspace","tab"],["agent"]]`), with a fixed vocabulary of built-in tokens (`state_icon`, `agent`, `terminal_title_stripped`, `branch`, `git_status`, etc.) plus arbitrary custom `$name` tokens fed by `pane.report_metadata`/`workspace.report_metadata`, and per-agent row overrides (`rows_by_agent.claude = [...]`).
- **Where:** `docs/next/website/src/content/docs/configuration.mdx`, `src/ui/sidebar/tokens.rs`, `src/config/sidebar.rs`
- **Notable:** Missing-value handling is spelled out precisely: "Missing values and their separators disappear; a row disappears when none of its tokens have a value" — a declarative row-collapsing rule rather than blank/placeholder cells.
- **Keywords:** sidebar rows, token vocabulary, `$name` custom token, rows_by_agent

### unified-pane-placement-model
- **What:** One placement vocabulary (`overlay`, `popup`, `split`, `tab`, `zoomed`) is shared across plugin panes and custom command keybindings: `overlay` is a temporary zoomed pane that restores prior focus, `popup` is a session-modal terminal that doesn't touch the tab layout and has no pane ID / doesn't participate in pane or agent APIs, the rest behave as normal tiled panes.
- **Where:** `docs/next/website/src/content/docs/plugins.mdx`, `docs/next/website/src/content/docs/configuration.mdx` (custom command keybindings), `src/app/api/plugins/panes.rs`, `src/popup_size.rs`
- **Notable:** Popups are deliberately outside the pane model entirely: "A popup has no pane ID, remains outside all `pane.*` and agent APIs, emits no pane lifecycle events... and does not export `HERDR_PANE_ID`." One placement enum serves both first-party keybindings and third-party plugin panes.
- **Keywords:** overlay/popup/split/tab/zoomed, popup non-pane, placement enum

### theme-auto-switching
- **What:** Themes can auto-follow the host terminal's reported light/dark appearance change (`[theme] auto_switch = true` with `light_name`/`dark_name`), falling back to known built-in light/dark sibling pairs (`tokyo-night`/`tokyo-night-day`, `gruvbox`/`gruvbox-light`) when not explicitly configured; manual Settings selection disables auto-switch.
- **Where:** `docs/next/website/src/content/docs/configuration.mdx`, `src/config/theme.rs`, `src/app/theme_sync.rs`
- **Keywords:** auto_switch, light/dark sibling themes, terminal appearance reporting

### ime-cursor-anchor-for-cjk
- **What:** macOS-only opt-in (`[experimental] reveal_hidden_cursor_for_cjk_ime = true`, scoped to a `cjk_ime_agents` list) that reveals a cursor anchor so native CJK input-method candidate windows can follow the focused pane even when the agent TUI hides the hardware cursor.
- **Where:** `docs/next/website/src/content/docs/configuration.mdx`
- **Notable:** Scoped per-agent specifically "to avoid showing an extra hardware cursor in unrelated applications" — a narrowly-targeted platform accessibility fix rather than a global toggle.
- **Keywords:** CJK IME, hidden cursor reveal, macOS-only, per-agent scoping

## docs-style

### layered-agents-md-scoping
- **What:** `AGENTS.md` (symlinked as `CLAUDE.md`) is explicitly divided into scope-tagged sections — Universal, Maintainer-only (gated on GitHub account `ogulcancelik`), Local-machine-only (gated on specific paths/env/SSH alias), External-contributor-guardrail (default for everyone else) — with an upfront rule telling any agent how to determine which sections apply to it.
- **Where:** `AGENTS.md` (Scope and Audience section)
- **Notable:** The gating conditions are concrete and checkable by an agent, not just prose: "Local Can machine workflow applies only on Can's own workstation or Windows VM setup, for example when `/home/can/Projects/herdr`, `HERDR_ENV=1`, or the `windows-wirt` SSH alias exists. If those facts are not true, skip local machine workflow."
- **Keywords:** scoped AGENTS.md, audience gating, maintainer vs contributor sections

### docs-next-staging-mirror
- **What:** Unreleased user-facing doc changes are staged under `docs/next/` (mirroring root `README.md`/`CHANGELOG.md` and the entire `website/src/content/docs/` tree) and only copied into the real stable docs during release review; a `just release-docs-check` recipe mechanically diffs stable vs staged copies, English vs `ja`/`zh-cn` translations, and refuses to release on any mismatch.
- **Where:** `docs/next/README.md`, `docs/next/CHANGELOG.md`, `docs/next/website/src/content/docs/` (mirrors `website/src/content/docs/`), `justfile` (`release-docs-check` recipe), `scripts/docs_translation_parity.py`
- **Notable:** `justfile`'s `release-docs-check` recipe is unusually exhaustive: it diffs config-reference JSON, root README/CHANGELOG, every stable `.mdx` against its staged counterpart, checks every staged `.mdx` has a matching released doc, and confirms `ja`/`zh-cn` translations exist for every English doc (and vice versa) — turning "did we forget to update docs" into a hard CI-style gate rather than a checklist.
- **Keywords:** docs/next staging, release-docs-check, translation parity, mirrored doc tree

## quality-gates

### just-recipe-composed-gates
- **What:** `just test` (nextest + Python maintenance-script unittest + integration-asset bun test + plugin-marketplace worker test), `just lint` (fmt --check + clippy -D warnings), `just ci` (lint + nextest with slow-test reporting), `just check` (ci + windows cross-lint + maintenance tests) form a layered set of gates from "fast local" to "full pre-release."
- **Where:** `justfile`
- **Notable:** `just windows-lint` runs `cargo clippy --target x86_64-pc-windows-msvc` from Linux/macOS specifically "to catch cfg(windows) compile and clippy failures before CI" — a deliberate cross-compile lint step baked into the local dev loop, not just CI.
- **Keywords:** just test/lint/ci/check, windows cross-lint, layered gates

### pre-commit-hook-enforced-formatting
- **What:** A repo-local git hook (`git config core.hooksPath .githooks`, installed via `just install-hooks`) runs `just lint` on every commit; `commit-msg` hook also present.
- **Where:** `.githooks/pre-commit`, `.githooks/commit-msg`, `justfile` (`install-hooks` recipe)
- **Keywords:** core.hooksPath, pre-commit, commit-msg, just install-hooks

### config-reference-consistency-check
- **What:** A Python script walks the serde structs in `src/config/*.rs` from the root `Config` struct, builds the canonical set of dotted TOML key paths, and diffs it against the documented preview config reference — failing CI on any missing, stale, duplicated, or value-drifted entry.
- **Where:** `scripts/config_reference_check.py`, `scripts/test_config_reference_check.py`
- **Notable:** Script docstring: "The comparison checks key names and serde-derived enum values, so failures name exact missing, stale, duplicated, or value-drifted entries" — config docs are generated/verified from the actual Rust type, not hand-synced prose.
- **Keywords:** config reference check, serde struct walk, doc/code drift detection

## testing-evals

### real-binary-pty-integration-tests
- **What:** Integration tests in `tests/*.rs` spawn the actual compiled `herdr` binary inside a real pseudo-terminal (via the `portable-pty` crate) and drive it over Unix sockets/PTY — covering multi-client behavior, thin-client mode, detach/reattach, live handoff, auto-detect launch, headless server mode, and CLI wrapper behavior end-to-end rather than mocking the process boundary.
- **Where:** `tests/multi_client.rs`, `tests/client_mode.rs`, `tests/detach_reattach.rs`, `tests/live_handoff.rs`, `tests/auto_detect.rs`, `tests/server_headless.rs`, `tests/cli_wrapper.rs`, `tests/cross_area.rs`, `tests/api_ping.rs`
- **Notable:** `tests/support/mod.rs` implements a PID/runtime-dir registry plus a background watchdog thread (`WATCHDOG_SCAN_INTERVAL`) that force-cleans up orphaned spawned `herdr` processes and runtime directories left behind by a crashed or killed test — real subprocess integration testing needed its own leak-detection infrastructure.
- **Keywords:** portable-pty, real binary spawn, PID watchdog, orphan cleanup

### evidence-capture-scripts-for-terminal-protocol
- **What:** Standalone Python scripts capture real terminal/keyboard behavior as fixture data rather than relying on assumed escape-sequence specs: `capture_agent_screen.py` repeatedly reads live Herdr pane state for agent-detection fixtures, `capture_key_matrix.py`/`capture_keys.py` record raw key sequences, and `verify_suspicious_keys.py` is an interactive verifier that captures a curated list of "suspicious" keys multiple times and writes a TSV for comparison across terminals.
- **Where:** `scripts/capture_agent_screen.py`, `scripts/capture_key_matrix.py`, `scripts/capture_keys.py`, `scripts/verify_suspicious_keys.py`, `tests/fixtures/keyboard_protocol_corpus.tsv`, `tests/fixtures/macos_terminal_variants.tsv`, `tests/fixtures/linux_terminal_variants.tsv`
- **Notable:** This mirrors the "screen detection is evidence-based" principle from `AGENTS.md` at the tooling level — the project has dedicated scripts specifically so that terminal-quirk fixes and detection-manifest edits are backed by captured real evidence (TSV corpora) instead of speculative escape-sequence tables.
- **Keywords:** evidence capture, TSV fixture corpus, keyboard protocol corpus, terminal variant matrix

### python-maintenance-test-suite
- **What:** A set of `unittest`-based Python tests (run via `python3 -m unittest scripts.test_*` inside `just test`) validate repo-maintenance scripts themselves: changelog parsing (`test_changelog.py`), manifest checking (`test_agent_detection_manifest_check.py`), config reference sync (`test_config_reference_check.py`), docs translation parity (`test_docs_translation_parity.py`), preview release logic (`test_preview.py`), and vendored-dependency patch/version checks (`test_vendor_libghostty_vt.py`, `test_vendor_portable_pty.py`).
- **Where:** `scripts/test_changelog.py`, `scripts/test_agent_detection_manifest_check.py`, `scripts/test_config_reference_check.py`, `scripts/test_docs_translation_parity.py`, `scripts/test_preview.py`, `scripts/test_vendor_libghostty_vt.py`, `scripts/test_vendor_portable_pty.py`, `justfile`
- **Notable:** The project's own release/doc/vendor tooling is treated as first-class code with its own test suite wired into the same `just test`/`just check` gates as the Rust code, not left as untested "just scripts."
- **Keywords:** unittest maintenance tests, scripts as tested code, just test integration

## workflow

### how-to-work-decision-guide
- **What:** A dedicated doc page frames three distinct working modes as a decision (not just a features list): plain `herdr` for local work, SSH-then-`herdr` for tmux-style remote work (including phone SSH clients), and `herdr --remote <host>` for a thin local client streaming a remote session (which uniquely enables local clipboard-image bridging to the remote server).
- **Where:** `docs/next/website/src/content/docs/how-to-work.mdx`, `docs/next/website/src/content/docs/persistence-remote.mdx`
- **Notable:** The clipboard-bridging capability is explicitly used as the tie-breaker between the two remote modes: "If you SSH into the server first and run `herdr` there, Herdr runs entirely on the server and cannot read your local desktop clipboard beyond normal terminal text paste."
- **Keywords:** local vs SSH vs --remote, thin client, clipboard image bridge

### plugin-authoring-and-distribution-workflow
- **What:** End-to-end plugin lifecycle: author a directory with `herdr-plugin.toml` + one script/binary → `herdr plugin link <path>` for local dev → `herdr plugin action invoke`/`pane open` to test → publish as a normal public GitHub repo tagged `herdr-plugin` → users install via `herdr plugin install owner/repo[/subdir]` with an interactive trust preview and optional build-command execution.
- **Where:** `docs/next/website/src/content/docs/plugins.mdx`, `docs/next/website/src/content/docs/marketplace.mdx`, `docs/next/website/src/content/docs/cli-reference.mdx`
- **Notable:** The marketplace is deliberately unreviewed and mechanical: "The index does not parse `herdr-plugin.toml` yet... Discovery is automatic and unreviewed. A listing means a repository tagged itself, not that Herdr vetted it." Distribution piggybacks entirely on GitHub topics + `git clone`, no custom registry service.
- **Keywords:** plugin link → install workflow, GitHub-topic marketplace, trust preview, build commands

### remote-ssh-bridge-with-managed-keepalive
- **What:** `herdr --remote <host>` manages its own temporary SSH config (including the user's real `~/.ssh/config` first, then adding fallback keepalive settings and a private per-attach control socket for connection reuse) unless disabled via `[remote] manage_ssh_config = false`, and auto-installs a matching `herdr` binary on hosts that lack one (interactive prompt; non-interactive runs refuse to modify the host).
- **Where:** `docs/next/website/src/content/docs/persistence-remote.mdx`, `docs/next/website/src/content/docs/configuration.mdx`, `src/remote/unix.rs`
- **Notable:** `src/remote/unix.rs` constants show the concrete bootstrap policy: `STABLE_UPDATE_MANIFEST_URL`/`PREVIEW_UPDATE_MANIFEST_URL` for fetching a matching release asset when platforms differ, and `HERDR_REMOTE_BINARY` env var to force a local custom binary instead (used for testing dev builds against a remote host).
- **Keywords:** managed SSH config, control socket reuse, auto-install remote binary, HERDR_REMOTE_BINARY

## repo-layout

### maintainer-worktree-branch-convention
- **What:** Maintainer workflow prescribes a fixed multi-checkout layout: shared integration checkout at `../herdr`, per-task worktrees at `../herdr-worktrees/<task-slug>`, task branches named `issue/<id>-<slug>`; all edits/tests happen in the task worktree, then the shared checkout is fast-forwarded and pushed — the task branch itself is never the "landing" branch.
- **Where:** `AGENTS.md` (Maintainer Workflow → Multi-agent isolation)
- **Notable:** Explicitly designed for concurrent agent sessions: "Read-only investigation can happen in the shared checkout... If you find unrelated implementation changes already in progress in the main worktree, use a dedicated worktree instead."
- **Keywords:** `../herdr-worktrees/<slug>`, `issue/<id>-<slug>` branches, fast-forward integration

### local-ignored-planning-directory
- **What:** `.local/prd/` is a git-ignored, locally-controlled directory explicitly designated for PRDs, planning notes, and exploratory specs that should never be committed.
- **Where:** `AGENTS.md` (Docs section: "Put local PRDs, planning notes, and exploratory specs under `.local/prd/`; `.local/` is ignored and locally controlled.")
- **Notable:** A named, documented convention for ephemeral planning artifacts rather than leaving them scattered or requiring `.gitignore` archaeology to discover.
- **Keywords:** `.local/prd/`, gitignored planning dir

### state-runtime-render-module-split
- **What:** Explicit architectural invariants stated as project principles: `AppState` is pure data testable without PTYs/async; `PaneState` (data) is separate from `PaneRuntime` (live process); `render()` only draws from `&AppState` and never mutates; `app/` is split into `state.rs`/`actions.rs`/`input.rs`; OS-specific code is isolated to `src/platform/<os>.rs` with only shared traits in `src/platform/mod.rs`.
- **Where:** `AGENTS.md` (Principles section), `src/app/mod.rs` (module doc comment restates the same split), `src/platform/` (`linux.rs`, `macos.rs`, `windows.rs`, `fallback.rs`, `mod.rs`)
- **Notable:** `src/app/mod.rs`'s top comment literally repeats the architectural rule as executable documentation: "`state.rs` — AppState, Mode, and pure data structs / `actions.rs` — state mutations (testable without PTYs/async) / `input.rs` — key/mouse → action translation."
- **Keywords:** AppState/PaneRuntime split, pure render, platform isolation, no god objects

## Coverage notes

**Read in full:** README.md, AGENTS.md/CLAUDE.md, SKILL.md, CONTRIBUTING.md, CHANGELOG.md (head), Cargo.toml, justfile, flake.nix, nix/package.nix, `.zed/settings.json`, `.githooks/pre-commit`, all 16 files under `docs/next/website/src/content/docs/*.mdx` (English only — `ja`/`zh-cn` locales were not read since they mirror the English content per the translation-parity script), `.codex/skills/{triage,herdr-pre-release-audit}/SKILL.md`, `.github/workflows/{issue-gate,pr-gate}.yml`.

**Read partially (top-of-file doc comments / signatures, not full implementations):** `src/agent_resume.rs`, `src/handoff_runtime.rs`, `src/detect/{mod,manifest,manifest_update}.rs`, `src/protocol/{wire,mod}.rs`, `src/persist/*.rs`, `src/server/{mod,handoff,autodetect,headless}.rs`, `src/plugin_command.rs`, `src/plugin_paths.rs`, `src/app/api/plugins/mod.rs`, `src/api/mod.rs`, `src/remote/unix.rs`, `src/remote.rs`, `src/render_prof.rs`, `src/main.rs`, `src/app/mod.rs`, `src/worktree.rs`, `src/workspace.rs`, `src/session.rs`, `scripts/*.py` (headers only), `tests/*.rs` (module doc comments + imports only), `tests/support/mod.rs` (partial).

**Directory structure enumerated but contents not read:** full `src/` tree at depth 2 (`src/api/schema/`, `src/app/{api,input}/`, `src/cli/`, `src/client/`, `src/config/`, `src/ghostty/`, `src/integration/assets/`, `src/pane/`, `src/pty/{actor,backend}/`, `src/terminal/`, `src/ui/sidebar/`, `src/workspace/git/`), `workers/plugin-marketplace/` (package.json/wrangler.toml/src file names only, contents not read), `.github/workflows/` (only `issue-gate.yml` and `pr-gate.yml` read in full; `approve-contributor.yml`, `approve-merged-contributor.yml`, `build-artifacts-manual.yml`, `ci.yml`, `label-next-release-issues.yml`, `nix.yml`, `release.yml` were listed by name only).

**Not read / inaccessible:** `vendor/` file contents (`vendor/libghostty-vt.vendor.json`, `vendor/libghostty-vt.patches.md`, `vendor/patches/libghostty-vt/`, `vendor/portable-pty/`) — a local `scout-block` hook denied both `Bash` and `Read` access to any path containing `vendor`, for both `ls`/`cat` and the `Read` tool. The `vendor-patch-lifecycle-discipline` and `vendored-native-dependency-with-tracked-patches` entries above are therefore sourced from `AGENTS.md`'s own description of that mechanism and from `Cargo.toml`'s `[patch.crates-io]` entry, not from directly reading the vendor files; the `vendor/` directory's existence was confirmed via the root `ls`. `.pi/` directory contents were not inspected (listed in root `ls` but not explored — likely local/private agent state, out of scope for an upstream-feature inventory). `website/` (Astro site source beyond the `docs/next` content mirror), `assets/`, and per-language translated `.mdx` files under `docs/next/website/src/content/docs/{ja,zh-cn}/` were not read.

**Proposed new domains?** None strictly required — every feature found mapped cleanly onto an existing domain. Two features sit at a domain boundary and are noted here rather than duplicated above: the vendor-patch lifecycle (`vendor-patch-lifecycle-discipline`) reads equally as `config-packaging` (it's about a vendored build dependency) and `self-improvement` (it's a mechanically-enforced self-maintenance process) — it was filed under `self-improvement` for its process angle, with a sibling entry under `config-packaging` for the packaging mechanics. The `.codex/skills/triage` skill also straddles `planning` (its actual job) and `skills` (its packaging as a reusable skill file) — filed under `planning` since the taxonomy description for `skills` emphasizes "the tool ships" (product-facing), while triage is a maintainer-only repo skill.

Status: DONE
Summary: Inventoried 16 taxonomy domains (harness, orchestration, context-memory, tooling, config-packaging, skills, hooks, safety, self-improvement, planning, ux, docs-style, quality-gates, testing-evals, workflow, repo-layout) covering roughly 55 distinct features from herdr's docs/next website docs, AGENTS.md/SKILL.md/CONTRIBUTING.md, justfile/flake.nix/nix package, `.codex/` skills, GitHub Actions gates, and top-of-file doc comments across `src/`. Report written to `/home/vantt/projects/research/multiplexer/docs/distillery/reports/distill-herdr-inventory-2026-07-15.md`.
Concerns: `vendor/` directory contents were blocked by a local `scout-block` hook (pattern `vendor`) for both Bash and Read tools in this environment, so the two vendor-patch-tracking entries are sourced secondhand from AGENTS.md's description rather than direct file reads — worth a follow-up pass with vendor access allowed if that mechanism needs deeper inventory. Deep Rust implementation logic (function bodies) was intentionally not read per the doc-first/mechanical-inventory brief.

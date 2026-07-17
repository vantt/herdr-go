---
topic: how-to-use-herdr
date: 2026-07-15
based_on: [herdr@a0678a3]
entries: [herdr:socket-api-control-surface, herdr:workspace-tab-pane-model, herdr:agent-detection-manifests, herdr:shipped-agent-skill-file, herdr:wait-primitives, herdr:native-agent-session-restore]
---

# How to use herdr (deeply)

**Bottom line:** herdr is a Rust terminal multiplexer purpose-built for running and coordinating multiple AI coding agents at once — it is tmux-shaped (server + client, prefix keys, BSP splits) but adds an agent layer: it detects agent identity/status per pane, rolls that state up to tabs/workspaces, and exposes the *entire* thing — layout, panes, agents, waits — over a local JSON socket that the `herdr` CLI, official integrations, and any script or coding agent can drive. Install with `curl -fsSL https://herdr.dev/install.sh | sh`, then just run `herdr` from a project directory — it launches a background server and attaches a client, auto-creating a workspace. The single most important mental model: **workspace → tab → pane** is the org structure (like tmux session→window→pane), every pane may additionally carry an **agent identity + status** (`idle`/`working`/`blocked`/`done`/`unknown`), and literally everything the mouse/keyboard does in the TUI has a 1:1 CLI/socket equivalent — so a human and an AI agent can drive the exact same surface.

---

## 0. Install & first run

```bash
curl -fsSL https://herdr.dev/install.sh | sh          # Linux/macOS
brew install herdr                                     # Homebrew
mise use -g herdr                                      # mise
powershell -ExecutionPolicy Bypass -c "irm https://herdr.dev/install.ps1 | iex"   # Windows beta (preview channel only)
nix run github:ogulcancelik/herdr/v0.x.y               # Nix (also: nix build, nix profile install)
```
Manual binaries: `herdr-linux-x86_64`, `herdr-linux-aarch64`, `herdr-macos-x86_64`, `herdr-macos-aarch64` (Windows: `herdr-windows-x86_64.exe`, preview-only) from GitHub releases.

First run:
```bash
herdr                    # launch or attach to the default background session
```
No socket management needed. If no workspace exists yet, herdr creates one automatically from the launch directory. `herdr --default-config > ~/.config/herdr/config.toml` seeds a full commented config if you want to edit one.

**What gets created on disk:**
| Path | Purpose |
| --- | --- |
| `~/.config/herdr/config.toml` (Linux/macOS) · `%APPDATA%\herdr\config.toml` (Windows) | optional config |
| `~/.config/herdr/session.json` | snapshot-restore state (workspaces/tabs/panes/cwd/layout/focus) |
| `~/.config/herdr/session-history.json` | opt-in pane screen-history replay (`[experimental] pane_history = true`) |
| `~/.config/herdr/plugins.json` | installed/linked plugin registry |
| `~/.config/herdr/herdr.sock` | default session's Unix socket (Windows: named pipe) |
| `~/.config/herdr/sessions/<name>/herdr.sock` | named-session sockets |
| `~/.config/herdr/agent-detection/<agent>.toml` | local override for a bundled/remote detection manifest |
| `~/.config/herdr/herdr.log`, `herdr-client.log`, `herdr-server.log` | rotating logs (`HERDR_LOG=herdr=debug` for more detail) |

Update: `herdr update` (installer-managed installs only — Homebrew/mise/Nix update through their own tools). Channels: `herdr channel show`, `herdr channel set preview`, `herdr channel set stable`. Windows beta defaults to preview and cannot switch to stable yet.

---

## 1. Mental model

**Hierarchy** (opaque, never-reused, never-retargeted IDs — always re-read them from JSON responses, never construct them):
```
workspace (w1)         — project-level container, rolls up agent state for the sidebar
  └─ tab (w1:t1)        — a layout inside the workspace (e.g. "agents", "logs", "server")
       └─ pane (w1:p1)  — a real terminal (BSP split tree: right/down)
            └─ agent    — optional identity+status Herdr assigns to what's running in the pane
```
A **pane** is just a terminal. An **agent** is a semantic layer herdr can attach to a pane's foreground process (via detection, an integration hook, or manual `pane report-agent`). `herdr agent ...` commands only work on panes with agent identity; `herdr pane ...` commands work on any pane. Closed IDs are dead forever; a pane moved cross-workspace gets a *new* public pane id.

**Agent status states** — five, and only five:
| State | Meaning |
| --- | --- |
| `blocked` | needs input/approval/decision |
| `working` | actively running |
| `done` | finished, **not yet seen** (background tab/workspace) |
| `idle` | finished/waiting, **seen** (foreground, or you already looked) |
| `unknown` | herdr can't confidently classify it |

`idle`/`done` are the *same* underlying finished state — the only difference is whether the tab/workspace was in the foreground when it finished. Focusing the pane/tab (or regaining outer-terminal focus) flips an existing `done` to `idle`; it never flips `idle` back to `done` retroactively — a *new* completion while unseen is what produces `done`. Rollups propagate `blocked`/`working` upward: pane → tab → workspace, so the sidebar shows which project needs attention without polling every pane by hand.

**Client/server**: herdr always runs a background server (owns panes/process state) plus one or more attached clients (render the UI). Detach with `ctrl+b q` — server and every agent keep running; `herdr` again reattaches. `herdr server stop` actually ends the session and kills its pane processes. `herdr --no-session` is a single-process escape hatch (debugging/compatibility only).

---

## Part A — Operator mastery (you, the human)

### A1. Layout control

herdr's layout is a **BSP (binary space partition) split tree** — every split is right/down, recursively.

Keyboard (prefix default `ctrl+b`):
| Action | Key |
| --- | --- |
| Split right / down | `prefix+v` / `prefix+minus` |
| Move between panes | `prefix+h/j/k/l` |
| Swap panes | `prefix+shift+h/j/k/l` |
| Zoom focused pane | `prefix+z` |
| Close pane | `prefix+x` |
| Resize mode | `prefix+r` |
| Copy mode | `prefix+[` |

Mouse (first-class, not a fallback): click panes/tabs/workspaces/agents to focus; **drag split borders** to resize; **right-click** for context menus including split/new-tab; **drag-select** text to copy (no `ctrl+c` needed); **double-click** a token to copy it; **ctrl-click** opens OSC-8/plain URL pane links (macOS: ctrl-click while mouse capture is on, or shift-cmd-click / `ui.mouse_capture = false` to bypass to terminal-native).

CLI/API equivalents (every mouse/keyboard layout action has one):
```bash
herdr pane split --current --direction right --ratio 0.333 [--cwd PATH] [--env K=V] [--focus|--no-focus]
herdr pane resize --direction left|right|up|down [--amount FLOAT] [--pane ID|--current]
herdr pane swap --direction left|right|up|down [--pane ID|--current]
herdr pane swap --source-pane ID --target-pane ID
herdr pane move <pane_id> --tab <tab_id> --split right|down [--target-pane ID] [--ratio FLOAT] [--focus|--no-focus]
herdr pane move <pane_id> --new-tab [--workspace ID] [--label TEXT] [--focus|--no-focus]
herdr pane move <pane_id> --new-workspace [--label TEXT] [--tab-label TEXT] [--focus|--no-focus]
herdr pane zoom [<pane_id>|--pane ID|--current] [--toggle|--on|--off]
herdr pane neighbor --direction left|right|up|down [--pane ID|--current]
herdr pane edges [--pane ID|--current]
herdr pane layout [--pane ID|--current]     # returns the tab's full BSP layout snapshot
```
Reason codes come back structured instead of needing string parsing: `pane.swap` → `no_neighbor`/`same_pane`/`not_found`/`cross_tab`; `pane.move` (same-tab) → `same_tab`; either into a zoomed tab → `zoomed_tab`; `pane.zoom` → `single_pane`/`already_zoomed`/`already_unzoomed`.

**Popup/overlay** (session-modal, not a tiled pane — no pane ID, doesn't export `HERDR_PANE_ID`):
```toml
[[keys.command]]
key = "prefix+alt+g"
type = "popup"          # popup | pane (zoomed, closes on exit) | shell (detached) | plugin_action
command = "lazygit"
width = "80%"           # number = cells, "N%" = percentage; omit for half-size default
height = "80%"
description = "run lazygit"
```
Plugin panes support the same idea programmatically: `herdr plugin pane open --plugin ID --entrypoint ID --placement overlay|popup|split|tab|zoomed [--width SIZE] [--height SIZE] [--workspace ID] [--target-pane PANE] [--direction right|down]`.

**Layout export/apply** — portable, declarative topology (not a live-process snapshot):
```bash
herdr layout export     # via socket: layout.export {"tab_id":"w1:t1"} → BSP tree of pane/split nodes
herdr layout apply      # via socket: layout.apply — recreates structure/labels/cwd/env/argv, NOT live PTYs/scrollback
```
(No dedicated top-level `herdr layout` CLI subcommand exists yet — these are raw socket methods `layout.export` / `layout.apply` / `layout.set_split_ratio`; drive them via `herdr api schema` + a socket client, or a plugin.)

### A2. Keybindings

The **prefix model**: press `ctrl+b` (default), release, then press the action key — one reserved chord instead of dozens, so herdr doesn't steal keys from shells/editors/tmux running inside it. `prefix+?` shows every active binding live.

**Five keys first**:
| Action | Key |
| --- | --- |
| New tab | `prefix+c` |
| Split right / down | `prefix+v` / `prefix+minus` |
| Move between panes | `prefix+h/j/k/l` |
| Workspace navigation | `prefix+w` |
| Detach | `prefix+q` |

Everything else (tab 1-9 jump `prefix+1..9`, rename tab `prefix+shift+t`, close tab `prefix+shift+x`, new workspace `prefix+shift+n`, goto picker `prefix+g`, toggle sidebar `prefix+b`) can stay on the mouse.

**Chord-safety** ("going prefix-free"): a direct (no-prefix) chord must survive OS → outer terminal → in-pane program. herdr mapped the defaults of Ghostty/iTerm2/Terminal.app/kitty/WezTerm/Alacritty/Warp/Windows Terminal/GNOME Terminal/Konsole plus GNOME/KDE globals and found **`ctrl+alt`** is the one modifier family left almost untouched everywhere — the safe default for direct bindings. Avoid `ctrl+alt+arrows` (GNOME/Ghostty/Konsole), `ctrl+alt+t` (Ubuntu/Fedora "launch terminal"), `ctrl+alt+l/a` (KDE lock/attention), `ctrl+alt+s/u` (Konsole), `ctrl+alt+f1..f12` (Linux VT switch).
```toml
[keys]
prefix = "ctrl+a"                                            # change the prefix itself
focus_pane_left = ["prefix+h", "ctrl+alt+h"]                  # array = multiple bindings for one action
switch_tab = "prefix+1..9"                                    # indexed jumps
```
`herdr config reset-keys` backs up `config.toml`, strips `[keys]`/`[[keys.command]]`, restores v2 defaults after restart/`reload-config`.

**Copy mode** (`prefix+[`): `h/j/k/l`, tmux-style `w/b/e`, `{`/`}`, PageUp/PageDown, `ctrl+b`/`ctrl+f`, `ctrl+u`/`ctrl+d` to move; `/`/`?` search (case-insensitive unless query has uppercase), `n`/`N` repeat; `v`/Space select, `y`/Enter copy, `q`/Esc cancel. Output stays live during copy mode — it isn't paused.

### A3. Remote & persistence

Two distinct remote paths — pick by what you need:

| Path | What happens | Use when |
| --- | --- | --- |
| `ssh you@server` then `herdr` | herdr runs *entirely* remote (server + client) — tmux-style | already in an SSH shell, phone SSH client, simplest setup, or Windows (native `--remote` isn't in the beta) |
| `herdr --remote workbox` | local herdr is a **thin client**: SSHes in, starts/attaches remote server, streams UI back | want local desktop features bridged (image clipboard paste), local muscle-memory keybindings |

```bash
herdr --remote workbox                       # SSH config host alias
herdr --remote ssh://you@server:2222          # explicit target
herdr --remote workbox --remote-keybindings server   # use remote config's keys instead of local snapshot-at-attach
herdr --remote workbox --session agents       # attach a named session on the remote host
herdr --remote workbox --handoff              # opt into experimental live handoff instead of restart/stop
```
`--remote` supports Linux/macOS x86_64/aarch64 remotes; it checks remote PATH/Homebrew/mise/Nix locations for a matching `herdr`, prompts to install to `~/.local/bin/herdr` interactively (fails non-interactively instead of modifying the host). `HERDR_REMOTE_BINARY=path` forces a local/custom binary for install. Passphrase keys in non-interactive contexts: `ssh-add` first. `[remote] manage_ssh_config = false` disables herdr's private temp SSH config + control-socket connection reuse in favor of plain `ssh`.

**Detach/reattach** (works locally and over either remote path): `ctrl+b q` detaches; run `herdr` again to reattach — server and every agent process keep running the whole time. `herdr server stop` (or `herdr session stop <name>`) actually ends the session and kills pane processes.

**Session survival across restart** — the state-path table that matters:
| Case | Processes keep running | Layout returns | Recent screen returns | Agent conversation resumes |
| --- | --- | --- | --- | --- |
| Detach/reattach | Yes | Yes | Yes (live terminal) | Yes (process never stopped) |
| Server restart | No | Yes (`session.json`) | Only with pane screen history | Only with native agent session restore |
| Update, no `--handoff` | compatible servers may keep running | Yes after restart | only with pane history | only with native resume |
| Update `--handoff` | best-effort | Yes | Yes if handoff succeeds | Yes if handoff succeeds |

**Named sessions** — fully separate runtime namespaces (own panes/tabs/workspaces/sockets/state, shared config file):
```bash
herdr session list [--json]
herdr session attach <name>
herdr session stop <name> [--json]
herdr session delete <name> [--json]
```

**Thin client / bridges** (no full herdr UI needed):
```bash
herdr agent attach reviewer [--takeover]           # direct-attach one agent terminal (ctrl+b q detaches, ctrl+b ctrl+b sends literal ctrl+b)
herdr terminal attach <terminal_id> [--takeover]
herdr terminal session observe <target> [--cols N] [--rows N]   # read-only; newline-delimited JSON terminal.frame (base64 ANSI) + terminal.closed
herdr terminal session control <target> [--takeover] [--cols N] [--rows N]  # writable; reads terminal.input/resize/scroll/release on stdin
```

**Clipboard bridge**: only `herdr --remote` (thin-client mode) can bridge local desktop clipboard (e.g. image paste → remote temp file + path paste); SSH-then-`herdr` cannot reach your local desktop clipboard beyond normal terminal text paste. Remote clipboard image bridging is Unix/macOS-only.

### A4. Config

Location: `~/.config/herdr/config.toml` (Linux/macOS) or `%APPDATA%\herdr\config.toml` (Windows). Fully optional — herdr runs with defaults. `herdr --default-config` prints the full commented default; `herdr --default-config > ~/.config/herdr/config.toml` seeds a starting file. `herdr server reload-config` (or the in-app global menu "reload config") applies most settings live without restarting panes; a few are startup-only.

Settings you'll actually touch:
```toml
onboarding = false                       # skip first-run setup flow

[terminal]
default_shell = "nu"                     # executable name/path for new interactive panes (else $SHELL / /bin/sh / PowerShell)
shell_mode = "auto"                      # auto | login | non_login — macOS auto = login shell for PATH setup
new_cwd = "follow"                       # follow | home | current | fixed path — new pane/tab/workspace cwd policy

[worktrees]
directory = "~/.herdr/worktrees"         # root for git worktree checkouts; layout is <directory>/<repo>/<branch-slug>

[theme]
name = "catppuccin"
auto_switch = true                       # follow host terminal light/dark reports
light_name = "catppuccin-latte"
dark_name = "catppuccin"

[ui.sidebar.agents]                      # per-row token layout for the Agent panel — see A-note below
rows = [["state_icon","workspace","tab"], ["agent"]]
row_gap = 0

[ui.toast]
delivery = "herdr"                       # herdr | terminal | system | off
delay_seconds = 1

[session]
resume_agents_on_restore = true          # native agent session resume after server restart (see B1)

[experimental]
pane_history = false                     # replay recent pane screen contents across a full restart — off by default: can contain secrets/tokens
kitty_graphics = false                   # image rendering for attached local clients
```
Safe-fallback behavior: **any invalid config value falls back to a safe default plus a startup warning** — a bad config never blocks startup.

Sidebar row tokens (Agent panel): `state_icon`, `state_text`, `workspace`, `tab`, `pane`, `agent`, `terminal_title`, `terminal_title_stripped`, `$name` (custom token, set via `pane report-metadata --token name=value`). Space panel tokens: `state_icon`, `state_text`, `workspace`, `branch`, `git_status`, `$name`. Override per-agent with `[ui.sidebar.agents.rows_by_agent]` keyed by canonical agent id (`claude`, `codex`, `pi`, …) — an override *replaces* `rows`, it doesn't extend it.

Full canonical key/type/default/allowed-value table: `herdr --default-config` or the generated config reference (all keys, filterable) — too large to duplicate here faithfully.

### A5. Everyday workflows

**Running agents in panes** — just run the executable, herdr detects it:
```bash
herdr                    # launch/attach
claude                   # or codex, pi, opencode, omp, ... — herdr auto-detects and tracks status
```
The sidebar shows `working`/`blocked`/`done`/`idle` across every workspace — that's the core workflow: start several agents in parallel, glance at the sidebar for which one needs a decision vs. which is still running vs. ready to review.

**Worktrees as workspaces** — a Git worktree checkout is a first-class herdr workspace with provenance, grouped under its parent repo workspace:
```bash
herdr worktree create [--workspace ID | --cwd PATH] [--branch NAME] [--base REF] [--path PATH] [--label TEXT] [--focus|--no-focus] [--json]
herdr worktree open   [--workspace ID | --cwd PATH] (--path PATH | --branch NAME) [--label TEXT] [--focus|--no-focus] [--json]
herdr worktree list   [--workspace ID | --cwd PATH] [--json]
herdr worktree remove --workspace ID [--force] [--json]
```
`worktree create` checks out an existing local branch if `--branch` already exists, else creates it from `--base`/`HEAD`; default path is `<worktrees.directory>/<repo>/<branch-slug>`. `worktree remove` runs `git worktree remove` (never deletes the branch); use `--force` when Git refuses a dirty checkout. **`workspace close` only closes herdr state — it never deletes files or branches.** Deleting a checkout is the explicit, separate `worktree remove` action.

**Troubleshooting quick hits**:
```bash
herdr -V && herdr status              # version + client/server/protocol compatibility
herdr server stop && herdr            # updated binary but old server still running → force new server
HERDR_LOG=herdr=debug herdr           # verbose logs at ~/.config/herdr/herdr*.log
```
Enter/Tab/Backspace double-firing → outdated outer terminal (fix: kitty ≥0.33.0, foot ≥1.20.0, Alacritty ≥0.15.0 — Kitty keyboard protocol release-byte bug). Alt+Left/Right showing `;3D`/`;3C` → add zsh `bindkey` for modified-arrow word nav, or map in `kitty.conf`; herdr deliberately doesn't rewrite these since apps may want raw Alt+arrows.

---

## Part B — Agent-driving mastery

### B1. How herdr understands agent types

Two independent sources feed pane state, and **each pane has exactly one status authority at a time**:

1. **Screen-manifest detection** (`src/detect/`) — a TOML rule engine reads the pane's *live bottom-of-buffer* text (never scrolled viewport, never scrollback) on a period and matches AND/OR gates of `contains`/`regex`/`line_regex`/OSC title/progress evidence against per-agent manifests (19 bundled `.toml` files) to classify `idle`/`working`/`blocked`. Manifests hot-reload from `herdr.dev/agent-detection/index.toml` without a restart; precedence is **local override (`~/.config/herdr/agent-detection/<agent>.toml`) > newer of {cached remote, bundled binary manifest}**.
2. **Lifecycle hook/plugin authority** — an installed official integration reports state directly over the socket. When one is installed *and actively reporting* for a pane, it becomes authoritative for `idle`/`working`/`blocked` and screen-manifest fallback is **disabled** for that pane (avoids two competing truths).

| Authority type | Agents | Effect |
| --- | --- | --- |
| Lifecycle authority (state + session) | Pi, OMP, Kimi Code CLI, OpenCode, Kilo Code CLI, Hermes Agent, MastraCode | hook/plugin reports author `idle`/`working`/`blocked` directly |
| Session identity only | Claude Code, Codex, GitHub Copilot CLI, Devin CLI, Droid, Qoder CLI, Cursor Agent CLI | state stays on screen-manifest detection (hooks don't cover every lifecycle transition — permission cancels, interrupts) |
| Detection only, no integration | Amp, Grok CLI, Antigravity CLI, Kiro CLI, Maki (also: Gemini CLI, Cline — less thoroughly tested) | screen manifest only |

**`blocked` is deliberately strict**: only fires on a known visible approval/question/permission screen match; unmatched known-agent screens fall back to `idle` (`default_known_agent_idle_fallback` in explain output) rather than guessing blocked.

**Sandbox/VM escape hatch**: `HERDR_AGENT=<agent> fence -- claude` tells herdr which manifest to use when a wrapper (VM, Bubblewrap, `fence`) hides the real process from host `/proc`. Scope it to the one foreground process — don't export it globally.

**Debug detection**:
```bash
herdr agent read <target> --source detection --format text     # exact bottom-buffer snapshot detection sees
herdr agent explain <target> [--json|--verbose]                 # live: agent, state, manifest source/version, matched rule + evidence, fallback/skip reasons
herdr agent explain --file screen.txt --agent codex --json       # explain a saved fixture offline
herdr server agent-manifests [--json]                            # active manifest sources + remote-update diagnostics
herdr server update-agent-manifests [--json]                     # fetch + reload immediately
herdr server reload-agent-manifests                              # reload after editing a local override
```

**Integrations** — install per agent to get hook/plugin reporting instead of detection-only:
```bash
herdr integration install {pi|omp|claude|codex|copilot|devin|droid|kimi|opencode|kilo|hermes|qodercli|cursor|mastracode}
herdr integration uninstall <same-list>
herdr integration status [--outdated-only]
```

**Native session resume** — after a full server restart, herdr relaunches the agent's own resume command instead of a plain shell, if a current-enough integration reported a native session reference:
| Agent | Min integration version | Resume command |
| --- | --- | --- |
| Pi | 2 | `pi --session <path-or-id>` |
| OMP | 3 | `omp --resume=<path-or-id>` |
| Claude Code | 6 | `claude --resume <id>` |
| Codex | 5 | `codex resume <id>` |
| Cursor Agent CLI | 1 | `cursor-agent --resume <id>` |
| GitHub Copilot CLI | 2 | `copilot --resume=<id>` |
| Devin CLI | 2 | `devin --resume <id>` |
| Droid | 2 | `droid --resume <id>` |
| Kimi Code CLI | 3 | `kimi --session <id>` |
| Qoder CLI | 2 | `qodercli --resume <id>` |
| OpenCode | 5 | `opencode --session <id>` |
| Kilo Code CLI | 1 | `kilo --session <id>` |
| Hermes Agent | 2 | `hermes --resume <id>` |
| MastraCode | 1 | `mastracode --thread <id>` |

Toggle: `[session] resume_agents_on_restore = false`. `herdr integration status` shows installed versions; reinstall to upgrade.

**Custom/manual reporting** (for agents with no integration, or your own hooks):
```bash
herdr pane report-agent <pane_id> --source ID --agent LABEL --state idle|working|blocked|unknown [--message TEXT] [--seq N] [--agent-session-id ID] [--agent-session-path PATH]
herdr pane report-agent-session <pane_id> --source ID --agent LABEL [--seq N] [--agent-session-id ID] [--agent-session-path PATH] [--session-start-source SOURCE]
herdr pane release-agent <pane_id> --source ID --agent LABEL [--seq N]
herdr pane report-metadata <pane_id> --source ID [--agent LABEL] [--applies-to-source ID] [--title TEXT|--clear-title] [--display-agent TEXT|--clear-display-agent] [--state-label STATUS=TEXT] [--clear-state-labels] [--token NAME=VALUE] [--clear-token NAME] [--seq N] [--ttl-ms N]
```
`report-agent` sets *semantic* state (drives waits/notifications/rollups). `report-metadata` is *display-only* (title, display name, per-state labels, `$name` tokens) and never overrides lifecycle authority — use it for cosmetic hooks that sit next to an existing integration.

### B2. The socket API

**Transport**: newline-delimited JSON over a local Unix domain socket (Windows: named pipe). One request per line; response echoes the same `id`.
```json
{"id":"req_1","method":"ping","params":{}}
{"id":"req_1","result":{"type":"pong"}}
```
Resolution order for which socket: (1) explicit CLI `--session <name>`, (2) `HERDR_SOCKET_PATH`, (3) `HERDR_SESSION=<name>`, (4) default session socket. Errors: `{"id":..., "error":{"code":"not_found","message":"..."}}`.

**Method surface by area** (dot notation — the same surface the CLI, integrations, and third-party clients all use):
| Area | Methods |
| --- | --- |
| Server | `ping`, `server.stop`, `server.reload_config`, `server.agent_manifests`, `server.reload_agent_manifests` |
| Notification | `notification.show` |
| Client | `client.window_title.set`, `client.window_title.clear` |
| Session | `session.snapshot` |
| Workspace | `workspace.create`, `.list`, `.get`, `.focus`, `.rename`, `.move`, `.report_metadata`, `.close` |
| Worktree | `worktree.list`, `.create`, `.open`, `.remove` |
| Tab | `tab.create`, `.list`, `.get`, `.focus`, `.rename`, `.move`, `.close` |
| Pane | `pane.split`, `.swap`, `.move`, `.zoom`, `.layout`, `.process_info`, `.neighbor`, `.edges`, `.focus_direction`, `.resize`, `.list`, `.current`, `.get`, `.rename`, `.send_text`, `.send_keys`, `.send_input`, `.read`, `.graphics.info/.set/.clear/.stream`, `.report_agent`, `.report_agent_session`, `.report_metadata`, `.clear_agent_authority`, `.release_agent`, `.close`, `.wait_for_output` |
| Popup | `popup.close` |
| Layout | `layout.export`, `.apply`, `.set_split_ratio` |
| Agent | `agent.list`, `.get`, `.read`, `.explain`, `.send`, `.rename`, `.focus`, `.start` |
| Events | `events.subscribe`, `.wait` |
| Integrations | `integration.install`, `.uninstall` |
| Plugins | `plugin.link`, `.list`, `.unlink`, `.enable`, `.disable`, `.action.list`, `.action.invoke`, `.log.list`, `.pane.open`, `.pane.focus`, `.pane.close` |

`herdr api schema` (short summary) / `herdr api schema --json` (full JSON Schema of requests/responses/errors/events, generated via `schemars` from Rust wire types — self-describing, not hand-maintained) / `herdr api schema --output PATH`.

**Bootstrap pattern** for stateful clients:
```bash
herdr api snapshot     # CLI wrapper for session.snapshot — one-time bootstrap, NOT a subscription
```
`session.snapshot` returns version/protocol metadata, focused ids, and full workspace/tab/pane/agent/layout records. After reading it once, subscribe to events and update your local cache incrementally — call `session.snapshot` again only after reconnecting or if the cache may be stale.
```json
{"id":"sub_1","method":"events.subscribe","params":{"subscriptions":[
  {"type":"pane.agent_status_changed","pane_id":"w1:p1","agent_status":"blocked"}
]}}
```
Event families: workspace (`workspace.created/.updated/.metadata_updated/.renamed/.moved/.closed/.focused`), tab (`.created/.closed/.focused/.renamed/.moved`), pane (`.created/.updated/.closed/.focused/.moved/.exited/.agent_detected/.output_matched/.agent_status_changed/.scroll_changed`), layout (`.updated`), worktree (`.created/.opened/.removed`).

### B3. Driving herdr from an agent via CLI

**Gate**: only act if inside a herdr-managed pane —
```bash
test "${HERDR_ENV:-}" = 1   # if this fails: say you're not running inside Herdr, and stop
```
Injected context (per managed pane): `HERDR_SOCKET_PATH`, `HERDR_ENV=1`, `HERDR_WORKSPACE_ID`, `HERDR_TAB_ID`, `HERDR_PANE_ID`. Prefer `--current` on pane commands rather than an omitted target (which falls back to the *UI-focused* pane, possibly someone else's).

Install the shipped agent-facing contract into any coding agent that supports reusable skills:
```bash
npx skills add ogulcancelik/herdr --skill herdr -g   # -g installs globally; omit for project-only
```
(Source of truth / manual fallback: `SKILL.md` at repo root.) A **different** doc, `herdr.dev/agent-guide.md`, is for an agent *teaching a human* to set up herdr — not the same job as `SKILL.md`, which teaches an agent to *operate* herdr.

**Split a sibling pane without stealing focus, run something, read it back**:
```bash
herdr pane split --current --direction right --no-focus   # read result.pane.pane_id from the JSON response — never guess it
herdr pane rename <returned-pane-id> "reviewer"
herdr pane run <returned-pane-id> "just test"              # submits text + Enter atomically (prefer over send-text + send-keys Enter)
herdr wait output <returned-pane-id> --match "test result" --timeout 120000
herdr pane read <returned-pane-id> --source recent-unwrapped --lines 120
```
Read `--source` options: `visible` (current viewport, UI feedback loops), `recent` (scrollback, soft-wrapped), `recent-unwrapped` (scrollback, unwrapped — best for logs/transcripts), `detection` (bottom-buffer snapshot, agent-detection debugging). `--format ansi` when color/styling is evidence, else `text`.

**Start and coordinate an actual agent** (interactive TUI, not a one-shot prompt-as-argv):
```bash
herdr pane split --current --direction right --no-focus
herdr pane rename <returned-pane-id> "reviewer"
herdr pane run <returned-pane-id> "codex"                              # or claude, pi, opencode, omp, ... — its normal launch, no non-interactive flags
herdr pane get <returned-pane-id>                                       # inspect agent_status
herdr wait agent-status <returned-pane-id> --status idle --timeout 30000
herdr pane run <returned-pane-id> "Review the current diff and report only actionable findings."
herdr wait agent-status <returned-pane-id> --status working --timeout 30000
herdr wait agent-status <returned-pane-id> --status done --timeout 120000   # 'done' if it may be in a background tab
herdr pane read <returned-pane-id> --source recent-unwrapped --lines 120
```
Or as a dedicated agent target (shows up in `agent list`, addressable/waitable by name):
```bash
herdr agent start reviewer -- claude                              # minimal form: current workspace/tab, all flags optional
herdr agent start reviewer --cwd ~/project --split right -- pi
herdr agent start docs --workspace w1 --tab w1:t1 -- claude
herdr agent wait reviewer --status idle --timeout 30000
```
`agent start` argument breakdown: `<name>` is your own reference label for later commands (`agent wait <name>`, `agent send <name>`, `agent read <name>`) — not a system-generated ID, so pick something memorable per role (e.g. `reviewer`, `docs`); `--` is the mandatory separator, everything after it is the literal launch command (`claude`, `codex`, `pi`, `opencode`, `omp`, ...) run with no non-interactive flags; `--cwd PATH` sets the new pane's working directory; `--workspace ID`/`--tab ID` place it into an *existing* workspace/tab (IDs come from `workspace.list`/`tab.list` — never hand-construct them); `--split right|down` picks the split direction when a new pane is created; `--focus`/`--no-focus` controls whether the new pane steals focus or starts in the background.

`herdr pane ...` vs `herdr agent ...`: use `pane` for ordinary terminals/servers/tests/shells (`pane split` + `pane run` for `cargo test`, not `agent start`); use `agent` only when the terminal is intentionally an agent target you'll coordinate with by name.

**`wait output` vs `wait agent-status`** — the two first-class wait primitives:
```bash
herdr wait output <pane_id> --match <text> [--source visible|recent|recent-unwrapped] [--lines N] [--timeout MS] [--regex] [--raw]
herdr wait agent-status <pane_id> --status idle|working|blocked|done|unknown [--timeout MS]
```
Both match immediately if already true, else block for the next transition; a timeout exits status `1`. Use `wait output` for generic command/server completion, `wait agent-status` for coding-agent lifecycle. **Always inspect before waiting** — read current output/state first, then wait for the *next* transition, since a wait can't retroactively see something that already happened.

**Coordinating multiple agents** (wait-on-each-other): have agent A finish, `wait agent-status <A> --status idle`, then read A's transcript and feed it into agent B's `pane run` — the same wait/read primitives compose across as many panes as you split.

**Safety rules** (from `SKILL.md`): use `--no-focus` for background work; use `--current` or an explicit ID, never rely on another client's focused pane; parse every ID from JSON, never from sidebar order; don't close panes/tabs/workspaces/sessions you didn't create unless asked; never run `herdr server stop` from an active session unless explicitly intended; never kill the main herdr process — use a named test session for anything that needs an isolated server.

### B4. Extending

**Plugins** — a plugin is just a `herdr-plugin.toml` manifest plus argv commands herdr can launch; there is **no separate plugin SDK** — the entire `herdr` CLI (every command above) is the plugin API, invoked via `HERDR_BIN_PATH` for portability across Unix sockets / Windows named pipes.
```toml
id = "example.worktree-bootstrap"
name = "Worktree Bootstrap"
version = "0.1.0"
min_herdr_version = "0.7.0"     # required; herdr refuses to link/install if this exceeds the running binary
platforms = ["linux", "macos", "windows"]

[[build]]
command = ["bun", "install"]                 # GitHub installs only; plugin.link does not run builds

[[actions]]
id = "bootstrap"
title = "Bootstrap worktree"
contexts = ["workspace"]
command = ["bun", "run", "bootstrap.ts"]

[[events]]
on = "worktree.created"                       # validated against known event names at link time (warning, not hard error, if unknown)
command = ["bun", "run", "bootstrap.ts"]

[[panes]]
id = "board"
placement = "overlay"                          # overlay (default) | popup | split | tab | zoomed
command = ["bun", "run", "board.ts"]

[[link_handlers]]
id = "github-issue"
pattern = "^https://github\\.com/[^/]+/[^/]+/(issues|pull)/[0-9]+$"
action = "bootstrap"
```
```bash
herdr plugin install <owner>/<repo>[/subdir...] [--ref REF] [--yes]     # GitHub shorthand only; clones with git, shows trust preview interactively
herdr plugin uninstall <plugin_id|owner/repo[/subdir...]>
herdr plugin link <path> [--disabled]           # local authoring/dev — no build step, no trust preview
herdr plugin unlink <plugin_id>                 # unregisters only, leaves files
herdr plugin list [--plugin ID] [--json]
herdr plugin enable <plugin_id> / disable <plugin_id>
herdr plugin config-dir <plugin_id>              # stable dir for .env/user config, seeded from legacy locations
herdr plugin action list [--plugin ID]
herdr plugin action invoke <action_id> [--plugin ID]   # use qualified plugin.id.action if action ids collide
herdr plugin log list [--plugin ID] [--limit N]
herdr plugin pane open --plugin ID --entrypoint ID [--placement ...] [--width SIZE] [--height SIZE] ...
herdr plugin pane focus <pane_id> / close <pane_id>
```
Runtime env injected into plugin commands: `HERDR_SOCKET_PATH`, `HERDR_BIN_PATH`, `HERDR_ENV=1`, `HERDR_PLUGIN_ID`, `HERDR_PLUGIN_ROOT`, `HERDR_PLUGIN_CONFIG_DIR`, `HERDR_PLUGIN_STATE_DIR`, `HERDR_PLUGIN_CONTEXT_JSON`, plus `HERDR_WORKSPACE_ID`/`HERDR_TAB_ID`/`HERDR_PANE_ID` when available; action commands also get `HERDR_PLUGIN_ACTION_ID`, event hooks get `HERDR_PLUGIN_EVENT(_JSON)`, pane commands get `HERDR_PLUGIN_ENTRYPOINT_ID`.

**Trust model** — explicit, no sandboxing: a plugin runs as your user with your environment and full CLI access, same as any editor/shell/agent extension. `herdr plugin install` previews source + commands and requires confirmation (or `--yes`) in interactive terminals; `herdr` validates the manifest and isolates each plugin's config/state directory but does **not** review or sandbox plugin behavior — vetting is on you.

**Marketplace** — an automatic, unreviewed index of public GitHub repos tagged `herdr-plugin` (browse at herdr.dev/plugins; refreshes every ~30 min). Getting listed = tag your public repo with the `herdr-plugin` topic; nothing else. Forks/archived repos excluded. The index doesn't parse the manifest (no `id`/`platforms`/`min_herdr_version` shown yet) — same trust guidance as direct install applies.

### B5. Auto-provisioning agents when opening a project

**No built-in support**: herdr has no per-project config file (no `.herdr.toml` or similar read from a project directory) and no "start these agents when this workspace opens" setting. The only config file is the single global `~/.config/herdr/config.toml` (A4) — confirmed against `src/config.rs`: there is no project-local config discovery in the codebase.

**How to build it yourself** — two composable primitives, no core changes needed:
1. **Plugin event hook on `workspace.created`** — confirmed in `PLUGIN_HOOK_EVENT_KINDS` (`src/api/schema/events.rs`), a valid hookable event name (`[[events]] on = "workspace.created"`). herdr auto-creates a workspace when you run `herdr` in a directory with no workspace yet (§0), which fires this event; your hook script gets `HERDR_PLUGIN_EVENT_JSON` (workspace/cwd info) and can call `$HERDR_BIN_PATH agent start <name> -- <agent>` for whichever agents that project needs. The hook fires for *every* new workspace — your script must match cwd/project itself before deciding what (if anything) to start.
2. **`layout.export`/`layout.apply`** to make the bootstrap repeatable/declarative: export a tab you've set up once (panes whose argv is `claude`/`codex`/...), then `layout.apply` recreates structure/labels/cwd/env/**argv** on a fresh tab — i.e. it re-launches those same agent commands (not a live-process snapshot, a fresh re-invoke).

**Cookbook precedent — read carefully, it is not a ready answer**: the official examples repo (`ogulcancelik/herdr-plugin-examples`, `dev-layout-bootstrap/`) sounds like exactly this use case but, verified by reading its source directly (cloned and inspected, not assumed from the README alone):
- It's a **manual `[[actions]]` invoke** (`herdr plugin action invoke setup`, optionally bound to a key) — **no `[[events]]` hook at all**, so it does **not** trigger automatically on workspace/project open.
- The 3 panes it creates run generic dev tooling (`nvim .`, `ls -la`, `git status --short`) — **not coding agents**; `claude`/`codex`/etc never appear in it.
- It builds the layout imperatively via `pane split`/`pane rename`/`pane run` calls in Lua — it does **not** use `layout.export`/`layout.apply`.

So it demonstrates the *scripted pane-bootstrap technique* (a useful Lua/CLI-calling-convention reference), not a working "open project → auto-run agents" implementation. Getting that actual behavior means forking the pattern: add `[[events]] on = "workspace.created"` to the manifest, and swap its `pane run "nvim ."`-style calls for `agent start <name> -- <agent>` calls gated on the new workspace's cwd.

---

## Part C — Limits, gotchas, and requirements

**Platform support**: stable releases = Linux + macOS (x86_64/aarch64). Native Windows is **preview-only beta** (ConPTY-based, not the Unix PTY model) — features may graduate to stable, stay preview, or be cut based on real-use feedback.

**Windows beta — not supported at all**: direct terminal attach, `herdr --remote` from the Windows binary, live server handoff, Unix fd handoff, Unix foreground process groups, remote clipboard image bridge, prefix input-source switching, signed-binary/SmartScreen avoidance. Windows workaround for remote: SSH into the server and run `herdr` there (same as any Unix remote-via-SSH path).

**Windows beta — partial/unverified**: live cwd after shell `cd` (process field doesn't reliably track later `cd`s — use integrations/shell OSC7 for live cwd), clipboard image paste to agents (unverified), Kitty graphics rendering (unverified — leave `kitty_graphics = false`), host cursor rendering (`host_cursor = "auto"` draws herdr's own cursor on native Windows/WSL by default; set `native` to opt back into the terminal's blinking cursor).

**What does/doesn't survive restart** (recap from A3's table): only detach/reattach preserves *running processes*; a full server restart always loses processes — layout returns from `session.json`, recent screen only returns with opt-in `pane_history`, and agent conversations only resume via native session restore (current-enough integration + not disabled).

**Non-sandboxed plugins**: no runtime isolation, full CLI + your environment — see B4 trust model. There is no Herdr-managed plugin storage API in v1; plugins own their own files/schema/migrations/cleanup.

**Things I could not verify from the read docs/code** (flag honestly, not fabricated):
- No dedicated top-level `herdr layout export|apply|set-split-ratio` CLI subcommand was found in `src/cli/*.rs` or `src/cli.rs` — only the raw socket methods `layout.export`/`layout.apply`/`layout.set_split_ratio` are documented (socket-api.mdx). If you need this from a shell script rather than a socket client, you likely have to hit the socket directly or via a plugin.
- Exact list of built-in `[theme].name` values (e.g. `catppuccin`, `tokyo-night`, `gruvbox`, …) is generated into the config reference component at doc-build time and wasn't directly enumerable from the source files read; use `herdr --default-config` or the live config reference for the authoritative list.
- `agent.explain`/`server.agent_manifests` require a server that already supports the method — after upgrading the herdr binary, restart or live-handoff before relying on newly added explain/manifest fields (stated directly in socket-api.mdx).

---

## Appendix — complete command reference

### CLI subcommands (grouped by area, from `src/cli/*.rs` + `src/cli.rs`)

**Top-level**: `herdr [--no-session] [--session NAME] [--remote TARGET [--remote-keybindings local|server]] [--handoff] [--default-config] [-V|--version] [-h|--help]`

**Launch/update/status**:
```
herdr update [--handoff]
herdr channel show
herdr channel set stable|preview
herdr config check
herdr config reset-keys
herdr status [--json]
herdr status server [--json]
herdr status client [--json]
```

**completion** — `herdr completion|completions <bash|elvish|fish|powershell|zsh>`

**api** — `herdr api snapshot` · `herdr api schema [--json | --output PATH]`

**server** — `herdr server` (run headless) · `stop` · `reload-config` · `agent-manifests [--json]` · `update-agent-manifests [--json]` · `reload-agent-manifests` · `live-handoff [--import-exe PATH] [--expected-protocol N] [--expected-version V]`

**session** — `herdr session list [--json]` · `attach <name>` · `stop <name> [--json]` · `delete <name> [--json]`

**workspace** — `list` · `create [--cwd PATH] [--label TEXT] [--env K=V] [--focus|--no-focus]` · `get <id>` · `focus <id>` · `rename <id> <label>` · `report-metadata <id> --source ID [--token N=V] [--clear-token N] [--seq N] [--ttl-ms N]` · `close <id>`

**worktree** — `list [--workspace ID|--cwd PATH] [--json]` · `create [...] [--branch NAME] [--base REF] [--path PATH] [--label TEXT] [--focus|--no-focus] [--json]` · `open [...] (--path PATH|--branch NAME) [...] [--json]` · `remove --workspace ID [--force] [--json]`

**tab** — `list [--workspace ID]` · `create [--workspace ID] [--cwd PATH] [--label TEXT] [--env K=V] [--focus|--no-focus]` · `get <id>` · `focus <id>` · `rename <id> <label>` · `close <id>`

**pane** — `list [--workspace ID]` · `current [--pane ID|--current]` · `get <id>` · `layout [...]` · `process-info [...]` · `neighbor --direction D [...]` · `edges [...]` · `focus --direction D [...]` · `resize --direction D [--amount F] [...]` · `zoom [<id>|...] [--toggle|--on|--off]` · `rename <id> <label>|--clear` · `read <id> [--source S] [--lines N] [--format text|ansi] [--ansi]` · `split [<id>|...] --direction right|down [--ratio F] [--cwd PATH] [--env K=V] [--focus|--no-focus]` · `swap --direction D [...] | --source-pane ID --target-pane ID` · `move <id> --tab ID --split D [--target-pane ID] [--ratio F] [...] | --new-tab [...] | --new-workspace [...]` · `close <id>` · `send-text <id> <text>` · `send-keys <id> <key...>` · `run <id> <command>` · `report-agent <id> --source ID --agent L --state S [...]` · `report-agent-session <id> --source ID --agent L [...]` · `release-agent <id> --source ID --agent L [--seq N]` · `report-metadata <id> --source ID [...]`

**agent** — `list` · `get <target>` · `read <target> [--source S] [--lines N] [--format text|ansi] [--ansi]` · `send <target> <text>` · `rename <target> <name>|--clear` · `focus <target>` · `wait <target> --status idle|working|blocked|unknown [--timeout MS]` · `attach <target> [--takeover]` · `start <name> [--cwd PATH] [--workspace ID] [--tab ID] [--split D] [--env K=V] [--focus|--no-focus] -- <argv...>` · `explain <target>|--file PATH --agent LABEL [--json|--verbose]`

**wait** — `output <pane_id> [--match TEXT] [--source S] [--lines N] [--timeout MS] [--regex] [--raw]` · `agent-status <pane_id> --status idle|working|blocked|done|unknown [--timeout MS]`

**terminal** — `attach <terminal_id> [--takeover]` · `session observe <target> [--cols N] [--rows N]` · `session control <target> [--takeover] [--cols N] [--rows N]` · `title set <title>` · `title clear`

**notification** — `show <title> [--body TEXT] [--position top-left|top-right|bottom-left|bottom-right] [--sound none|done|request]`

**integration** — `install <pi|omp|claude|codex|copilot|devin|droid|kimi|opencode|kilo|hermes|qodercli|cursor|mastracode>` · `uninstall <same>` · `status [--outdated-only]`

**plugin** — `install <owner>/<repo>[/subdir...] [--ref REF] [--yes]` · `uninstall <plugin_id|owner/repo[/subdir...]>` · `link <path> [--disabled]` · `unlink <plugin_id>` · `enable <plugin_id>` · `disable <plugin_id>` · `list [--plugin ID] [--json]` · `config-dir <plugin_id>` · `action list [--plugin ID]` · `action invoke <action_id> [--plugin ID]` · `log list [--plugin ID] [--limit N]` · `pane open --plugin ID --entrypoint ID [--placement P] [--width S] [--height S] [--workspace ID] [--target-pane ID] [--direction D] [--cwd PATH] [--env K=V] [--focus|--no-focus]` · `pane focus <pane_id>` · `pane close <pane_id>`

### Socket API methods (by area, from `docs/next/website/src/content/docs/socket-api.mdx` + `src/api/schema/*.rs`)

| Area | Methods |
| --- | --- |
| Server | `ping`, `server.stop`, `server.reload_config`, `server.agent_manifests`, `server.reload_agent_manifests` |
| Notification | `notification.show` |
| Client | `client.window_title.set`, `client.window_title.clear` |
| Session | `session.snapshot` |
| Workspace | `workspace.create`, `.list`, `.get`, `.focus`, `.rename`, `.move`, `.report_metadata`, `.close` |
| Worktree | `worktree.list`, `.create`, `.open`, `.remove` |
| Tab | `tab.create`, `.list`, `.get`, `.focus`, `.rename`, `.move`, `.close` |
| Pane | `pane.split`, `.swap`, `.move`, `.zoom`, `.layout`, `.process_info`, `.neighbor`, `.edges`, `.focus_direction`, `.resize`, `.list`, `.current`, `.get`, `.rename`, `.send_text`, `.send_keys`, `.send_input`, `.read`, `.graphics.info`, `.graphics.set`, `.graphics.clear`, `.graphics.stream`, `.report_agent`, `.report_agent_session`, `.report_metadata`, `.clear_agent_authority`, `.release_agent`, `.close`, `.wait_for_output` |
| Popup | `popup.close` |
| Layout | `layout.export`, `.apply`, `.set_split_ratio` |
| Agent | `agent.list`, `.get`, `.read`, `.explain`, `.send`, `.rename`, `.focus`, `.start` |
| Events | `events.subscribe`, `.wait` |
| Integrations | `integration.install`, `.uninstall` |
| Plugins | `plugin.link`, `.list`, `.unlink`, `.enable`, `.disable`, `.action.list`, `.action.invoke`, `.log.list`, `.pane.open`, `.pane.focus`, `.pane.close` |

Event names (subscribable via `events.subscribe`): `workspace.{created,updated,metadata_updated,renamed,moved,closed,focused}`, `tab.{created,closed,focused,renamed,moved}`, `pane.{created,updated,closed,focused,moved,exited,agent_detected,output_matched,agent_status_changed,scroll_changed}`, `layout.updated`, `worktree.{created,opened,removed}`.

Environment variables (CLI + pane context): `HERDR_CONFIG_PATH`, `HERDR_SESSION`, `HERDR_SOCKET_PATH`, `HERDR_ENV`, `HERDR_PANE_ID`, `HERDR_TAB_ID`, `HERDR_WORKSPACE_ID`, `HERDR_LOG`, `HERDR_DISABLE_SOUND`, `HERDR_AGENT` (sandbox detection hint), `HERDR_BIN_PATH` (plugin CLI-as-API), `HERDR_REMOTE_BINARY` (custom remote-attach binary), `HERDR_ACTIVE_WORKSPACE_ID`/`HERDR_ACTIVE_TAB_ID`/`HERDR_ACTIVE_PANE_ID`/`HERDR_ACTIVE_PANE_CWD` (custom command keybindings), `HERDR_PLUGIN_ID`/`HERDR_PLUGIN_ROOT`/`HERDR_PLUGIN_CONFIG_DIR`/`HERDR_PLUGIN_STATE_DIR`/`HERDR_PLUGIN_CONTEXT_JSON`/`HERDR_PLUGIN_ACTION_ID`/`HERDR_PLUGIN_EVENT(_JSON)`/`HERDR_PLUGIN_ENTRYPOINT_ID`/`HERDR_PLUGIN_CLICKED_URL`/`HERDR_PLUGIN_LINK_HANDLER_ID` (plugin runtime).

---

## Coverage notes

**Read in full**: `README.md`, `SKILL.md`, `AGENTS.md` (repo root); all 18 requested English `docs/next/website/src/content/docs/*.mdx` files (quick-start, concepts, cli-reference, keyboard, configuration, config-reference, persistence-remote, how-to-work, session-state, agents, integrations, agent-skill, socket-api, plugins, marketplace, install, troubleshooting, windows-beta, index). All `src/cli/*.rs` files (pane, agent, workspace, tab, worktree, server, plugin, integration, api, spec, status, notification, runtime, completion) plus the CLI top-level dispatcher (`src/cli.rs`, for `session`/`channel`/`config`/`update`, since there's no `src/cli/mod.rs`). `src/api/mod.rs`, `src/api/wait.rs`. Cross-checked the socket method-name list in `socket-api.mdx` directly against `Method::` enum usage in `src/api/mod.rs::request_changes_ui` and the CLI's `Method::*` construction — matched, no discrepancies found.

**Skimmed, not fully read**: `src/api/schema/*.rs` individual files (agents.rs, common.rs, events.rs, integrations.rs, panes.rs, plugins.rs, response.rs, server.rs, session.rs, tabs.rs, tests.rs, workspaces.rs, worktrees.rs) — relied on the mdx docs (which are doc-comment-derived) plus targeted greps rather than reading every struct definition; `src/detect/mod.rs` and `src/agent_resume.rs` doc-comment headers only, not the full detection/resume implementation; `docs/distillery/reports/distill-herdr-inventory-2026-07-15.md` and `docs/distillery/sources/herdr.md` (pre-existing inventory) for orientation only.

**Gaps / unverified** (see Part C for detail): no CLI subcommand found for `layout.export`/`layout.apply`/`layout.set_split_ratio` — socket-only as far as the read code/docs show; exact built-in theme name list is doc-build-generated and wasn't in the raw mdx; did not fetch any herdr.dev page externally — everything above is sourced from the local `upstreams/herdr` clone at `a0678a3`.

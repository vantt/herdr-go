# Reading Map

Where each area of this project lives. bee-scribing owns this file: it is updated whenever an area spec is created or moved. Read this before any broad search — it answers "where does X live" without a grep.

| Area | Code entry points | Key types | Spec |
|---|---|---|---|
| security | `src/security/{paths,slug,redact}.rs` | `Boundary`, `sanitize_slug`, `redact` | — |
| config | `src/config/mod.rs` | `Config`, `Secrets`, `ConfigError` | `docs/specs/installation.md` (partial; native roots and token lifecycle) |
| doctor | `src/doctor/{mod,checks,prompt,edit}.rs` | `Check`, guided fixes, settings editor | `docs/specs/doctor.md` (partial; diagnose/fix/edit surface) |
| herdr port | `src/herdr/{mod,wire,fake,socket}.rs` | `Herdr` trait, `SocketHerdr` (platform-local client), `Snapshot` (agents, workspaces, tabs, panes, layouts, global focus), `Pane`, `PaneLayout`, `AgentStatus`, `ScreenRead`, `FakeHerdr` | `docs/specs/herdr-port.md` (partial; screen-read and input detail still open) |
| web + observe | `src/web/{mod,auth,api,create,screen}.rs` | `AppState`, `AuthSession`, `AgentRow`/`ShellRow`/`AgentsResponse` (switcher list, agents + zero-agent-workspace shells), `read_screen`/`send_reply` (poll screen + reply), `create_pane`/`create_agent` (open a shell / start an agent) | `docs/specs/web-api.md` (partial; login/health/switcher (agents+shells)/create surface done, screen read/reply/keys still predate the spec) |
| supervisor | `src/supervisor.rs` | `Supervisor`, `RestartAction`, `SpawnHerdr` | — |
| watcher | `src/watcher.rs` | `PollWatcher`, `StatusCursor`, `StatusChange` | — |
| store | `src/store/{mod,sqlite,memory}.rs` | `Store`, `SqliteStore`, `MemoryStore`, `Notification` | — |
| notify | `src/notify/{mod,telegram}.rs` | `Notifier`, `NotifyService`, `TelegramNotifier` | — |
| composition root | `src/main.rs` | arg parse, wiring, loops | — |
| service lifecycle | `src/main.rs` (`service` verb), `src/doctor/checks.rs` (`run_service_command` + platform helpers) | `run_service_command` | `docs/specs/service-lifecycle.md` (partial; Windows branch unproven on real hardware) |
| frontend | `web/src/{main,api}.ts`, `web/src/views/*.ts`, `web/src/styles.css` | `AgentRow`, `ShellRow`, `HealthInfo`, `TerminalProps`, `NewPaneRef` (post-create AND shell-entry navigation reference), `Route`/`pathForRoute`/`parseRoutePath`/`resolveBootstrapDecision` (URL routing for all three screens), `buildHomeGroups` (combines agent + shell rows), view renderers (poll-based, no WebSocket client) | switcher: `docs/specs/switcher.md` (partial; now covers shell entries on home + its own URL); terminal detail: `docs/specs/terminal-detail.md` (partial); create sheet: `docs/specs/create-sheet.md` (partial; only missing screenshots); login: `docs/specs/login.md` (partial) |
| tests (e2e) | `tests/observe_reply_e2e.rs` | real-server observe/reply proofs | — |
| installation | `install.sh`, `dev-deploy.sh`, `.github/workflows/{release,ci}.yml`, `scripts/windows-runtime-smoke.ps1`, `scripts/windows-install-smoke.ps1`, `scripts/macos-install-smoke.sh`, `packaging/herdr-go.service`, `build.rs` | native roots and token lifecycle (`src/config/mod.rs`), `Assets` (`src/web/mod.rs`) | `docs/specs/installation.md` (partial) |

## Not yet specced (known future areas)

| Area | Planned home |
|---|---|
| provision (GitHub repo → checkout → workspace → agent, via Telegram wizard) | later slice |
| Tier 1 verbs (say/read/stop with readiness guard) | later slice |
| events.subscribe upgrade (poll → subscribe) | later slice (PBI-003 gates it) |

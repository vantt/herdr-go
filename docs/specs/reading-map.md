# Reading Map

Where each area of this project lives. bee-scribing owns this file: it is updated whenever an area spec is created or moved. Read this before any broad search — it answers "where does X live" without a grep.

| Area | Code entry points | Key types | Spec |
|---|---|---|---|
| security | `src/security/{paths,slug,redact}.rs` | `Boundary`, `sanitize_slug`, `redact` | — |
| config | `src/config/mod.rs` | `Config`, `Secrets`, `ConfigError` | — |
| herdr port | `src/herdr/{mod,wire,fake,socket}.rs` | `Herdr` trait, `SocketHerdr` (herdr.sock client), `Snapshot`(flat), `AgentStatus`, `ScreenRead`, `FakeHerdr` | — |
| web + observe | `src/web/{mod,auth,api,screen}.rs` | `AppState`, `AuthSession`, `read_screen`/`send_reply` (poll screen + reply) | — (web interface serving/embedding covered by `installation.md`) |
| supervisor | `src/supervisor.rs` | `Supervisor`, `RestartAction`, `SpawnHerdr` | — |
| watcher | `src/watcher.rs` | `PollWatcher`, `StatusCursor`, `StatusChange` | — |
| store | `src/store/{mod,sqlite,memory}.rs` | `Store`, `SqliteStore`, `MemoryStore`, `Notification` | — |
| notify | `src/notify/{mod,telegram}.rs` | `Notifier`, `NotifyService`, `TelegramNotifier` | — |
| composition root | `src/main.rs` | arg parse, wiring, loops | — |
| frontend | `web/src/{main,api,ws}.ts`, `web/src/views/*.ts`, `web/src/styles.css` | `TerminalSocket`, view renderers | switcher screen: `docs/specs/switcher.md` (partial); login/terminal-detail screens not yet specced |
| tests (e2e) | `tests/tier2_relay_e2e.rs`, `tests/gateway_behavior_e2e.rs` | real-server WS proofs | — |
| installation | `install.sh`, `dev-deploy.sh`, `.github/workflows/release.yml`, `packaging/herdr-go.service`, `build.rs` | `data_dir()`/`config_dir()` (`src/config/mod.rs`), `Assets` (`src/web/mod.rs`) | `docs/specs/installation.md` (partial) |

## Not yet specced (known future areas)

| Area | Planned home |
|---|---|
| provision (GitHub repo → checkout → workspace → agent, via Telegram wizard) | later slice |
| Tier 1 verbs (say/read/stop with readiness guard) | later slice |
| events.subscribe upgrade (poll → subscribe) | later slice (PBI-003 gates it) |

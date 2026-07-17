# Reading Map

Where each area of this project lives. bee-scribing owns this file: it is updated whenever an area spec is created or moved. Read this before any broad search — it answers "where does X live" without a grep.

| Area | Code entry points | Key types |
|---|---|---|
| security | `src/security/{paths,slug,redact}.rs` | `Boundary`, `sanitize_slug`, `redact` |
| config | `src/config/mod.rs` | `Config`, `Secrets`, `ConfigError` |
| herdr port | `src/herdr/{mod,wire,fake,cli}.rs` | `HerdrControl`, `HerdrStream`, `TerminalFrame`, `Snapshot`, `FakeHerdr`, `CliHerdr` |
| web + relay | `src/web/{mod,auth,api,relay}.rs` | `AppState`, `AuthSession`, `ws_terminal` |
| supervisor | `src/supervisor.rs` | `Supervisor`, `RestartAction`, `SpawnHerdr` |
| watcher | `src/watcher.rs` | `PollWatcher`, `StatusCursor`, `StatusChange` |
| store | `src/store/{mod,sqlite,memory}.rs` | `Store`, `SqliteStore`, `MemoryStore`, `Notification` |
| notify | `src/notify/{mod,telegram}.rs` | `Notifier`, `NotifyService`, `TelegramNotifier` |
| composition root | `src/main.rs` | arg parse, wiring, loops |
| frontend | `web/src/{main,api,ws}.ts`, `web/src/views/*.ts`, `web/src/styles.css` | `TerminalSocket`, view renderers |
| tests (e2e) | `tests/tier2_relay_e2e.rs`, `tests/gateway_behavior_e2e.rs` | real-server WS proofs |

## Not yet specced (known future areas)

| Area | Planned home |
|---|---|
| provision (GitHub repo → checkout → workspace → agent, via Telegram wizard) | later slice |
| Tier 1 verbs (say/read/stop with readiness guard) | later slice |
| events.subscribe upgrade (poll → subscribe) | later slice (PBI-003 gates it) |

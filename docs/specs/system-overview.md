# System Overview — herdr-go

Technology-agnostic description of what the system does now. First read for any human or agent new to the repository. bee-scribing owns this file.

## Purpose

A remote gateway + supervisor for [herdr](https://github.com/ogulcancelik/herdr): watch and type into AI coding agents running in herdr terminals, from a phone, over the web; and keep herdr alive automatically. The gateway is the single security boundary in front of herdr's unauthenticated socket. herdr remains the source of truth for all session/agent lifecycle — the gateway is (near) stateless about sessions.

## Shape

`systemd → herdctl (supervisor + web) → herdr → coding agents`. One self-contained binary, `herdctl` (Rust) — the web UI (TypeScript + xterm.js) is built into that binary, with an on-disk copy able to override it for local iteration (see `docs/specs/installation.md`).

## Areas (M1)

| Area | What it does | Code |
|---|---|---|
| security | Pure validators: path-allowlist (7-step ordered), byte-level slug, single redactor. Not a sandbox — governs which paths the gateway hands to herdr. | `src/security/` |
| config | Strict decoding (unknown key = error), fail-closed empty allowlist, secrets from env only. | `src/config/` |
| herdr | The gateway is a **client of the herdr server**: `Herdr` trait over `herdr.sock`'s JSON API (`session.snapshot`, `ping`, `pane.read`, `pane.send_input`). `SocketHerdr` for real, `FakeHerdr` for tests/`--demo`. | `src/herdr/` |
| web | axum: token+cookie auth (fail-closed, silent-404), switcher API, and the **observe/reply** surface (poll a pane's screen, post a reply). No live PTY — herdr's API can't size a terminal to a phone (DISCOVERY 2026-07-18). | `src/web/` |
| supervisor | Health-checks herdr, relaunches it when down. Never force-kills; agents outlive the gateway. | `src/supervisor.rs` |
| watcher | Polls status (500ms), emits de-duplicated status changes. | `src/watcher.rs` |
| store | SQLite (WAL, all-or-nothing migrations): poll offset + at-least-once notification outbox. Never stores terminal output or credentials. | `src/store/` |
| notify | Outbound alerts (Telegram) when an agent is blocked/done, at-least-once, redacted. | `src/notify/` |
| frontend | Mobile-first web UI: login → agent switcher (status badges) → landscape live terminal. | `web/src/` |
| installation | Install/upgrade flow (published-copy download with source-build fallback), background-service setup, and how the binary chooses between its built-in web UI and an on-disk override. | `install.sh`, `dev-deploy.sh`, `build.rs` |

## Design invariants

- **Hexagonal only at real seams** (ports with ≥2 real impls): herdr, event source, store, notifier. Everything else is concrete. `main.rs` is the sole composition root.
- **The Tier 2 relay is a transparent pipe**: web ↔ `HerdrStream` directly, never through the control plane, never redacted (the human sees the real screen).
- **Fail-closed everywhere it touches trust**: auth, path validation, config, empty allowlist.
- **herdr is the source of truth**: opaque ids are read fresh from snapshots, never constructed or cached.

## Where reality diverges from the spec

herdr behaviors verified live in the M0 spikes (see `../DISCOVERY.md`) win over any looser reading: one-request-per-connection socket, exact protocol pinning, subscribe replay requiring de-dup, `seq` ordering-only (no backfill), EOF ≠ `terminal.closed`, `--session` always explicit.

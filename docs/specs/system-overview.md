# System Overview — herdr-go

Technology-agnostic description of what the system does now. First read for any human or agent new to the repository. bee-scribing owns this file.

## Purpose

A remote gateway + supervisor for [herdr](https://github.com/ogulcancelik/herdr): watch and type into AI coding agents running in herdr terminals, from a phone, over the web; and keep herdr alive automatically. The gateway is the single security boundary in front of herdr's unauthenticated socket. herdr remains the source of truth for all session/agent lifecycle — the gateway is (near) stateless about sessions.

## Shape

`systemd → herdr-go (supervisor + web) → herdr → coding agents`. One self-contained binary, `herdr-go` (Rust crate `herdr_go`) — the web UI (TypeScript + xterm.js) is built into that binary, with an on-disk copy able to override it for local iteration (see `docs/specs/installation.md`).

## Areas (M1)

| Area | What it does | Code |
|---|---|---|
| security | Pure validators: path-allowlist (7-step ordered), byte-level slug, single redactor. Not a sandbox — governs which paths the gateway hands to herdr. | `src/security/` |
| config | Strict decoding (unknown key = error), fail-closed empty allowlist, native per-user roots, and an owner-only login-token lifecycle that is validated before serving. | `src/config/` |
| herdr | The gateway is a client of the herdr server over the platform's local per-user endpoint, using the server's request and subscription protocol. The real server and the test/demo substitute share one application-facing contract. Spec: `herdr-port.md`. | `src/herdr/` |
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
- **One native local-endpoint contract**: explicit override, named session, and
  default session resolve through the same selection path for startup,
  diagnostics, requests, subscriptions, and supervisor recovery. Linux retains
  its Unix local endpoint; the Windows branch roots default and named endpoints
  in the native user profile, independent of Unix-compatible home variables,
  and selects the corresponding native local endpoint without a gateway-owned
  relay.
- **Protected token before listener**: a token is created atomically with
  owner-only protection, an existing token is validated on every start, and
  any protection failure stops startup before the network listener opens.
- **Platform-native user state**: Windows-style selection uses roaming data for
  configuration, local data for persistent/runtime state, and an absolute native
  user profile for the default workspace root. Linux keeps its established
  per-user locations and migration behavior.
- **herdr is the source of truth**: opaque ids are read fresh from snapshots, never constructed or cached.
- **Herdr Go is the current product identity**: new configuration, data,
  background services, release archives, and documentation use `herdr-go`.
  `herdr-gateway` is recognized only to migrate an existing installation or
  preserve historical evidence; it is never a second active identity.
- **Safe promoted defaults**: demo mode listens on loopback unless the operator
  explicitly overrides the bind address. The systemd service keeps system
  hardening while allowing supervised work in ordinary user projects, and the
  installer performs its user-manager preflight before changing machine state.
- **Mode-isolated migration and normal-path docs**: default legacy state is
  moved only by normal default-config startup; doctor, demo, and explicit-config
  modes leave it untouched. Development deployment preflights before mutation,
  and public operator docs present supported install paths as working instead
  of treating possible release defects as the default state.
- **Proof precedes Windows publication**: the Windows compatibility check uses
  an immutable, checksum-verified upstream executable and binds every runtime
  invocation, including supervisor recovery, to that exact file. A Windows
  release decision is based on this real-Windows proof rather than on
  host-side contract tests alone; Linux and macOS release behavior is unchanged.

## Where reality diverges from the spec

herdr behaviors verified live in the M0 spikes (see `../DISCOVERY.md`) win over any looser reading: one-request-per-connection socket, exact protocol pinning, subscribe replay requiring de-dup, `seq` ordering-only (no backfill), EOF ≠ `terminal.closed`, `--session` always explicit.

## Open gaps

- Windows Server 2022 has a completed production proof in continuous
  compatibility checks. A separate Windows 11 support claim remains unproven
  until the same operator-facing lifecycle is exercised on Windows 11.

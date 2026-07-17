# CONTEXT — m1-gateway-web-terminal

Source of truth for locked decisions on the M1 build. Decisions are cited by id; never reinterpreted here.

## Goal

Runnable, beautiful, tested herdr-gateway MVP: the **web terminal axis** (mobile-first switcher → landscape live terminal) plus the supervisor/herdr-client foundation, an installer, and full usage docs. Built so it runs and is fully testable **without a live herdr** (Fake adapter), then points at real herdr by config.

## Locked decisions (cite, do not reinterpret)

- **Stack** (`afbc6161`): tokio + axum. Telegram Bot API via reqwest directly (no teloxide). Frontend TypeScript + xterm.js (PRD hard split).
- **Architecture** (`4e3ef1a1`): clean, hexagonal ports **only at real seams** with ≥2 real implementations: `HerdrPort`, `EventSource`, `Notifier`, `Store`. Security validators stay pure functions (no port). Tier 2 relay is a transparent pipe web↔HerdrStream, **bypasses core by design**. `main.rs` is the sole composition root.
- **HerdrPort split** (`da82b90f`): two traits — `HerdrControl` (snapshot, workspace.create, agent start, ping/version, Tier 1 verbs) and `HerdrStream` (observe/control frame+input). Tier 2 relay depends on `HerdrStream` only. Both implemented by the same CLI adapter today, socket later.
- **Scope B / Telegram** (`302c0544`): Telegram = notify + Tier 1 + provision wizard; web = auth + switcher + Tier 2 terminal. (Telegram is a later slice; M1 targets the web axis + foundation. Notify design stays poll-based.)
- **Open questions closed** (`bc4a65a4`): provision supports create-new + clone-URL, private default; web auth = static token + session cookie, single operator; web bound to **Tailscale tailnet** (not public Internet), auth stays fail-closed as defense-in-depth.
- **Rotate resize** (`82eff9f7`): hybrid — observe connection → observer resize (viewport only); control connection → controller resize (real PTY reflow). Both get a `full=true` frame next.

## Verified herdr constraints (from DISCOVERY.md — discovery wins spec)

- **One request per connection** (PBI-001): the socket answers only the first request per connection. A live subscription holds a dedicated connection; every other call is its own short-lived connection (like `herdr api` CLI). The herdr client MUST be built on this model.
- **Subscribe replay + mandatory de-dup**: each connect replays a recent-event ring buffer; client must be idempotent (cursor de-dup) and reconcile with `session.snapshot`. Reconnect discipline: snapshot → subscribe → catch-up-dedup → re-snapshot. Error responses carry no `id` — correlate FIFO. Per-pane `pane.*` events need `pane_id`.
- **Protocol pin exact-match** (not `>=`), number bumps per herdr release; pin per vendored version, checked at startup handshake, typed error on mismatch. Current `16`.
- **`seq` ordering-only** (PBI-002): herdr never emits seq gaps; slow clients are coalesced. Relay applies in seq order, needs no backfill; reconnect = reset xterm.js on the `full=true` frame.
- **EOF ≠ terminal.closed**: abrupt IPC EOF emits no `terminal.closed`; relay treats raw EOF identically (end WS, reconnect + full frame).
- **`--session` explicit always**: `HERDR_SESSION` env is ignored by herdr; every invocation prepends `--session`.

## Airemote patterns to port (evidence-backed, see porting-log.md)

path-allowlist 7-step ordered · slug byte-level allowlist · redactor single idempotent · strict config decoding · empty-allowlist fail-closed · token env-only · session-isolation `--session` · protocol pin · poll + de-dup EventSource · single-service flock · auth-gate fail-closed silent · never-store-output/credentials · adversarial+mutation testing.

## Non-goals (M1)

Session lifecycle records (herdr is source of truth, gateway stateless per session) · Telegram bot (later slice) · provision (later slice) · Tier 1 verbs (later slice) · web push/PWA · sandboxing agents.

## Build posture

FULL AUTOPILOT (`7b04f2c7`) — overnight autonomous, Gates 1-3 auto-approved incl. high-risk; secret-file reads and review P1 still stop (neither needed for this build). Every slice caps against a real passing `cargo`/`npm` verify with recorded output.

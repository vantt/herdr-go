# DISCOVERY — verified deltas (discovery wins spec)

Live/source-verified findings that refine or correct the PRD. Each entry cites evidence. When this file and the PRD disagree, **this file wins**.

---

## 2026-07-17 — `events.subscribe` reliability (PBI-001, PRD §10 [CAO] #1)

**Status:** live-verified against a running herdr **0.7.4 / protocol 16**. Evidence: `.bee/spikes/pbi-001-events-subscribe/` (`verdict.md`, `v3-findings.json`, `replay-check.mjs`, `schema.json`). Verdict: **`events.subscribe` is reliable — Event watcher may upgrade poll 500ms → subscribe.** Notify stays on poll for M0 ship (Telegram-B, decision 302c0544) — the upgrade is a later optimization, not a ship blocker.

**Deltas / sharpenings (these win over any looser PRD reading):**

1. **Subscribe works; the "not implemented" note was airemote's, not herdr's.** herdr acks `{"type":"subscription_started"}` then streams. Do not carry airemote's `Subscribe = ErrNotImplemented` forward as a herdr fact — it was airemote's own wrapper stance. Resolves §10 #1: subscribe is trustworthy.

2. **The socket API is ONE-REQUEST-PER-CONNECTION** (new constraint, not in the PRD). Only the first request on a connection is answered; later requests on the same socket are silently ignored (reproduced with snapshot→subscribe and split→close). → A live subscription **holds a dedicated connection** whose sole request is `events.subscribe`; every other call (snapshot, `pane.*`, `workspace.*`, `agent.*`) is its own short-lived connection, exactly as the `herdr api` CLI does. The gateway's herdr client must be built on this model.

3. **Subscribe replays a bounded recent-event ring buffer on every connect**, then streams live. Two zero-activity fresh subscribes each returned **exactly 93 events**, identical, including `pane_closed`/`workspace_closed` for defunct entities (an event-log replay, not a current-state dump). Consequence: **no silent drop across reconnect** (missed events re-delivered), but **already-seen events are re-delivered too** → the client MUST be idempotent: de-dup against a last-processed cursor and reconcile with `session.snapshot`. This subsumes PRD §8 "duplicate events → de-dup" — de-dup is non-negotiable regardless of the 2×/1ms `agent_status_changed` quirk. Mandatory reconnect discipline: `session.snapshot` → dedicated `events.subscribe` → catch-up-then-dedup → re-snapshot on every reconnect.

4. **Two event envelopes; per-pane events need `pane_id`.** Resource events (`EventEnvelope`, **underscore** kinds: `workspace_*`, `tab_*`, `pane_created/closed/updated/focused/moved/exited/agent_detected/agent_status_changed`, `layout_updated`) subscribe by `type` alone. The 3 filtered events (`SubscriptionEventEnvelope`, **dot** kinds: `pane.output_matched`, `pane.agent_status_changed`, `pane.scroll_changed`) **require `pane_id`** — watching agent status via subscribe means enumerating panes from the snapshot and subscribing per-pane, re-subscribing as panes/agents appear. Wire quirk: **error responses carry no `id`** — correlate FIFO, not by id.

**Residual (verified-live TODO):** drive a throwaway agent idle→working→blocked→done in an isolated workspace; confirm `pane.agent_status_changed` delivery latency + the 2×/1ms duplicate before committing agent-status notify to subscribe. Deferred here to avoid disturbing the operator's live agent sessions.

---

## 2026-07-17 — Tier 2 `observe`/`control` relay (PBI-002, PRD §10 [CAO] #2)

**Status:** protocol layer verified against `upstreams/herdr` source (`PROTOCOL_VERSION = 16`). Evidence: `.bee/spikes/tier2-observe-control/findings.md`. Verdict: **relay architecture feasible — GO**; §10 #2 de-risked at protocol layer, only live performance/UX numbers remain.

**Confirmed as PRD §5.1 states (no change):** frame schema (7 fields exact), newline-delimited JSON, `full=true` first frame + on resize, single-writer with `--takeover` eviction, protocol number pinning.

**Deltas / sharpenings (these win over any looser PRD reading):**

1. **`seq` is ordering-only, not loss detection.** herdr never emits `seq` gaps — a slow client is coalesced to latest state, never skipped (`render_stream.rs:105-114`, `46-59`). The relay applies frames in `seq` order and needs **no gap-recovery/backfill**. "Reconnect resync theo seq" (§10 #2) = server sends `full=true` frame #1, relay resets xterm.js and applies it; there is no per-seq backfill.

2. **Rotate hinges on observer-vs-controller resize** (`headless.rs:2836-2875`). Controller resize mutates the **real PTY** system-wide; observer resize reshapes only that client's own viewport. Both force a `full=true` frame. → **Open product decision:** which mode does the gateway use on phone rotate? Mobile-first WYSIWYG argues controller; not disturbing desktop clients argues observer. Logged as a decision; resolve before building §11 step 4.

3. **Protocol compat is exact-match, not `>=`** (`wire.rs:906-932`), and the number bumps per herdr release (not stable). Gateway pins it **per vendored herdr version**, checks at startup handshake, emits a typed error on mismatch. Sharpens PRD §8 "pin protocol number".

4. **Abrupt IPC EOF emits no `terminal.closed`** (`client/mod.rs:981`). The relay must treat a raw socket EOF identically to a graceful `terminal.closed` (end the WS stream, reconnect + full frame). Do not assume every stream end carries a `terminal.closed` marker.

5. **Latency floor is known, real latency is not.** Server render cadence is throttled to ~60fps (16ms tick, `app/mod.rs:33`); backpressure is bounded non-blocking with implicit coalescing (no cross-client stall, no seq holes). Floor = 16ms tick + base64-ANSI diff size. Real cellular/tailnet RTT is **deferred to a live test** once the relay (§11 step 4) exists.

---

## 2026-07-18 — herdr socket is the client contract; M1 CLI/wire names were wrong (live probe of herdr 0.7.4)

**Status:** live-probed against the running herdr 0.7.4 server (session `default`, protocol 16). Evidence: `herdr api schema --json` (85-command typed API, saved snapshot), `herdr api snapshot`, `herdr pane read`, `herdr session list`. This **supersedes the M1 herdr-facing design** (`src/herdr/*`, built and tested only against a Fake whose shapes were invented from the PRD's wire prose). The pure layers (security, config, web auth, store, notify *logic*) are unaffected — they never touched herdr's real surface.

**Architecture (corrects the M1 framing):**

- **The gateway is another client of the herdr *server*, a peer of the TUI** — not an observer of any "client". herdr's server owns the runtime and exposes a Unix socket (`~/.config/herdr/herdr.sock`); the TUI, the `herdr` CLI binary (each command is a one-shot socket client), and the gateway are all just clients of it. `session.snapshot` returns the *server's* runtime state. `status server` vs `status client` is server-health vs TUI-local-health and is irrelevant to us.
- Therefore the correct herdr adapter is a **socket client** (JSON-over-unix-socket to `herdr.sock`), not a `herdr`-binary subprocess wrapper. The CLI binary is a leaky one-shot proxy and — critically — exposes **no streaming**, so the "CLI adapter today, socket later" staging can't reach live Tier 2 at all. Swap the implementation behind the existing `HerdrControl`/`HerdrStream` traits (the hexagonal seam pays off; consumers unchanged). Keep the `herdr` binary only for the supervisor's `herdr server` launch.

**Real socket API (from `api schema`, protocol 16) vs. the M1 design's invented names:**

| Need | Real socket command | M1 design used (WRONG) |
|---|---|---|
| Snapshot | `session.snapshot` | `herdr session snapshot` CLI / nested `Snapshot` type |
| Read screen | `pane.read` (`--source visible/recent`, `--format text/ansi`) | — |
| Send input | `pane.send_input` (raw), `pane.send_text`, `pane.send_keys` | `terminal.input` |
| Resize | `pane.resize` | `terminal.resize` |
| Events | `events.subscribe`, `events.wait` | (matched PBI-001) |
| Health | `ping` | `status server` |
| Provision | `workspace.create`, `agent.start` | (ok) |
| Deep-link stamp | `pane.report_metadata` | (ok, PRD §6) |

The M1 names `terminal.observe` / `terminal.control` / `terminal.frame` **do not exist** in the 85-command API.

**Snapshot shape (real):** `session.snapshot` (== CLI `api snapshot`) returns a **flat** `result.snapshot.agents[]`, each: `pane_id` (opaque, e.g. `w3:p6`), `workspace_id`, `tab_id`, `agent` (kind), `agent_status`, `terminal_title`/`terminal_title_stripped`, `cwd`, `focused`. **NOT** the nested workspace→tab→pane→agent tree the M1 `Snapshot` type models. Adopt the flat shape.

**Agent status set (real):** `idle | working | blocked | done | unknown` (the M1 enum is missing `unknown`; `done` exists via `pane.wait_for_output`/`wait agent-status` but the live snapshot reported a finished-looking agent as `idle` — done-vs-idle semantics need care).

**Two genuinely-unverified points (need a source spike before Tier 2 live):**

1. **Live-render mechanism is unknown.** There is **no** request command that streams terminal frames. The socket offers `pane.read` (a screen *snapshot*, poll-only) and `events.subscribe` (event deltas). The continuous render the TUI uses to draw live is not a named request — it is a separate attach/stream framing (or an output-event subscription) that must be read from herdr source to pin. **PBI-002's claim that observe/control/`terminal.frame` were "verified from source" is not reflected in the socket API and is now suspect.** Until spiked, live Tier 2 is unproven; `pane.read` polling is the working stand-in.
2. **No socket-level single-writer / takeover lock.** The 85 commands contain no `takeover`/`writer`/`claim`/`attach` concept (only `pane.clear_agent_authority`, about agent authority, not a terminal write lock). The PRD's "one writer per terminal, `--takeover` to evict" likely describes the **TUI interactive attach**, not socket `pane.send_input`. If any client may send input freely, the relay's takeover/single-writer model is unnecessary at this layer. **Confirm in the spike.**

**Consequence for M1:** the herdr adapter (`cli.rs` → new socket client), the wire types (`wire.rs`: flat snapshot, status set, real command names, drop the invented frame/observe types until the live-render mechanism is known), the relay (`web/relay.rs`: input via `pane.send_input`, revisit takeover), and the Fake (`fake.rs`: re-shape to mirror real herdr so tests stay honest) all need a **reality-alignment pass**. Everything else in M1 stands.

### Shipped realignment (feature herdr-socket-observe, 2026-07-18)

Live-verified and implemented:
- **Socket framing confirmed live**: newline-delimited JSON, one request→response per connection. `{"id","method","params"}\n` → `{"id","result":{…}}\n`; errors are `{"error":{"message":…}}` (no `id`). Built as `src/herdr/socket.rs` (`SocketHerdr`).
- **`pane.resize` is relative** (direction+amount, split-ratio) — **there is no absolute cols×rows sizing in the request API**, and no attach/stream command. So the request API **cannot size the PTY**; a polled `pane.read` returns the desktop's current width (observed 214–292 cols).
- **Product decision** (`675fc93a`): rather than chase the attach/stream (unverified) to get phone-width, the phone **observes a zoom/pan screen** (poll `pane.read`, render ANSI, scroll to pan, A−/A+ to zoom) and **replies via a textarea** → `pane.send_input {text}` + Enter. No PTY sizing, no live stream, no spike. The M1 live-WS Tier 2 terminal is removed.
- **Live-verified against real herdr 0.7.4**: `/api/health` (proto 16), `/api/agents` (7 real agents, correct statuses/titles), `/api/panes/:pane/screen` (real 67-line ANSI screen). `pane.read` `revision` sometimes returns 0 → the client detects change by comparing text, not revision. `send_input` to a live agent was **not** probed (would disturb real work); it is covered by the Fake e2e + schema, to be confirmed by the operator on a real reply.
- **Live-render stream + any single-writer/takeover** remain unverified and out of scope (a future spike) — the observe/reply model does not need them.

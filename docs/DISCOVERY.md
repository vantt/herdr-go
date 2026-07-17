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

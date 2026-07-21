---
purpose: standalone analysis, not a locked decision — advisory only, referenced by PBI-031
date: 2026-07-21
status: informational (no decision recorded; current poll model is unchanged)
---

# Poll (`pane.read`) vs WebSocket for the terminal-detail screen

Triggered by a question about why the project once had a WebSocket relay and now polls. No code or architecture changed as a result of this analysis — it is background for whoever picks up PBI-031 (or reopens the live-terminal question later).

## Framing correction

"`pane.read` vs WebSocket" is not one choice at one boundary — there are two:

```
herdr  ──(A: gateway ↔ herdr)──  gateway  ──(B: gateway ↔ phone browser)──  browser
```

- **Boundary A (gateway↔herdr):** hard-constrained, not a design choice. `docs/DISCOVERY.md` (realignment section): *"There is no request command that streams terminal frames."* herdr only exposes `pane.read` (poll snapshot) and `events.subscribe` (event deltas — verified reliable for `pane.agent_status_changed`, not proven to cover raw output content). Whatever boundary B does, the gateway still has to poll `pane.read` upstream — content cannot arrive faster than that poll cadence.
- **Boundary B (gateway↔browser):** the only boundary where poll vs push is actually a live choice. Today: client-driven HTTP poll every 1500ms (`web/src/views/terminal.ts`, `POLL_MS`). Previously: a WebSocket push (`src/web/relay.rs` + `web/src/ws.ts`, removed).

Consequence: reintroducing WebSocket at boundary B does not make terminal content fresher than boundary A's poll cadence allows — it can only shave the last-hop latency and change the push/pull ergonomics.

## Trade-off table (boundary B only)

| Axis | Poll (current) | WebSocket (removed) |
|---|---|---|
| Perceived latency | up to ~1.5s + round trip | faster last hop, but still capped by boundary A's poll cadence |
| Mobile battery/bandwidth | short request/response, can stop entirely when backgrounded | long-lived connection keeps the radio from sleeping — works against the product's mobile-first goal |
| Complexity/reliability | self-healing — a failed cycle just retries next tick | inherits every reconnect gotcha already paid for once in PRD §8: 1-request/1-connection, replay+de-dup, `seq` no-backfill, EOF ≠ `terminal.closed`, the `snapshot→subscribe→catch-up-dedup→re-snapshot` reconnect discipline |
| Server cost under many concurrent viewers | stateless between requests | one live connection + cleanup state per viewer |
| Security/exposure | each request re-authenticates, nothing lingers | long-lived session needs its own thinking about mid-session token expiry, idle timeout, socket-exhaustion |
| Fit with actual usage | matches "glance occasionally, reply, leave" | built for a continuously-watched session, which this product doesn't have |

## Historical evidence

`docs/DISCOVERY.md`, decision `675fc93a`: the WebSocket relay was removed specifically because herdr's request API cannot size a PTY to phone width (`pane.resize` is relative-only, no absolute cols×rows) — so "live" wouldn't even render at the right width. The team paid the reconnect/de-dup complexity cost once already, for a payoff (true live width-correct rendering) that turned out to be unreachable. Rebuilding WebSocket now reproduces that same cost for the same unreached payoff, unless the goal has changed.

## Lighter middle path (not yet verified — this is PBI-031)

Instead of reintroducing a long-lived connection at boundary B, gate boundary-A polling on `events.subscribe`: only re-poll `pane.read` when an event says something changed, instead of polling on a blind interval. This keeps poll's self-healing property (no reconnect state machine at boundary B) while cutting wasted polls.

**Open question, unverified:** does herdr emit an event for "this pane's output changed", or does `events.subscribe` only cover status/metadata transitions (`pane.agent_status_changed` is the only one confirmed reliable, per PRD §8)? If no content-change event exists, this middle path collapses back to plain interval polling and isn't worth building. This needs a small spike before real planning — see PBI-031.

## Source Pack

- `docs/DISCOVERY.md` (realignment + shipped-realignment sections)
- `docs/PRD.md` §8 (verified herdr constraints table) and §10 (risk log, `675fc93a`)
- `web/src/views/terminal.ts` (current poll implementation, `POLL_MS = 1500`)
- `git log --all -- web/package.json`, `git show 8f522b0:web/package.json` (confirmed xterm.js since the first commit; WebSocket relay was a transport choice, not a renderer choice)
- `docs/distillery/sources/herdr.md:134` (`session.snapshot` is one-time bootstrap, kept current via `events.subscribe` — scope of what subscribe covers not fully pinned here)

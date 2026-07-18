---
artifact_contract: bee-plan/v1
artifact_readiness: implementation-ready
mode: high-risk
---

# Plan — herdr-socket-observe (snapshot-view + reply model)

## Mode gate

Flags: external systems (herdr socket), public contracts (herdr layer + web API), audit/security (boundary), existing covered behavior (replaces M1 live-WS terminal), multi-domain. → **high-risk**.

## Discovery / validating (live-probed herdr 0.7.4)

Verified against the running socket (`.bee/spikes/herdr-socket-observe/`): newline-delimited JSON, one request→response per connection; `ping` proto 16; `session.snapshot` → flat `agents[]` (7 real agents); `pane.read` → screen text (69 lines × 214 cols, key `result.read.text`); **`pane.resize` is relative (direction+amount) — no absolute grid sizing**; no attach/stream in the request API. → live PTY sizing is impossible via request-API, which is why the model pivots to snapshot-view + reply (decision `675fc93a`). Reality check now PASSES.

## Approach

Drop the live-PTY/WebSocket Tier 2. New model, fully inside the verified request-API:
- **Observe** = poll `pane.read` (~1-2s) → render the (wide) ANSI screen into a **zoom/pan** viewport (no fit-to-phone; the user pinch-zooms/pans). xterm.js is reused only as a static ANSI renderer.
- **Reply** = a button reveals a textarea → send `pane.send_input {pane_id, text}` then an Enter key (handles send≠submit). Readiness is the human's call (they see the screen).

Herdr surface simplifies to request/response only — **drop `HerdrStream`/`FrameStream`/`ControlSession`**; the trait becomes `snapshot`, `ping`, `read_pane`, `send_input`. Web drops the WS relay for two HTTP endpoints. `SocketHerdr` speaks `herdr.sock`; `CliHerdr` removed (supervisor keeps a thin `herdr server` spawn).

## Slices (cells)

- **hso-1 — wire + socket client + trait**: reshape `wire.rs` (flat `Snapshot{agents:[{pane_id,workspace_id,tab_id,kind,status,title}]}`, `AgentStatus`+`Unknown`, `ScreenRead{text,revision}`); new `socket` module (unix-socket, newline-JSON request/response, id correlation, typed error); simplify the herdr trait to `Herdr { snapshot, ping, read_pane, send_input }` (remove HerdrStream/ControlSession/FrameStream). Reshape `FakeHerdr` to match (flat snapshot, screen buffer for read, echo on send). Unit tests + recorded live probe.
- **hso-2 — SocketHerdr + web endpoints**: `SocketHerdr` impl of the trait over the socket (session.snapshot, ping, pane.read, pane.send_input text+Enter). Web: remove WS relay; add `GET /api/agents/:pane/screen` (auth → poll read) and `POST /api/agents/:pane/input` (auth → send reply); keep `/api/agents`, `/api/health`, auth. Update `AppState`. Tests (unauth 404, screen read via Fake, input via Fake).
- **hso-3 — frontend + wire main + LIVE verify**: frontend observe view = zoom/pan screen (poll `screen` endpoint, render ANSI via xterm as static renderer) + Reply button → textarea → POST input. Remove `ws.ts` live socket. `main.rs` uses `SocketHerdr` (non-demo), socket path config (default `~/.config/herdr/herdr.sock`), supervisor keeps binary spawn. **LIVE verify against real herdr**: switcher lists the 7 real agents; open one → see its screen, zoom/pan; reply a harmless line → confirm it lands in that agent's composer. Update README/usage/DISCOVERY.

## Test matrix

- Socket: request/response round-trip, error (no id) FIFO, one-conn-per-request, flat snapshot parse, unknown status.
- Adapter: snapshot maps 7 agents; read_pane returns text+revision; send_input encodes text+Enter.
- Fake: mirrors real shapes → prior consumer tests (web auth/api, watcher, notify) still pass.
- Frontend: ansi renders; zoom/pan works; reply posts text; screen polls.
- **Live**: real agents listed; real screen rendered; reply lands (verified by driving a throwaway/own pane, not asserted).

## Acceptance

`cargo test` + clippy + web bundle/test green; **and** `herdctl --config …` against live herdr shows the 7 real agents, an openable zoom/pan screen, and a working reply — verified by driving it.

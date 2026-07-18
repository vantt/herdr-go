# CONTEXT — herdr-socket-observe

Realign the herdr integration to herdr 0.7.4's real socket API and make the gateway observe + drive **real herdr** (poll-based). Supersedes the M1 Fake-derived herdr design. Source of truth for locked decisions; cite, do not reinterpret.

## What & why

M1's herdr adapter was built from PRD wire prose and never touched real herdr; its command/type names (`terminal.observe/control/frame`, nested snapshot) don't exist. Full finding: `docs/DISCOVERY.md` 2026-07-18. This feature makes the gateway a **socket client** of the herdr server and observes real agents.

## Locked decisions (cite)

- **Socket client, not CLI subprocess** (`91e06fce`): talk `~/.config/herdr/herdr.sock` JSON API (85 commands, protocol 16). Keep the `herdr` binary only for the supervisor's `herdr server` launch.
- **Fork C — poll first** (`64d0297b`): terminal via `pane.read` polling, not a live frame stream. Live-render stream + takeover are a later spike (A1/A2 in DISCOVERY).
- **B3 flat snapshot**: `session.snapshot` → flat `agents[]` (`pane_id` like `w3:p6`, `workspace_id`, `tab_id`, `agent`, `agent_status`, `terminal_title`). Drop the nested tree.
- **B4 status set**: `idle | working | blocked | done | unknown` (add Unknown).
- **B5 input**: `pane.send_input` (raw) for xterm passthrough.
- **B7 Fake reshape**: mirror real herdr (flat snapshot, `pane.read` screen buffer, `send_input` echo, `resize`) so tests stay honest.
- **Resize for phone** (`61588e75`): control-mode → `pane.resize` PTY to the landscape viewport fitted **minus the on-screen keyboard**, clamp ~60-120 cols / 15-40 rows, debounce ~250ms, prefer landscape. Per-client viewport is the ideal if a later spike confirms herdr supports it.

## Real socket API (verified live, to be re-probed in validating)

`session.snapshot`, `ping`, `pane.read` (`source`, `format`), `pane.resize`, `pane.send_input`/`send_text`/`send_keys`, `events.subscribe`/`wait`, `workspace.create`, `agent.start`. Envelopes: `request` (method+params, id), `success_response`, `error_response`, `event`, `subscription_event`. One-request-per-connection (PBI-001); errors carry no id → correlate FIFO.

## Architecture

Keep `HerdrControl`/`HerdrStream` traits unchanged — swap the impl to `SocketHerdr`. `observe`/`control` become poll loops **inside** the adapter (poll `pane.read`, hash, yield a frame on change; input → `pane.send_input`; resize → `pane.resize`). Relay + frontend stay largely as-is (the seam pays off). Reshape `wire.rs` (flat snapshot, status, envelopes) + `fake.rs`.

## Non-goals

Live socket frame-stream (spike later) · takeover/single-writer model (unverified — spike) · Tier 1 verbs · provision.

## Posture

FULL AUTOPILOT. Every cell caps against real `cargo`/live-herdr verify. Validating probes the **live** herdr socket (unlike M1).

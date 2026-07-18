# Terminal scrollback: why the mobile gateway can't scroll agent history

Date: 2026-07-18. Verified live against herdr 0.7.4 (socket probes + `herdr api
schema` + binary strings) and on a real phone.

## Symptom

On mobile the terminal shows only the current screen (~67-69 rows). You can
swipe through that whole screen but cannot scroll back into earlier output.

## What the gateway uses (and its ceiling)

The gateway is a client of herdr's **JSON control socket** `~/.config/herdr/herdr.sock`.

- `pane.read` params (`herdr api schema` → `PaneReadParams`): `pane_id`,
  `source`, `lines`, `format`, `strip_ansi`.
- `source` variants: `visible`, `recent`, `recent_unwrapped`, `detection` — all
  return the **current screen only**. `--lines 1000 --source recent` still
  returned ~69 lines.
- Every agent pane reports `scroll.max_offset_from_bottom = 0` (no scrollback in
  this API's model — the agents are full-screen/alt-screen TUIs).
- No scrollback/stream/attach method anywhere in the schema (only
  `events.subscribe` for status, `PaneScrollInfo`, `ServerLiveHandoffParams`).
- `pane.send_keys` accepts `pageup`/`wheelup`/`scrollup` etc., but sending them
  never changed `offset_from_bottom` — they are no-ops through this API.

So through the JSON API the gateway is at its ceiling: current screen only.

## How the native herd client DOES scroll (the real mechanism)

The native client does **not** use `herdr.sock`. It connects to a **second
socket** `~/.config/herdr/herdr-client.sock` speaking a **binary "SemanticFrame"
protocol** (client log: `handshake succeeded version=16 encoding=SemanticFrame`).

- Client sends `terminal.input` / `terminal.resize` / **`terminal.scroll`**.
- Server streams **SemanticFrame** frames — the structured terminal render
  **including scrollback**, held server-side (that is why detach → re-attach
  keeps the scroll position; the buffer lives on the server, not the client).

So herdr has two tiers: JSON API (`herdr.sock`, per-pane snapshot, no scrollback)
and the attach channel (`client.sock`, SemanticFrame, has scrollback).

## Why the gateway can't just reuse `client.sock`

1. **Exclusive attach** — binary string: `already has an attached client; retry
   with --takeover`. Attaching would kick the user's desktop client. No
   observe/read-only/mirror mode was found.
2. **Wrong granularity** — the attach renders the **whole herd session** (all
   workspaces/tabs/panes + chrome), not a single pane. The gateway is a per-pane
   mobile view; mirroring the entire desktop TUI defeats that.
3. **Undocumented binary protocol** — SemanticFrame has no available source;
   reverse-engineering the frame format is fragile across herd updates.

## Conclusion / recommended path

Scrollback data exists server-side but only through the wrong-shaped attach
channel. The clean fix is a **herd feature request**: expose per-pane scrollback
on the JSON API — e.g. a scroll offset/depth on `pane.read`, or a `pane.scroll` +
history read. The internals already exist (`terminal.scroll` + server-side
scrollback), so it should be small on herd's side. This is outside the gateway.

## What the gateway shipped for the mobile terminal (this is complete)

Dark-only UI; bottom control bar; Reply text sheet (Enter-to-submit off by
default); a Keys d-pad driving `pane.send_keys` for TUI option menus; one-tap
switch between Keys and Reply; and **touch scrolling of the current screen**
(fixed by `pointer-events: none` on `.xterm` so swipes reach `.term-viewport`
instead of being eaten by xterm's own touchmove handler). Scrollback beyond the
current screen is a herd limitation, not a gateway bug.

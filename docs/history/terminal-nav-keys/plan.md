---
artifact_contract: bee-plan/v1
artifact_readiness: implementation-ready
mode: standard
---

# terminal navigation: recent scrollback + key input

## Problem

1. The mobile terminal only shows the last screen — you cannot scroll up to
   read earlier output.
2. When an agent asks a question with selectable options (arrow-key menu), there
   is no way to answer: the reply UI only sends typed text + Enter.

## Live herdr facts (verified read-only against 0.7.4)

- `pane.read` `source` variants: `visible` (current use), `recent`,
  `recent_unwrapped`, `detection`. `recent` returns recent scrollback beyond the
  visible screen.
- herdr has `pane.send_keys { pane_id, keys: [...] }`. Confirmed-valid key names:
  up, down, left, right, enter, escape, tab, space, pageup, pagedown, home, end,
  ctrl+c, shift+tab, plus single characters.

## Approach

**Backend**
- `read_pane`: request `source: "recent"` so the screen carries recent history.
- Add `Herdr::send_keys(pane_id, &[String])`; socket impl calls `pane.send_keys`,
  fake impl echoes keys + bumps revision.
- New route `POST /api/panes/:pane/keys` with body `{ keys: [...] }`.

**Web**
- `api.ts`: `sendKeys(paneId, keys)`.
- `terminal.ts`: a "Keys" button in the bottom bar opens a persistent d-pad
  overlay (↑ ↓ ← → Enter Esc Tab, plus Close). Each press posts the key and
  re-polls. The existing Reply text sheet is unchanged — together they cover
  arrow menus, numbered menus (type the number), and "Other → free text".
- Verify the viewport scrolls vertically through the recent buffer.

## Mode gate

Risk flags: 2 (public contracts — new endpoint + trait method; existing covered
behavior — terminal). Story-sized (backend contract + web UI). → `standard`.

## Cells (current slice)

- `terminal-nav-keys-1` — backend: recent source + send_keys trait/socket/fake +
  keys endpoint + tests.
- `terminal-nav-keys-2` — web: sendKeys api + d-pad UI + scroll verify.

## Verify

- `cargo test && cargo clippy -D warnings`; `npm run bundle && npm run test --run`.
- Browser (emulated phone): open a pane, scroll up through history; open Keys,
  press arrows/Enter and see the demo screen react; Reply text still works.

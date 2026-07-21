---
artifact_contract: bee-plan/v1
artifact_readiness: implementation-ready
mode: tiny
---

# terminal detail: auto-linkify URLs (PBI-030)

## Problem

Pane content rendered in the terminal detail view (`web/src/views/terminal.ts`)
is plain xterm.js output. Plain xterm.js does not auto-linkify URLs, and the
browser does not linkify text painted onto xterm's canvas/DOM renderer either
— a URL in agent output (e.g. a PR link, a docs link) is inert text the user
must manually copy.

## Verified facts (read against this repo, not assumed)

- `web/package.json` already lists `@xterm/addon-web-links@^0.11.0` as a
  dependency, but `grep -rn "WebLinksAddon" web/src/` returns nothing — the
  addon is installed but never wired.
- `web/src/views/terminal.ts:122-131` constructs the `Terminal` with
  `disableStdin: true` (read-only display view) and calls `term.open(viewport)`.
  `WebLinksAddon` listens for mouse events independent of stdin, so it works
  unchanged under `disableStdin: true`.
- No other addon (`FitAddon` etc.) is currently loaded, so there is no existing
  `loadAddon` call site to fold into — this adds the first one.
- `term.dispose()` (on back-button, line ~256) disposes loaded addons
  automatically; no separate addon teardown is needed.

## Approach

**Web only** — no backend/server change, no public contract change.

- `web/src/views/terminal.ts`: import `WebLinksAddon` from
  `@xterm/addon-web-links` and `term.loadAddon(new WebLinksAddon())` right
  after `term.open(viewport)`.

## Mode gate

Risk flags: 0 (no public contract, no backend, purely additive client-side
wiring of an already-vetted, already-installed dependency). → `tiny`.

## Cells (current slice)

- `pbi-030-terminal-url-linkify-1` — wire `WebLinksAddon` into the terminal
  detail view.

## Verify

`cd web && npm run bundle && npm run test -- --run`; manual: open a pane whose
screen contains a URL, confirm it renders as a clickable/hoverable link.

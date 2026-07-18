---
artifact_contract: bee-plan/v1
artifact_readiness: implementation-ready
mode: tiny
---

# dark-only UI

## Problem

Light-mode devices get a washed-out pale UI (desktop) and an invisible terminal
(mobile). The web UI is a dark-first design: the xterm terminal colours are
hardcoded for a dark background. But `web/src/styles.css` carries a
`@media (prefers-color-scheme: light)` block that flips the app surfaces/text to
a pale palette. On a light-mode device the terminal viewport background becomes
light while xterm still paints its light-on-dark foreground → the terminal text
is light-on-light and effectively invisible.

## Root cause (verified)

Browser repro on an emulated phone: identical app, only `prefers-color-scheme`
differs. Light → pale chrome + blank terminal. Dark → full, readable UI with a
dark terminal. Single source: the light media block.

## Mode gate

Risk flags: 0 (no auth/authorization/data/audit/external/contract/cross-platform
change). Files: 2. → `tiny`.

## Approach

Make the app dark-only, matching the stated dark-first intent and the
hardcoded-dark terminal:

- Remove the `@media (prefers-color-scheme: light)` override in
  `web/src/styles.css`. `:root` already declares the dark palette and
  `color-scheme: dark`.
- Set the `index.html` `color-scheme` meta to `dark` only, so form controls,
  scrollbars, and UA surfaces render dark regardless of the OS preference.

## Verify

- `npm run bundle` (web) succeeds.
- Browser: emulate a light-mode device, log in (demo), open an agent — the
  terminal renders dark text on a dark background and the chrome stays dark.
- Repo verify: `cargo test && cargo clippy -D warnings && npm run bundle && npm run test --run`.

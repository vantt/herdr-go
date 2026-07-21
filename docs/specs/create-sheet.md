---
area: create-sheet
updated: 2026-07-21
sources: [web-create-sheet]
decisions: [S1, S2, S3, S5, D1, D2, D3, D6]
coverage: partial
---

# Spec: Create Sheet (new shell / new agent)

The Operator's only write action beyond replying into a pane that already
exists: pick a destination already open on the desktop, then create a plain
shell or start an agent there. Opened from the switcher screen's FAB; ends by
landing the Operator directly in the new terminal.

## Entry Points & Triggers

- Tapping the FAB on the switcher screen → the sheet opens over the switcher,
  which stays exactly as it was underneath.
- Tapping a destination row → that destination becomes the selected one.
- Tapping Shell, or an agent preset row → creates the corresponding thing in
  the selected destination.
- Tapping the close (✕) button → the sheet closes with nothing created.

## Data Dictionary

| # | Element | Meaning | Values | Required | Default |
|---|---|---|---|---|---|
| 1 | Destination — label | Display name for the destination | text | yes | — |
| 2 | Destination — path | The folder something created here would start in | path, or absent when it can't be resolved | no | absent — shown as "no folder yet" |
| 3 | Destination — caveat | Warns the folder shown may not be trustworthy | `"Folder not detected"` (no path at all) · `"Folder may be stale"` (a path exists but isn't the live one) · absent (path is the live folder) | no | absent |
| 3a | Destination — disambiguator | Distinguishes two destinations that would otherwise look identical | a short marker appended after the label, shown only on destinations that share both their label and their folder with another one currently listed | no | absent (shown on every destination in the ordinary, non-colliding case) |
| 4 | Action row — Shell | Always the first action; creates a plain shell | fixed label "Shell" | yes | — |
| 5 | Action row — preset | One row per operator-configured agent preset | the preset's label only — never the command it runs | no | none (no presets configured → Shell is the only action) |
| 6 | Sheet status | What the Operator sees while destinations are loading or when there are none | `"Loading…"` · `"No destinations available."` · `"Session expired. Log in again to continue."` · `"Could not load destinations."` | no | absent once destinations are showing |

## Behaviors & Operations

### Open the sheet

- **Triggers:** tapping the switcher's FAB.
- **Blocked when:** never structurally — the FAB itself is disabled first
  whenever herdr is unreachable (see switcher.md), so a disabled FAB cannot
  be tapped at all.
- **What changes:** the destination and preset list is fetched fresh — never
  reused from a previous opening, so a destination that appeared or vanished
  since the sheet was last opened is always current.
- **Side effects:** none to the switcher screen underneath; its own list,
  scroll position, and grouping are untouched.
- **Afterwards:** the Operator sees every destination that currently exists —
  including one with no agent running in it, which the switcher's own list
  never shows — plus every configured preset, in one screen with no second
  fetch needed.

### Select a destination

- **Triggers:** tapping any destination row.
- **What changes:** that row becomes the selected one; any previous selection
  is cleared.
- **Side effects:** none — no request is sent yet.
- **Afterwards:** the Operator sees exactly one destination marked selected.

### Create a shell

- **Triggers:** tapping Shell with a destination selected.
- **Blocked when:** a create request is already in flight — every action row
  is disabled for the duration, so a second tap cannot fire a second request.
- **What changes:** a plain shell is created in the selected destination's
  folder when it resolves; when it does not resolve, the shell is still
  created — herdr is left to work out where.
- **Side effects:** none to the sheet's own destination list.
- **On failure:** the backend's own error message appears inside the still-
  open sheet; nothing closes, and every action row re-enables so the Operator
  can retry or pick a different destination.
- **Afterwards:** the sheet closes and the Operator is taken directly into
  the new shell's terminal (see terminal-detail.md).

### Start an agent

- **Triggers:** tapping a preset row with a destination selected.
- **Blocked when:** a create request is already in flight (same guard as
  Create a shell). A destination whose folder cannot be resolved refuses this
  action specifically — starting an agent there would put it in an unrelated
  folder, so the sheet shows that refusal as an inline error instead.
- **What changes:** an agent is started under the tapped preset, in the
  selected destination.
- **Side effects:** none to the sheet's own destination list.
- **On failure:** same as Create a shell — inline error, sheet stays open and
  usable.
- **Afterwards:** same as Create a shell — the sheet closes and the Operator
  is taken directly into the new agent's terminal.

## Actors & Access

| Capability | Operator (authenticated) | Anyone without a session |
|---|---|---|
| Open the sheet | ✓ (FAB disabled only when herdr is unreachable) | — |
| See every destination, including agentless ones | ✓ | — |
| See a preset's configured command | — (label only) | — |
| Create a shell | ✓ | — |
| Start an agent by preset | ✓ | — |

## Business Rules

- **R1.** A destination whose folder cannot be resolved still appears and is
  still selectable — it is never hidden or disabled, only marked with a
  caveat (per S2).
- **R2.** Creating a shell in an unresolved destination still succeeds; the
  sheet never blocks it with a confirmation step (per S2). Starting an agent
  in the same situation is refused instead — see terminal-detail.md's linked
  spec and `herdr-port.md` R17 for why the two are not symmetric.
- **R3.** A create failure is always shown inside the still-open sheet with
  the backend's own message; the sheet is never dismissed on error (per S3).
- **R4.** Two destinations sharing both a label and a folder each get a short
  disambiguating marker appended to their label; a destination with no such
  collision shows its label exactly as it otherwise would, with no marker
  (per S1).
- **R5.** The command an agent preset runs is never sent to, or visible in,
  the sheet — only its label (per parent D4, `new-shell-new-agent`).

## Edge Cases Settled

- **No presets configured.** Shell is still offered; the action list is
  simply one row.
- **No destinations at all** (herdr up but nothing open). The sheet shows
  "No destinations available." rather than an empty list.
- **The session expired while the sheet was opening.** Shown as "Session
  expired. Log in again to continue." rather than a generic error.
- **Tapping Shell/a preset twice quickly.** Only one request is ever sent;
  every action row is disabled for the duration of the first.

## Open Gaps

- No current screenshot exists under `visuals/create-sheet/` for the open
  sheet, a caveat row, an inline error state, or two disambiguated rows.

## Visuals

No current snapshot — see Open Gaps.

## Pointers (implementation)

- `web/src/views/create-sheet.ts` — `renderCreateSheet`, `destinationCaveat`,
  `collisionSuffixes` (R4's disambiguator), the open/close/select/submit
  logic described above.
- `web/src/views/switcher.ts` — the FAB that opens this sheet (see
  switcher.md).
- `web/src/api.ts` — `fetchCreateOptions`, `createPane`, `createAgent`.
- `web/src/main.ts` — `NewPaneRef`, the minimal reference this sheet builds
  on success and hands to the Operator's next screen.
- `docs/specs/web-api.md` — the backend contract this sheet consumes.

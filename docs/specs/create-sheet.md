---
area: create-sheet
updated: 2026-07-23
sources: [web-create-sheet, default-agent-presets, pbi-053-create-sheet-overlay-ux]
decisions: [S1, S2, S3, S5, D1, D2, D3, D6, 898c9cd5-33fe-4a7f-b0e8-fb7ab7c69b25, pbi-053-D1, pbi-053-D2, pbi-053-D3, pbi-053-D4, pbi-053-D5, pbi-053-D6, pbi-053-D7, pbi-053-D8, pbi-053-D9]
coverage: partial
---

# Spec: Create Sheet (new shell / new agent)

The Operator's only write action beyond replying into a pane that already
exists: pick a destination already open on the desktop and a type (a plain
shell, or an agent preset), then confirm to create it there. Opened from the
switcher screen's FAB; ends by landing the Operator directly in the new
terminal.

## Entry Points & Triggers

- Tapping the FAB on the switcher screen → the sheet opens over the switcher,
  which stays exactly as it was underneath.
- Tapping the Destination field → its list of destinations opens; tapping the
  Type field → its list of Shell/presets opens. Only one list is ever open at
  a time — opening one closes the other if it was open.
- Tapping a destination or type in an open list → that value becomes the
  selected one and the list collapses back to its one-line summary; nothing
  is created yet.
- Tapping New → creates the selected type in the selected destination.
- Tapping the close (✕) button → the sheet closes with nothing created.

## Data Dictionary

| # | Element | Meaning | Values | Required | Default |
|---|---|---|---|---|---|
| 1 | Destination — label | Display name for the destination | text | yes | the first destination in the list is selected by default when the sheet opens |
| 2 | Destination — path | The folder something created here would start in | path, or absent when it can't be resolved | no | absent — shown as "no folder yet" |
| 3 | Destination — caveat | Warns the folder shown may not be trustworthy | `"Folder not detected"` (no path at all) · `"Folder may be stale"` (a path exists but isn't the live one) · absent (path is the live folder) | no | absent |
| 3a | Destination — disambiguator | Distinguishes two destinations that would otherwise look identical | a short marker appended after the label, shown only on destinations that share both their label and their folder with another one currently listed | no | absent (shown on every destination in the ordinary, non-colliding case) |
| 4 | Type — Shell | Always the first entry in the Type field's list; creates a plain shell | fixed label "Shell" | yes | selected by default when the sheet opens |
| 5 | Type — preset | One entry per operator-configured agent preset | the preset's label only — never the command it runs | no | a freshly set-up gateway offers 3 presets already: Claude, Codex, and Agy (see the `installation` area's setup-file defaults, R20a) — an operator who has edited the preset list, including down to none, sees exactly what they left it with, not these 3 |
| 6 | Sheet status | What the Operator sees while destinations are loading or when there are none | `"Loading…"` · `"No destinations available."` · `"Session expired. Log in again to continue."` · `"Could not load destinations."` | no | absent once destinations are showing |

## Behaviors & Operations

### Open the sheet

- **Triggers:** tapping the switcher's FAB.
- **Blocked when:** never structurally — the FAB itself is disabled first
  whenever herdr is unreachable (see switcher.md), so a disabled FAB cannot
  be tapped at all.
- **What changes:** the destination and preset list is fetched fresh — never
  reused from a previous opening, so a destination that appeared or vanished
  since the sheet was last opened is always current. The Destination field
  and the Type field both start with a value already selected — the first
  destination in the list, and Shell — so New is available the moment the
  sheet opens, without the Operator having to touch either field first (per
  pbi-053 D3).
- **Side effects:** none to the switcher screen underneath; its own list,
  scroll position, and grouping are untouched.
- **Afterwards:** the Operator sees every destination that currently exists —
  including one with no agent running in it, which the switcher's own list
  never shows — plus every configured preset, collapsed into two one-line
  fields with no second fetch needed.

### Select a destination

- **Triggers:** tapping the Destination field to open its list, then tapping
  any destination in that list.
- **What changes:** the tapped destination becomes the selected one and the
  list collapses back to a one-line summary showing it; any previous
  selection is replaced. Opening the Destination field closes the Type
  field's list first, if it was open (per pbi-053 D7).
- **Side effects:** none — no request is sent yet.
- **Afterwards:** the Destination field shows exactly the selected
  destination's summary; nothing else changes.

### Select a type

- **Triggers:** tapping the Type field to open its list of Shell and every
  configured preset, then tapping one.
- **What changes:** the tapped type becomes the selected one and the list
  collapses back to a one-line summary showing it; any previous selection is
  replaced. Opening the Type field closes the Destination field's list
  first, if it was open (per pbi-053 D7).
- **Side effects:** none — no request is sent yet; selecting a type never
  creates anything by itself (per pbi-053 D1).
- **Afterwards:** the Type field shows exactly the selected type's summary;
  nothing else changes.

### Create a shell

- **Triggers:** tapping New with Shell selected as the type.
- **Blocked when:** a create request is already in flight — New is disabled
  for the duration, so a second tap cannot fire a second request.
- **What changes:** a plain shell is created in the selected destination's
  folder when it resolves; when it does not resolve, the shell is still
  created — herdr is left to work out where.
- **Side effects:** none to the sheet's own destination list.
- **On failure:** the backend's own error message appears inside the still-
  open sheet; nothing closes, and New re-enables so the Operator can retry or
  pick a different destination or type.
- **Afterwards:** the sheet closes and the Operator is taken directly into
  the new shell's terminal (see terminal-detail.md).

### Start an agent

- **Triggers:** tapping New with a preset selected as the type.
- **Blocked when:** a create request is already in flight (same guard as
  Create a shell). A destination whose folder cannot be resolved refuses this
  action specifically — starting an agent there would put it in an unrelated
  folder, so the sheet shows that refusal as an inline error instead.
- **What changes:** an agent is started under the selected preset, in the
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
- **R6.** Selecting a destination or a type only changes what is selected;
  creating a shell or starting an agent always requires a separate, explicit
  New confirmation — selection is never itself the create trigger (per
  pbi-053 D1, D2).
- **R7.** The Destination field and the Type field are collapsed to a
  one-line summary by default; opening one to choose from its full list
  always closes the other's list first if it was open — only one list is
  ever open at a time (per pbi-053 D6, D7).
- **R8.** Every field, caveat, path, and disambiguator shown on a destination
  or type carries over unchanged into the collapsed/expanded presentation —
  collapsing a field never hides information the Operator could see before,
  it only changes when that information is shown (per pbi-053 D8).

## Edge Cases Settled

- **No presets configured.** Shell is still offered; the Type field's list is
  simply one entry.
- **No destinations at all** (herdr up but nothing open). The sheet shows
  "No destinations available." rather than an empty list.
- **The session expired while the sheet was opening.** Shown as "Session
  expired. Log in again to continue." rather than a generic error.
- **Tapping New twice quickly.** Only one request is ever sent; New is
  disabled for the duration of the first.

## Open Gaps

- No current screenshot exists under `visuals/create-sheet/` for the open
  sheet, a caveat row, an inline error state, two disambiguated rows, or
  either field's dropdown open.
- The dropdown popup's on-screen behavior on mobile Safari/WebKit has not
  been manually verified on a real device — this repo has previously hit
  WebKit-specific rendering bugs with overlay positioning (see
  `pbi-027-visual-viewport-keyboard`), and the popup is anchored directly
  under its own field rather than floating independently of the sheet
  (chosen specifically to avoid that known failure mode), which has only
  been proven by automated tests — they cannot observe real WebKit rendering
  (per pbi-053 D4).

## Visuals

No current snapshot — see Open Gaps.

## Pointers (implementation)

- `web/src/views/create-sheet.ts` — `renderCreateSheet`, `destinationCaveat`,
  `collisionSuffixes` (R4's disambiguator), the two dropdown fields (trigger +
  popup listbox, `openDropdown` mutual-exclusion state per R7), the New
  button (`selectedIndex`/`selectedPreset`, `handleAction`), and the
  open/close/select/submit logic described above.
- `web/src/views/switcher.ts` — the FAB that opens this sheet (see
  switcher.md).
- `web/src/api.ts` — `fetchCreateOptions`, `createPane`, `createAgent`.
- `web/src/main.ts` — `NewPaneRef`, the minimal reference this sheet builds
  on success and hands to the Operator's next screen.
- `docs/specs/web-api.md` — the backend contract this sheet consumes.

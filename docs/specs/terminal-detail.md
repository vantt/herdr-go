---
area: terminal-detail
updated: 2026-07-21
sources: [terminal-overlay-tweaks, web-create-sheet, home-shell-workspaces, pbi-030-terminal-url-linkify]
decisions: [a04d2754-8182-4188-9861-c93257ec8841, S5, hsw-D5, 88dcc7fc-1b10-4d6c-b51b-72f5eb6a4402]
coverage: partial
---

# Spec: Terminal Detail

Terminal Detail lets a signed-in operator observe one coding agent's current terminal from a phone, adjust the view, send a text reply, or send common navigation keys without taking direct control of the terminal surface.

## Entry Points & Triggers

- Selecting an agent in the agent list opens that agent's terminal detail.
- Selecting a shell entry in the agent list (a plain-shell pane in a
  workspace with no agents, `switcher.md`) opens that specific pane's
  terminal detail the same way (per hsw-D5).
- Successfully creating a shell or agent from the create sheet (`create-sheet.md`)
  opens directly into its terminal detail — the Operator never lands back on
  the agent list first (per parent D6, `new-shell-new-agent`).
- Back returns to the agent list.
- Opening the screen loads the current terminal immediately and continues refreshing it.
- Type opens the reply panel; Keys opens the navigation-key panel; either panel can switch directly to the other or close.

## Data Dictionary

| # | Element | Meaning | Values | Required | Default |
|---|---|---|---|---|---|
| 1 | Terminal title | The selected agent's display name. For a pane opened straight from creating it, or from a shell entry on the agent list, no full agent record exists — the title is derived instead from the minimal reference already in hand: "shell" for a plain shell, or the started agent's name for an agent (per S5, reused as-is by hsw-D5 for shell entries) | display text | yes | selected agent, or the minimal reference described above |
| 2 | Connection state | Whether a current screen can be shown | `Loading` — initial contact pending · `Live` — screen available · `Pane gone` — selected terminal no longer exists · `Disconnected` — refresh failed | yes | `Loading` |
| 3 | Terminal screen | The selected agent's latest visible output; any URL in the output renders as a clickable link | read-only terminal content, auto-linkified | yes | latest available |
| 4 | Reply text | Free-text input sent to the selected agent | text; empty text is not sent | no | empty |
| 5 | Press Enter (submit) | Whether Enter follows the reply text | on/off | yes | on |
| 6 | Navigation keys | Common controls for interactive prompts | Up · Down · Enter · Left · Right · Space · Escape | no | — |
| 7 | Zoom | Terminal text size | 7–22, adjusted one step at a time | yes | 12 |

## Behaviors & Operations

### Observe

- **Runs when:** the screen opens and at regular intervals while it remains open.
- **What changes:** the displayed screen and connection state follow the latest result; unchanged content is not redrawn.
- **Side effects:** none.
- **Afterwards:** the operator sees the latest available output or a clear unavailable/disconnected state; the coding agent continues running independently.

### Pan and zoom

- **Blocked when:** zoom reaches its lower or upper limit.
- **What changes:** the operator moves around wide/tall output or adjusts text size; terminal lines keep their natural shape rather than wrapping to the phone width.
- **Side effects:** none.
- **Afterwards:** only the operator's view changes; the coding agent receives no input.

### Send a text reply

- **Blocked when:** reply text is empty; a failed send keeps the panel open and marks the input invalid.
- **What changes:** a non-empty reply is sent; when Press Enter is on, Enter follows the text. During sending, duplicate sends are temporarily blocked.
- **Side effects:** success clears the input, closes the panel, and refreshes the terminal promptly.
- **Afterwards:** the operator sees the refreshed prompt/output; the coding agent receives the reply and optional Enter key.

### Send navigation keys

- **Blocked when:** the request cannot reach the selected agent.
- **What changes:** one selected key is sent at a time.
- **Side effects:** the screen refreshes promptly; the panel stays open so the operator can send a sequence.
- **Afterwards:** the operator sees the agent's updated interactive prompt; the agent receives exactly the selected key.

### Open a bottom panel

- **Runs when:** the operator opens Type or Keys.
- **What changes:** only one panel is open; the screen reserves enough lower space for the panel and scrolls to the newest content.
- **Side effects:** closing the panel removes the temporary reserved space; switching panels happens in one action.
- **Afterwards:** the operator continues to see the bottom prompt immediately above the open panel (per decision a04d2754-8182-4188-9861-c93257ec8841).

## Actors & Access

| Capability | Signed-in operator | Visitor without a valid session | Coding agent |
|---|---|---|---|
| Observe terminal | ✓ | — | supplies current output |
| Pan/zoom | ✓ | — | unaffected |
| Send reply or keys | ✓ | — | receives input |
| Continue when browser disconnects | — | — | ✓ |

## Business Rules

- **R1.** The terminal surface is observational; input occurs only through Type or Keys.
- **R2.** The free-text footer launcher is labeled Type (per decision a04d2754-8188-9861-c93257ec8841).
- **R3.** Press Enter (submit) defaults on and can be turned off before sending (per decision a04d2754-8188-9861-c93257ec8841).
- **R4.** Only one bottom panel is open at a time, and each offers direct switching to the other.
- **R5.** Opening either bottom panel preserves visibility of the newest prompt; closing it restores the normal viewport (per decision a04d2754-8188-9861-c93257ec8841).
- **R6.** A pane opened straight from creating it is immediately observable
  and repliable without waiting for a fuller agent record to exist — the
  pane's own id is all this screen needs to start reading and sending input
  (per S5, `herdr-port.md` R12).
- **R7.** A pane opened from a shell entry (an already-existing plain-shell
  pane, not one just created) uses the same minimal reference as R6, not a
  full agent record — there never is one to fetch for a plain shell (per
  hsw-D5).

## Edge Cases Settled

- A URL appearing anywhere in the pane output is rendered as a clickable/hoverable link (via xterm's WebLinksAddon); this holds even though the terminal surface is otherwise read-only (`disableStdin: true`), since link handling binds to mouse events, not stdin.
- Empty reply text sends nothing and leaves the visible state unchanged.
- A missing selected terminal shows Pane gone; a refresh failure shows Disconnected.
- Repeated identical screen content is not redrawn.
- Terminal dimensions and zoom stay within bounded ranges.
- A short panel may need no additional reserved space, but opening it still moves the view to the newest content.

## Open Gaps

- URL auto-linkify is not under automated test coverage: the verify command (`tsc`/build + existing vitest suite) is green identically before and after the change, since no test exercises `WebLinksAddon` behavior. Confirmed manually only (URL in pane output renders clickable); jsdom's missing canvas `getContext` makes a real xterm-render assertion impractical in this repo's current test setup.
- No current terminal-detail snapshot is stored; capture both Type-open and Keys-open states with the bottom prompt visible.
- Automated layout tests do not yet measure the bottom-panel inset; current proof is behavior verification plus screen logic inspection.
- Automated coverage does not yet exercise every connection state, reply default, panel switch, send failure, key sequence, or inset restoration.
- The exact user-facing state after a session expires while this screen is already open is not separately specified.

## Visuals

No snapshot is currently available. Needed: Type-open and Keys-open mobile states showing the newest prompt above the panel.

## Pointers (implementation)

- `web/src/views/terminal.ts` — screen refresh, sizing, zoom, reply/keys panels, and inset behavior; `terminalHead` derives the title/kind shown from either a full agent record or the minimal post-create reference (S5).
- `web/src/styles.css` — terminal viewport, footer, bottom panels, and key hierarchy.
- `web/src/api.ts` — screen reads, reply submission, and navigation-key requests.
- `web/src/main.ts` — navigation into and out of terminal detail; `NewPaneRef`, the minimal post-create reference (S5).
- `web/test/terminal.test.ts` — current narrow unit coverage.
- `.bee/cells/terminal-overlay-tweaks-1.json` — captured verification evidence.
- `docs/specs/create-sheet.md` — the screen this one is entered from after a create.

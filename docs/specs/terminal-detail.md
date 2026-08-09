---
area: terminal-detail
updated: 2026-07-27
sources: [terminal-overlay-tweaks, web-create-sheet, home-shell-workspaces, pbi-030-terminal-url-linkify, pbi-025-terminal-detail-url, switcher-login-url, pbi-027-visual-viewport-keyboard, terminal-scrollback-agent-panes, terminal-scroll-nudge-buttons]
decisions: [a04d2754-8182-4188-9861-c93257ec8841, S5, hsw-D5, 88dcc7fc-1b10-4d6c-b51b-72f5eb6a4402, 55268bb3-3ce0-486c-8eb7-2c299dd52fc2, 4479bd23-b0f1-4571-bf03-f4c35bdde575, 76c625b2-42a1-4f15-9feb-66f992ccdaf6, 31b0a5d4-18ec-4ec1-bf05-5b18850de664, fd5cfe33-7eca-4b0b-a636-228ccc7a5bc5, swlogin-D1, pbi027-D1, pbi027-D2, pbi027-D3, pbi027-D4, tsap-D1, tsap-D2, tsap-D3, tsap-D4, tsap-D5, tsap-D9, tsnb-D1, tsnb-D2, tsnb-D3, tsnb-D4, tsnb-D6, tsnb-D7]
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
- This screen has its own link. Opening that link directly — a saved bookmark, a shared link, or refreshing the page while the screen is already open — opens the same pane's terminal detail directly, without visiting the agent list first (per pbi025-D1/D3).
- Back — the on-screen control or the device's own back control — returns to the agent list, in exactly one step either way (per pbi025-D2).
- Opening the screen loads the current terminal immediately and continues refreshing it.
- Type opens the reply panel; Keys opens the navigation-key panel; either panel can switch directly to the other or close.
- Scrolling the terminal screen up past what is currently loaded asks for
  older history (per tsap-D1–tsap-D5, tsap-D9).
- Up/Down scroll controls beside the terminal offer the same two requests
  directly, without needing to scroll by hand: Up asks for older history,
  Down returns to the live end — present for every kind of pane, not only
  certain coding agents (per tsnb-D1–tsnb-D3).

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
| 8 | Floating header | Overlay showing the pane's display name and its own folder path, while the operator scrolls down through the terminal content | display text (name) + path text, path omitted when unresolved | no | hidden |

## Behaviors & Operations

### Observe

- **Runs when:** the screen opens and at regular intervals while it remains open, except while older history is being shown (see Scroll back through history) — regular refresh resumes automatically, immediately rather than waiting for its next scheduled turn, once the operator returns to the live end (per tsnb-D4, tsnb-D7).
- **What changes:** the displayed screen and connection state follow the latest result; unchanged content is not redrawn.
- **Side effects:** none.
- **Afterwards:** the operator sees the latest available output or a clear unavailable/disconnected state; the coding agent continues running independently.

### Pan and zoom

- **Blocked when:** zoom reaches its lower or upper limit.
- **What changes:** the operator moves around wide/tall output or adjusts text size; terminal lines keep their natural shape rather than wrapping to the phone width.
- **Side effects:** none.
- **Afterwards:** only the operator's view changes; the coding agent receives no input.

### Reveal the floating header while reading

- **Runs when:** the terminal viewport's scroll position moves downward (the
  operator is reading forward through loaded content), regardless of
  whether that content is the live end or previously loaded history.
- **Blocked when:** nothing — this is a passive display, never a network
  request.
- **What changes:** a header overlays the top of the terminal viewport,
  showing the pane's display name and, when known, its own folder path.
- **Side effects:** none; the header never affects polling, history
  loading, or what is sent to the agent.
- **Afterwards:** the header fades out as soon as the scroll position moves
  upward, or after a few seconds with no further scroll in either
  direction — matching the Up/Down scroll controls' own idle-fade timing
  (R19).

### Scroll back through history

- **Runs when:** the operator scrolls the terminal screen up past the top of
  what is currently loaded, or uses the Up scroll control beside the
  terminal (per tsnb-D1/tsnb-D3) — both ask for the same older-history
  request.
- **Blocked when:** the request cannot reach the selected agent.
- **What changes:** older output is loaded and shown above the current
  content, replacing the whole displayed screen with the longer result
  (this is not an incremental append — the redraw is a full one, per
  tsap-D3/tsap-D9).
- **On a coding agent whose interactive display holds no extra history to
  give** (most coding-agent CLIs take over the whole screen and, by the same
  long-standing terminal convention any full-screen program follows, keep no
  history behind that display): the screen instead asks that agent to
  scroll its own display backward, the same as if the operator pressed the
  key for it directly, then always returns that display to its live end
  afterward — whether or not anything new was actually revealed (per
  tsap-D1/tsap-D4/tsap-D5). This never sends any other input to the agent
  and never changes what the agent is doing.
- **On a pane that has printed less than one screen's worth of output** (a
  short pane, agent or plain shell alike): nothing further exists to load
  either way. The screen may still try the display-scroll request above
  once before concluding there is nothing more — an accepted, low-cost
  round trip, not a defect (see Open Gaps).
- **Side effects:** the regular refresh stops running for as long as older
  content is being shown, so it cannot overwrite what was just loaded before
  the operator has had a chance to see it (per tsnb-D4) — beyond that, none
  beyond the display-scroll request described above, which is assumed but
  not proven to be harmless when it lands on an ordinary shell prompt rather
  than a coding agent's own display (see Open Gaps).
- **Afterwards:** the operator sees either more real output, or the coding
  agent's own redraw of its earlier display. Scrolling back down by hand,
  using the Down scroll control beside the terminal (per tsnb-D3), or
  opening the reply or navigation-key panel (per tsnb-D6, since both panels
  are for driving the live session) all return the view to the live,
  continuously-refreshing view — any of the three resumes the regular
  refresh immediately rather than waiting for its next scheduled turn (per
  tsnb-D4, tsnb-D7).

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
- **What changes:** only one panel is open; the screen reserves enough lower space for the panel and scrolls to the newest content. When Type is open and the phone's on-screen keyboard appears, the reserved space also grows so Type's own input and Send control stay above the keyboard, not just the terminal content above it; if the keyboard's height changes while Type stays open (for example a suggestion bar toggling), the reserved space follows it live, without moving the operator's current scroll position (per pbi027-D3, pbi027-D4).
- **Side effects:** closing the panel removes the temporary reserved space, including any keyboard-aware portion; switching panels happens in one action. The Up/Down scroll controls are hidden for as long as either panel is open, since both panels already return the view to the live end and occupy the same corner of the screen (per tsnb-D1).
- **Afterwards:** the operator continues to see the bottom prompt immediately above the open panel, and — while Type is open — Type's own input stays above the on-screen keyboard too (per decision a04d2754-8182-4188-9861-c93257ec8841; pbi027-D3).

### Reopen from this screen's own link

- **Runs when:** the operator opens this screen's link directly (bookmark, shared link, or a page refresh while the screen is already open).
- **Blocked when:** the link's pane no longer exists, or the operator is not currently signed in.
- **What changes:** a valid, still-existing pane opens straight into its terminal detail. A signed-out operator sees the sign-in screen first; on successful sign-in they land in that same terminal detail if the pane still exists, otherwise the agent list (per pbi025-D5).
- **Side effects:** a link whose pane no longer exists opens the agent list instead, with no error message (per pbi025-D3) — indistinguishable from opening the agent list any other way.
- **Afterwards:** the operator either lands directly back in the terminal they were viewing, or lands on the agent list with no explanation (stale link), or lands back in that terminal after signing in.

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
- **R5.** Opening either bottom panel preserves visibility of the newest prompt; closing it restores the normal viewport (per decision a04d2754-8188-9861-c93257ec8841). While Type is open, the reserved space also expands to keep Type itself above the phone's on-screen keyboard when one is showing (per pbi027-D3); Keys never needs this, since Keys has no text entry to raise a keyboard (per pbi027-D1).
- **R6.** A pane opened straight from creating it is immediately observable
  and repliable without waiting for a fuller agent record to exist — the
  pane's own id is all this screen needs to start reading and sending input
  (per S5, `herdr-port.md` R12).
- **R7.** A pane opened from a shell entry (an already-existing plain-shell
  pane, not one just created) uses the same minimal reference as R6, not a
  full agent record — there never is one to fetch for a plain shell (per
  hsw-D5).
- **R8.** This screen has its own dedicated link. The agent list (`switcher.md` R15) and the sign-in screen (`login.md` R1) each have their own dedicated link too, symmetric with this one (per pbi025-D1, extended to the other two screens by swlogin-D1 — originally the agent list and sign-in shared one undifferentiated link, per the now-superseded pbi025-D4).
- **R9.** A stale or invalid link (the referenced pane no longer exists) opens the agent list with no error message — silent, identical to opening the agent list any other way (per pbi025-D3).
- **R10.** Opening this screen's link while signed out shows the sign-in screen; signing in successfully then returns the operator to that same terminal detail, if the pane still exists (per pbi025-D5).
- **R11.** If the on-screen keyboard's height changes while Type stays open, the reserved space updates to match, but the operator's current scroll position in the terminal is left alone — only opening Type the first time scrolls to the newest content (per pbi027-D4).
- **R12.** Whether a pane has real extra history to scroll back into is
  discovered by actually reading it, never assumed from which coding agent
  is running or any status the terminal host reports about it — an agent
  can change whether it takes over the whole screen between its own
  versions while its name stays the same, and the host's own signal for
  "how much history exists" is not reliable enough on its own to branch on
  (per tsap-D1/tsap-D3, `herdr-port.md` R18).
- **R13.** Scrolling back is always a full redraw of the whole screen with
  the longer result, never an incremental addition to what was already
  shown (per tsap-D3/tsap-D9).
- **R14.** A coding agent's display asked to scroll backward is always
  asked to return to its live end afterward, whether or not the backward
  scroll revealed anything — the operator is never left mid-scroll in an
  agent's own display by this screen's own action (per tsap-D4/tsap-D5).
- **R15.** The screen never sends the older-history display-scroll request
  while its own regular refresh is in flight, and never lets a regular
  refresh land while a display-scroll request is in flight — the two never
  interleave (per tsap-D3).
- **R16.** Up/Down scroll controls beside the terminal ask for the same
  older-history/return-to-live requests as scrolling by hand, for every kind
  of pane — not only coding agents whose display can hold extra history
  (per tsnb-D2).
- **R17.** The regular refresh does not run while older history is being
  shown; it resumes, immediately rather than waiting for its next scheduled
  turn, as soon as the operator returns to the live end — by scrolling back
  down, using the Down scroll control, or opening the reply/navigation-key
  panel (per tsnb-D4, tsnb-D6, tsnb-D7).
- **R18.** The Up/Down scroll controls are hidden while the reply or
  navigation-key panel is open (per tsnb-D1).
- **R19.** The Up/Down scroll controls fade from view after a few seconds
  with no touch or scroll on the terminal screen, and reappear immediately
  on the next touch or scroll (per tsnb-D1).
- **R20.** The floating header (pane name + path) shows while the terminal
  viewport's scroll position moves downward, and fades out on an upward
  move or a few seconds of no further scroll — an overlay on top of the
  terminal content, never reserving layout space of its own.

## Edge Cases Settled

- A URL appearing anywhere in the pane output is rendered as a clickable/hoverable link (via xterm's WebLinksAddon); this holds even though the terminal surface is otherwise read-only (`disableStdin: true`), since link handling binds to mouse events, not stdin.
- Empty reply text sends nothing and leaves the visible state unchanged.
- A missing selected terminal shows Pane gone; a refresh failure shows Disconnected.
- Repeated identical screen content is not redrawn.
- Terminal dimensions and zoom stay within bounded ranges.
- A short panel may need no additional reserved space, but opening it still moves the view to the newest content.
- The on-screen keyboard changing height while Type is already open (for example a suggestion bar appearing) does not force the view back to the newest content — only the initial opening of Type does that (per pbi027-D4).
- On phones/browsers that do not report on-screen-keyboard height, Type's reserved space falls back to today's panel-only sizing with no error (per pbi027-D2).
- A short pane (agent or plain shell, printed less than one screen's worth of
  output) has nothing more to load when scrolled back, either way; the
  screen may still try the display-scroll request once before concluding
  that, at low, accepted cost (per tsap-D4/tsap-D5).
- Once the loaded history grows past this screen's own display limit, or the
  loaded content shrinks, the operator's reading position is kept as close
  as possible to where it was rather than left at a stale or invalid
  position (per tsap-D3).
- The Up/Down scroll controls fade out after a few seconds of no touch or
  scroll interaction with the terminal screen, and reappear on the next one
  (per tsnb-D1).

## Open Gaps

- URL auto-linkify is not under automated test coverage: the verify command (`tsc`/build + existing vitest suite) is green identically before and after the change, since no test exercises `WebLinksAddon` behavior. Confirmed manually only (URL in pane output renders clickable); jsdom's missing canvas `getContext` makes a real xterm-render assertion impractical in this repo's current test setup.
- No current terminal-detail snapshot is stored; capture both Type-open and Keys-open states with the bottom prompt visible.
- Automated layout tests do not yet measure the bottom-panel inset. The on-screen-keyboard space calculation itself has unit coverage, but the resulting on-screen layout (whether the panel and terminal content visibly clear the keyboard) is not automated-tested — same jsdom rendering limitation noted above for URL auto-linkify.
- Automated coverage does not yet exercise every connection state, reply default, panel switch, send failure, key sequence, or inset restoration.
- The exact user-facing state after a session expires while this screen is already open and rendered (mid-view expiry, as opposed to opening this screen's link while already signed out — R10) is not separately specified.
- The display-scroll fallback (R12/R14) has only been verified, live, against
  one coding agent's own display. Whether every other full-screen coding
  agent reacts the same way to that same scroll key — including agents not
  yet encountered — is unconfirmed. If a future agent turns out not to
  react, the operator simply sees no older history for it, with no further
  fallback designed yet. Answered by: a live check the next time this is
  found not to hold for some agent.
- Sending the display-scroll request into an ordinary short shell prompt
  (rather than a coding agent's own display) is assumed harmless — most
  terminal programs ignore a key they have no binding for — but this has
  not been independently confirmed for every kind of program that could be
  running in a short pane. Accepted as low-severity (the operator gestured
  the request, and no text is ever submitted by it) rather than proven safe.
- If the display-scroll request's return-to-live-end step itself fails to
  reach the pane, the pane can be left scrolled away from its live view
  until the operator next asks to scroll back (which retries and restores
  it). Tracked as a small follow-up, not yet fixed (`docs/backlog.md`
  PBI-058).
- Returning to the live end while regular refresh is paused (scrolling back
  down by hand, the Down scroll control, or opening a bottom panel) relies
  on that action firing the same signal a real browser fires after such a
  scroll — confirmed to hold in real browsers, but this repo's automated
  test environment does not fire that signal on its own, so the relevant
  tests trigger it explicitly to prove the underlying logic in isolation.
  Same class of gap as the on-screen-keyboard and URL-linkify items above:
  confirming the felt experience (scroll controls, fade timing, and
  resume-on-return-to-live) on a real mobile browser is still pending.

## Visuals

No snapshot is currently available. Needed: Type-open and Keys-open mobile states showing the newest prompt above the panel.

## Pointers (implementation)

- `web/src/views/terminal.ts` — screen refresh, sizing, zoom, reply/keys panels, and inset behavior; `terminalHead` derives the title/kind/path shown from either a full agent record or the minimal post-create reference (S5); the floating `.term-header` overlay (R20) reads the same `path`.
- `src/herdr/wire.rs` — `Snapshot::path_for_pane_id` joins an agent's `pane_id` against `panes[]` for its own folder (the same join `ShellRow`'s path already used).
- `web/src/styles.css` — terminal viewport, footer, bottom panels, and key hierarchy.
- `web/src/api.ts` — screen reads (including the older-history request),
  reply submission, and navigation-key requests.
- `web/src/main.ts` — navigation into and out of terminal detail; `NewPaneRef`, the minimal post-create reference (S5); `pathForRoute`/`parseTerminalPaneId`/`resolvePaneRef`/`resolveLoginRedirect` build/parse this screen's link and resolve it back to a pane on load or after sign-in (pbi025-D1/D2/D3/D5).
- `web/test/terminal.test.ts` — current narrow unit coverage.
- `web/test/main.test.ts` — link build/parse, pane resolution, stale-link fallback, and sign-in-redirect coverage (pbi025-D1-D5).
- `.bee/cells/terminal-overlay-tweaks-1.json` — captured verification evidence.
- `.bee/cells/pbi-027-keyboard-inset-1.json` — captured verification evidence for the on-screen-keyboard reserved space.
- `docs/specs/create-sheet.md` — the screen this one is entered from after a create.
- `src/herdr/pane_scroller.rs` — the older-history read and its display-scroll
  fallback (R12-R15, `herdr-port.md` R18).
- `src/web/screen.rs` — the older-history flag on the screen-read route
  (`web-api.md` R10).
- `.bee/cells/terminal-scrollback-agent-panes-{1,2,3}.json` — captured
  verification evidence for the scroll-back-through-history feature.
- `.bee/cells/terminal-scroll-nudge-buttons-{1,2}.json` — captured
  verification evidence for the regular-refresh pause/resume behavior and
  the Up/Down scroll controls.

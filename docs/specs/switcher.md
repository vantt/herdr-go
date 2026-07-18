---
area: switcher
updated: 2026-07-18
sources: [terminal-workspace-org, dark-only-ui, agent-card-legibility]
decisions: [D2, D3, D4, D5, D6, D7, D8, de2781bf]
coverage: partial
---

# Spec: Switcher (home agent list)

The screen the Operator lands on right after logging in: every AI coding agent
terminal session currently running is listed here, so the Operator can pick one to
open and interact with live. On a phone this is the entry point before opening a
specific terminal's live screen (a separate area, not covered here).

## Entry Points & Triggers

- App load with a valid session → the switcher screen, which immediately fetches
  and renders the current agent list.
- App load with no/expired session → the login screen instead (a separate area).
- Pull-down gesture past a short threshold while already scrolled to the top, or
  tapping the refresh icon → re-fetches and re-renders the list.
- Tapping an agent card → opens that agent's live terminal screen (separate area).
- Tapping a workspace section's header (only present when more than one workspace
  is currently shown) → collapses or expands that section in place; no re-fetch.
- Tapping the logout icon → ends the session, returns to login.

## Data Dictionary

Per agent row, in display order:

| # | Element | Meaning | Values | Required | Default |
|---|---------|---------|--------|----------|---------|
| 1 | Terminal identity | Primary line: the agent's live terminal title alone (no longer kind-prefixed — kind now shows via rows 2 and 5 below), wraps onto a second line rather than being cut to one | free text, falls back to the kind alone when no title is available yet | yes | — |
| 2 | Kind + tab caption | One caption line under the identity line, naming the agent kind and, when known, the tab (inside the agent's workspace) the terminal lives in | `"{kind} · {tab}"` when a tab name is known, or just the kind alone when it isn't | yes | tab part omitted when unknown |
| 3 | Status badge | The agent's current readiness | `working` — actively producing output · `blocked` — waiting on the Operator · `done` — finished, idle since · `idle` — no work in progress · `unknown` — herdr reported a value this app doesn't recognize | yes | — |
| 4 | Kind watermark | A faint, decorative background monogram (the kind's first letter) tinted a color unique to that kind — a fast visual anchor so the Operator can recognize a kind at a glance without reading text; carries no information a screen reader needs (the kind is already read from row 2) | one letter, color derived from the kind (same kind always gets the same color) | yes | — |

Per workspace-group header (rendered only when the visible agents currently span
more than one workspace — see R2):

| # | Element | Meaning | Values | Required | Default |
|---|---------|---------|--------|----------|---------|
| 5 | Workspace label | herdr's own name for the workspace this group of agents belongs to | free text, herdr's own project/workspace name | yes | — |
| 6 | Workspace status badge | Rollup readiness for the whole workspace, same vocabulary as row 4 | `working` / `blocked` / `done` / `idle` / `unknown` — herdr's own summary across every agent in that workspace | yes | `unknown` on a data join miss (never a crash) |

Header-level (not per row): a small health dot — herdr reachable / herdr
unreachable / health check itself failed — checked once per screen load,
independent of the agent list.

## Behaviors & Operations

### Load / refresh the list

- **Triggers:** screen open, pull-to-refresh gesture, tapping the refresh icon.
- **Blocked when:** never.
- **What changes:** the full agent list is re-fetched and re-rendered from scratch.
- **Side effects:** none (read-only).
- **Afterwards:** the Operator sees the current agent cards, an empty-state message
  when herdr currently has nothing running, or an unreachable message when the
  gateway itself cannot be reached; the refresh icon spins while the fetch is in
  flight. If the fetch reports the session has expired, the Operator is silently
  returned to the login screen with no error message — indistinguishable from a
  first-time visit (per this app's fail-closed, silent auth design).

### Group by workspace (display-only, no network call)

- **Runs when:** every time the list is rendered, after any load/refresh.
- **What changes:** if the currently visible agents span more than one distinct
  workspace, the list is arranged into one section per workspace, ordered
  alphabetically by workspace label (per D7), each carrying the header described
  above. If every visible agent belongs to the same single workspace, the list
  stays the plain flat list it always was — only the tab-label caption (row 3) is
  new (per D3).
- **Side effects:** none.
- **Afterwards:** with several active workspaces, the Operator scans a short list
  of workspace headers (each carrying its own rollup status) instead of a long flat
  list, to find which workspace needs attention. With one workspace, nothing about
  the list's shape changes from before.

### Collapse / expand a workspace section

- **Triggers:** tapping a workspace section's header (present only when more than
  one workspace is currently shown).
- **Blocked when:** never.
- **What changes:** only that section's rows toggle hidden or shown; other
  sections are unaffected. Every section starts expanded the first time a
  multi-workspace list renders (per D8).
- **Side effects:** none; nothing is re-fetched.
- **Afterwards:** the Operator sees the collapsed section reduced to just its
  header until tapped again. This state is remembered only for as long as the
  screen stays open in this browsing session (it survives a manual refresh of the
  same open screen) — a fresh app load (e.g. a page reload) always starts every
  section expanded again, never remembered across visits (per D6).

### Log out

- **Triggers:** tapping the logout icon.
- **What changes:** the Operator's session ends.
- **Side effects:** none beyond ending the session.
- **Afterwards:** the Operator is returned to the login screen.

## Actors & Access

Single-operator system — there is exactly one human role.

| Capability | Operator (valid session) | Anyone without a valid session |
|---|---|---|
| See the agent list | ✓ | — (silently returned to login, no error reveals the screen exists) |
| See workspace grouping / collapse sections | ✓ | — |
| Open an agent's live terminal | ✓ | — |
| Log out | ✓ | n/a |
| See the health dot | ✓ (shown only within this screen, though the underlying health check itself requires no session) | — |

## Business Rules

- **R1.** Workspace and tab names shown anywhere on this screen always come from
  herdr's own live data, resolved fresh on every list load — never cached or
  hand-entered (per D2).
- **R2.** The workspace grouping (and its header row) appears only when the
  currently visible agents span more than one distinct workspace; a single active
  workspace shows no extra grouping chrome (per D3).
- **R3.** Workspace sections sort alphabetically by workspace label; agents inside
  a section keep the order the data arrived in — no separate sort (per D7).
- **R4.** Every workspace section starts expanded; nothing auto-collapses on load
  (per D8).
- **R5.** Collapse/expand state exists only for the current open screen — it is
  never saved, and a fresh app load always starts every section expanded (per D6).
- **R6 (not yet implemented — backlog PBI-009).** Aggregating agents from more
  than one physical machine into a single switcher view — no "machine" concept
  exists in this app or in herdr today (per D5).
- **R7 (not yet implemented — backlog PBI-008).** Renaming a terminal/tab/workspace
  directly from this screen — today that can only be done from herdr's own
  interface.
- **R8 (app-wide, not switcher-specific).** This screen renders dark-only — there
  is no light theme (per decision `de2781bf`). This applies to the whole web app,
  not just this screen; a future spec for the app shell/login/terminal-detail
  areas should carry the same rule rather than restate it directly.
- **R9.** The kind watermark (Data Dictionary row 4) is purely decorative — it
  never substitutes for the textual kind caption (row 2), and its color is
  always derived the same way from a given kind value, never assigned by hand
  or looked up in a table (per D4, feature `agent-card-legibility`).
- **R10.** The primary identity line shows only the terminal's own title; kind is
  never repeated there — it appears exactly once as text, in the caption (row 2)
  (per D1/D2, feature `agent-card-legibility`).

## Edge Cases Settled

- No agents running anywhere → an empty-state message replaces the list entirely;
  never an empty grouped section.
- A workspace or tab herdr hasn't (or can't) resolve a name for → the tab caption
  is simply omitted for that row (never a placeholder like "undefined"); a
  workspace whose rollup status can't be resolved shows an `unknown` badge instead
  of failing the whole list.
- The gateway itself is unreachable → a "could not reach the gateway" message
  replaces the list; pull-to-refresh or the refresh icon is how the Operator
  retries.

## Open Gaps

- No current screenshot exists under `visuals/switcher/` for either the flat
  (single-workspace) or grouped (multi-workspace) layout — attempted to capture
  one live (isolated demo instance, port 8799) but no headless-browser tooling
  was available in this environment without a system-level install this session
  couldn't authorize (`sudo` required). The user confirmed on their own phone,
  against the live deployed service, that the workspace/project grouping renders
  and reads correctly — this is real UAT, but a stored snapshot under
  `visuals/switcher/` still does not exist. Next agent/session with browser
  tooling (or the user directly) should capture one and refresh this section.
- Resolved (2026-07-18, feature `agent-card-legibility`): the user's earlier
  report of not clearly recognizing what each row represents at a glance turned
  out to be about the title being cut too short to read, and too much of the
  kind repeated redundantly. Addressed by dropping the kind prefix from the
  title (freeing 2 lines of room for the real content) and adding the kind
  watermark as a non-textual recognition aid — see Data Dictionary rows 1/4 and
  R9/R10. Still no stored screenshot to confirm the new layout by eye (same gap
  as above); user has not yet re-confirmed the new card layout on-device.
- The exact rollup rule herdr itself uses to compute a workspace's status (e.g.
  which single status wins when agents inside it disagree) is not documented
  anywhere this app controls — herdr computes and reports it as a single opaque
  value; this app only displays it as given (R1). If herdr's own precedence rule
  is ever needed here, it is a question for herdr's own documentation, not this
  spec.
- The login screen and the individual live-terminal screen (opened by tapping an
  agent card) are separate areas with their own behavior, not yet specced.

## Visuals

No current snapshot — see Open Gaps.

## Pointers (implementation)

- `web/src/views/switcher.ts` — renders this screen; `groupByWorkspace` implements
  the grouping/sort/badge-trigger logic; `kindAccentColor` implements the
  watermark's hash-to-color logic; `renderAgentCard`/`renderWorkspaceSection`
  render the two row shapes; pull-to-refresh listens on `#switcher-body`.
- `web/src/api.ts` — `fetchAgents`, `fetchHealth`, the `AgentRow`/`HealthInfo`
  types.
- `web/test/switcher.test.ts` — unit tests for `groupByWorkspace`'s and
  `kindAccentColor`'s boundary behavior.
- `src/web/api.rs` — `GET /api/agents` handler, `AgentRow` (Rust).
- `src/herdr/wire.rs` — `Snapshot::workspace_label_for` / `tab_label_for` /
  `workspace_status_for` resolvers; `Workspace`/`Tab` wire types.
- `src/herdr/fake.rs` — `FakeHerdr::new()`'s fixture data (used by `--demo` mode
  and tests).
- `src/web/auth.rs` — session/auth mechanics referenced in Actors & Access.
- `web/src/styles.css` — `.status-badge`, `.status-dot`, `.workspace-header`,
  `.workspace-section` rules.

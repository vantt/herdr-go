---
area: web-api
updated: 2026-07-21
sources: [web-create-endpoints, home-shell-workspaces]
decisions: [P1, P2, P4, P6, P7, P9, P10, bc4a65a4, hsw-D1, hsw-D3]
coverage: partial
---

# Spec: Web API (the phone's HTTP surface)

The gateway's face on the tailnet: a single-operator, token-authenticated HTTP
API the phone talks to. It covers logging in, seeing the current agents, and
— since this feature landed — asking for a destination list and creating a
new shell or agent inside one. It never renders anything itself; screens that
consume it are specced separately (see Pointers).

## Entry Points & Triggers

- `POST /api/login` → validate the operator's token, issue a session cookie.
- `POST /api/logout` → invalidate the current session cookie.
- `GET /api/health` → liveness, version, protocol generation, and whether the
  terminal host answers.
- `GET /api/agents` → the switcher list: one row per running agent, plus one
  row per plain-shell pane in a workspace that has no agents at all (per
  hsw-D1/hsw-D3).
- `GET /api/create-options` → the create sheet's opening fetch: every
  workspace as a possible destination, plus the operator's agent presets.
- `POST /api/panes` → open a plain shell in a chosen workspace.
- `POST /api/agents` → start an agent, by preset, in a chosen workspace.
- `GET /api/panes/<id>/screen`, `POST /api/panes/<id>/input`,
  `POST /api/panes/<id>/keys` → read a pane's visible screen and reply to it.
  Predate this spec; see Open Gaps.

## Data Dictionary

| # | Element | Meaning | Values | Required | Default |
|---|---|---|---|---|---|
| 1 | Session cookie | The single proof of authentication. Issued on login, checked on every other route. | httpOnly, `SameSite=Strict`, 7-day expiry | yes (except login/health) | — |
| 2 | Destination — workspace id | The workspace a shell or agent can be created inside | opaque id, read fresh from the snapshot | yes | — |
| 3 | Destination — label | Display name shown for the destination | text | yes | — |
| 4 | Destination — path | The folder a new shell or agent would start in, if it resolves | path, or absent when the join misses | no | absent |
| 5 | Destination — path is live | Whether `path` is the folder's current live directory, as opposed to only its process start directory | boolean | yes | `false` |
| 6 | Preset — label | The name an agent-create request keys into; the only thing about a preset the phone ever sees | text | yes | — |
| 7 | Agent row — pane id | The opaque address the screen/input endpoints take for this agent | text | yes | — |
| 8 | Agent row — status | Drives the switcher's status badge | same values as the terminal host reports, joined per `herdr-port.md` R7 | yes | — |
| 9 | Error body | Shape returned on every non-2xx response except the opaque 404 | `{ "error": "<message>" }` | yes (on error) | — |
| 10 | Shell row — pane id | The opaque address the screen/input endpoints take for this specific shell pane | text | yes | — |
| 11 | Shell row — folder | That pane's own current folder | path | no | absent |
| 12 | Shell row — workspace/tab label | The workspace and tab this shell pane lives in | text | yes | — |

A shell row (hsw-D1/hsw-D2) carries no status, kind, or display fields — none
exist for a plain shell with no agent attached; a shell row is only ever
listed for a workspace with zero agents (hsw-D3).

`argv` — the actual command line a preset runs — is never listed here: it is
never sent to the phone in any response, at any endpoint (per P6). The
operator edits it through `doctor`, not through this API.

## Behaviors & Operations

### Log in

- **Blocked when:** no token is configured, or the submitted token does not
  match (compared in constant time). Both cases return the same opaque 404 as
  any other route (per bc4a65a4) — a wrong token and a missing one look
  identical from outside.
- **What changes:** a new session cookie is issued and recorded.
- **Side effects:** none.
- **Afterwards:** the phone holds a session cookie good for 7 days; every
  other route now accepts it.

### Log out

- **Blocked when:** never — always succeeds, even with no active session.
- **What changes:** the presented session cookie, if any, is invalidated.
- **Side effects:** none.
- **Afterwards:** the phone's cookie no longer authenticates; the next
  protected request gets the opaque 404.

### Check health

- **Triggers:** the phone or an operator probe.
- **Afterwards:** the caller learns the gateway's version, its protocol
  generation, and whether the terminal host currently answers — without
  authenticating.

### List agents (switcher)

- **Triggers:** the switcher screen loading or refreshing.
- **Blocked when:** unauthenticated → opaque 404. The terminal host is
  unreachable → 502.
- **Afterwards:** the Operator sees one row per running agent, each carrying
  its workspace and tab labels and the workspace's rolled-up status, **plus**
  one shell row per plain-shell pane belonging to a workspace with zero
  agents (hsw-D1/hsw-D3) — a workspace with at least one agent contributes no
  shell rows even for its own plain-shell panes, unchanged from before this
  addition (per P1, still true: an agent-having workspace's non-agent panes
  stay invisible here). "List create options" remains the endpoint that never
  drops any workspace at all, agent or not.

### List create options

- **Triggers:** the create sheet opening (FAB tap).
- **Blocked when:** unauthenticated → opaque 404. The terminal host is
  unreachable → 502.
- **What it reads:** every workspace in the current snapshot — including one
  where every pane is a plain shell, which the agent list structurally omits
  (per P1) — plus the operator's configured agent presets.
- **Afterwards:** the Operator sees the full destination list in one round
  trip alongside the preset list, so the sheet needs no second fetch (per
  P4). A destination whose folder cannot be resolved still appears, with no
  path and `path is live: false`, rather than being dropped (per P2). Only
  preset labels are visible; the commands they run are not.

### Open a shell

- **Triggers:** the Operator picking a destination and choosing "Shell".
- **Blocked when:** unauthenticated → opaque 404. The destination no longer
  exists → 409, not 404, so the phone can tell "that workspace closed" apart
  from "log in again" (per P7). The terminal host is unreachable for another
  reason → 502.
- **What changes:** a new tab holding one shell pane is created in the chosen
  workspace, seeded with the destination's resolved folder when one exists.
  When no folder resolves, the terminal host is left to compute its own
  anchor for the new shell — see `herdr-port.md` R17.
- **Side effects:** none beyond the new pane; the desktop's own focus never
  moves (`herdr-port.md` R10).
- **Afterwards:** the Operator holds the new tab and pane ids and can open the
  new shell immediately.

### Start an agent

- **Triggers:** the Operator picking a destination and an agent preset.
- **Blocked when:** unauthenticated → opaque 404. The named preset is not one
  the operator configured → 400, before the terminal host is ever asked (per
  P6 — the request cannot pick what runs). The destination no longer exists,
  or the terminal host refuses the placement → 409. **The destination's
  folder does not resolve → 409, refusing rather than starting the agent in
  an arbitrary folder** (per P10 — this is the one place this area diverges
  from "open a shell": see `herdr-port.md` R17 for why). Any other terminal
  host failure → 502.
- **What changes:** an agent pane is created in the workspace's active tab,
  running the preset's configured command — never a command the request
  supplied.
- **Side effects:** none beyond the new pane; the desktop's own focus never
  moves.
- **Afterwards:** the Operator holds the new tab and pane ids plus the name
  the agent actually started under, and can open it immediately. A
  successful start means the pane exists, not that the agent has finished
  starting (`herdr-port.md` R14).

## Actors & Access

| Capability | Operator (via the phone, authenticated) | Anonymous visitor |
|---|---|---|
| Log in | ✓ | ✓ (only route reachable without a session) |
| See the agent list, including shell rows for zero-agent workspaces | ✓ | — |
| See the destination + preset list | ✓ | — |
| Open a shell | ✓ | — |
| Start an agent by preset | ✓ | — |
| See or influence what an agent preset runs | — | — |

Nobody but the operator, editing config through `doctor`, ever supplies
`argv`. The phone can only select among presets the operator already chose.

## Business Rules

- **R1.** Every route except `/api/login` and `/api/health` requires the
  session cookie; failing that check returns the same opaque 404 as an
  unknown route, never a descriptive rejection (per bc4a65a4).
- **R2.** A destination that no longer exists by the time it is acted on is
  409, never 404 — 404 stays reserved exclusively for the unauthenticated
  answer so the phone can tell the two apart (per P7).
- **R3.** An agent-create request names a preset by label only. `argv` is
  never accepted from the client in any field, and an unknown label is
  rejected before the terminal host is ever called (per P6).
- **R4.** The destination list is assembled from every workspace, not from
  the agent list, so a workspace with no agents still appears (per P1).
- **R5.** Opening a shell and starting an agent are separate routes with
  separate request and response shapes, not one route branching on a type
  field (per P5, new-shell-new-agent).
- **R6.** Starting an agent refuses with 409 when the destination's folder
  does not resolve, rather than falling back to an unrelated folder; opening
  a shell does not need this refusal because its own fallback is safe (per
  P10 — see `herdr-port.md` R17 for the underlying asymmetry).
- **R7.** The agent list response also carries a shell row for every
  plain-shell pane in a workspace with zero agents, resolved from the same
  snapshot fetch as the agent rows (one round trip, not a second endpoint);
  a workspace with at least one agent contributes no shell rows even for its
  own plain-shell panes (per hsw-D1/hsw-D3).

## Edge Cases Settled

- **Unauthenticated request to any protected route.** Opaque 404, identical
  to a request for a route that does not exist.
- **A destination that vanished between opening the sheet and acting on it.**
  409 on either create route, never 404.
- **An agent-create request naming a preset the operator never configured.**
  400, and nothing is created — the terminal host is never called.
- **A destination whose folder cannot be resolved.** Opening a shell there
  still succeeds (the host computes its own anchor). Starting an agent there
  is refused with 409.
- **The terminal host unreachable for any other reason.** 502 with the
  underlying message, on every route that talks to it.
- **A workspace with 2+ plain-shell panes and zero agents.** Each pane gets
  its own shell row; they are never merged into one row per workspace
  (per hsw-D1).

## Open Gaps

- The screen read/reply/keys endpoints (`GET /api/panes/<id>/screen`,
  `POST /api/panes/<id>/input`, `POST /api/panes/<id>/keys`) predate this
  spec and are recorded here only at the entry-point level. Answered by: a
  harvest pass over the terminal detail flow (mirrors the same gap already
  recorded in `herdr-port.md`).
- The login token's own lifecycle (rotation, where it is stored, how the
  operator changes it) is `config`/`doctor` territory, referenced but not
  specced here. Answered by: `installation.md` or a `doctor` harvest pass.

## Visuals

Not applicable — this spec describes the HTTP surface, not a screen. The
switcher screen that consumes `GET /api/agents` (agent rows and shell rows
alike) is specced in `switcher.md`. `GET /api/create-options` and the two
create routes are consumed by the create sheet, specced in `create-sheet.md`.

## Pointers (implementation)

- `src/web/mod.rs` — `AppState` (including the operator's `agent_presets`,
  attached via `AppState::with_agent_presets` rather than a constructor
  parameter, so the pre-existing call sites keep compiling), the route table.
- `src/web/auth.rs` — `AuthSession`, `silent_404`, login/logout handlers.
- `src/web/api.rs` — `agents`, `create_options`, `health` handlers and their
  response types (`AgentRow`, `ShellRow`, `AgentsResponse`, `Destination`,
  `PresetOption`).
- `src/web/create.rs` — `create_pane`, `create_agent`, and the shared
  `herdr_error_response` mapping described above.
- `src/web/screen.rs` — the observe/reply surface (Open Gaps).
- `docs/specs/herdr-port.md` — what this area's handlers actually ask the
  terminal host, and why the two create routes diverge on an unresolved
  folder (R17).

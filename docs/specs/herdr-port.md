---
area: herdr-port
updated: 2026-07-21
sources: [new-shell-new-agent, terminal-workspace-org, web-create-endpoints]
decisions: [D5, D6, D7, D10, D11, P2, P10, 32e1c056, cbb74712, e511daeb]
coverage: partial
---

# Spec: herdr port (talking to the terminal host)

The gateway does not own terminals. A separate desktop application — the terminal
host — runs the Operator's shells and agents, and the gateway is a client of it.
This area covers everything the gateway asks the host and how it interprets the
answers: what a snapshot contains, how workspaces, tabs and panes relate, which
folder a workspace is "in", which pane's folder governs anything created there,
and — since the create operations landed — how the gateway asks the host to open
a shell or start an agent, and how a refusal is understood. Every screen the
Operator sees on their phone is a rendering of what this area read.

## Entry Points & Triggers

- The switcher screen loading or refreshing → a full snapshot is requested.
- The notification watcher's poll interval elapsing → a full snapshot is requested.
- Opening a terminal, and every screen refresh while it is open → that pane's
  visible screen is requested.
- The Operator sending text or a navigation key from a terminal screen → that
  input is delivered to the pane.
- Startup and the diagnostic command → the host is asked to identify itself and
  its protocol generation.
- The Operator asking for a new shell in a chosen workspace → a tab is created
  there.
- The Operator asking to start an agent in a chosen workspace → an agent is
  started there under a generated name.

## Data Dictionary

The snapshot is one envelope describing everything the host currently has alive.
Five collections plus three pointers.

| # | Element | Meaning | Values | Required | Default |
|---|---|---|---|---|---|
| 1 | Workspaces | The Operator's top-level groupings. In practice one workspace is one project the Operator is working on. | list | yes | — |
| 2 | — workspace id | Opaque handle issued by the host | text | yes | — |
| 3 | — label | Display name. The host derives it from the workspace's live folder unless the Operator renamed it, so it is normally the project's folder name — but it is a display name, never a path. | text | yes | — |
| 4 | — active tab | Which of the workspace's tabs is the current one **within that workspace**, independent of what the Operator is looking at right now | tab id | yes | — |
| 5 | — agent status | Rolled-up readiness of the agents inside | same values as an agent's status | yes | — |
| 6 | Tabs | Subdivisions of a workspace | list | yes | — |
| 7 | Panes | Every live terminal, whether or not an agent is attached. A plain shell appears here and nowhere else. | list | yes | — |
| 8 | — pane id | Opaque handle issued by the host | text | yes | — |
| 9 | — folder | The directory the pane's shell is currently in. The host reads it live, so it follows the Operator typing a directory change; it is not the directory the pane was opened in. | path, may be absent | no | — |
| 10 | — foreground folder | The directory of whatever process currently holds the pane, which can differ from the shell's own when a running command moved itself. Absent on platforms where the host cannot resolve it. | path, may be absent | no | — |
| 11 | Agents | The subset of panes with a named agent attached | list | yes | — |
| 11a | — agent name | The name the agent runs under. Unique across the host at any moment: starting a second agent under a name already in use is refused. Absent on hosts or records that never set one. | text, may be blank | no | blank |
| 12 | Layouts | One entry per tab, for every tab of every workspace — including tabs the Operator is not currently looking at. Each entry records which pane is focused **within that tab**. | list | yes | — |
| 13 | — focused pane | The pane focused inside this tab. Absent when the host cannot publish an id for it. | pane id, may be absent | no | — |
| 14 | Globally focused workspace / tab / pane (not shown) | What the Operator is looking at **right now** on the desktop. Describes exactly one workspace — it says nothing about any other. | ids, may be absent | no | — |
| — | Protocol generation (not shown) | The host's contract version, matched exactly rather than by range | number | yes | — |

## Behaviors & Operations

### Read a snapshot

- **Triggers:** any screen load or refresh, and the watcher's poll.
- **Blocked when:** the host is unreachable, or its protocol generation differs
  from the one this gateway was built against (per R6).
- **What changes:** nothing on the host — this is a read.
- **Side effects:** none.
- **On a malformed answer:** an envelope missing its panes, layouts, or agents is
  rejected whole, because the host always sends all three and their absence means
  a host that cannot be trusted rather than an empty desk (per R5). A missing
  workspaces or tabs collection is tolerated as empty instead, so a partial answer
  still yields a usable list.
- **Afterwards:** the Operator sees the current agents grouped by workspace; where
  a label or status cannot be resolved by joining ids, the Operator sees a blank
  label or an unknown status rather than an error or a crash (per R7).

### Resolve a workspace's folder for something new

Anything the Operator creates inside a workspace — a shell, an agent — starts in
a folder. This operation decides which one.

- **Triggers:** a request to create something in a named workspace.
- **What it reads:** that workspace's own active tab, then the layout entry for
  exactly that workspace **and** that tab, then that entry's focused pane, then
  that pane's folder — preferring the foreground folder when it exists (per R2,
  R4).
- **Blocked when:** nothing; every step that cannot resolve yields no folder
  rather than a wrong one (per R3).
- **Side effects:** none — this is a read.
- **Afterwards:** the caller has either the exact folder the terminal host itself
  would have chosen for a new terminal in that workspace, or nothing. There is no
  third outcome and no guess. When a folder is found, the caller also learns
  whether it is the pane's current live directory or only its process start
  directory — the same distinction R4 already governs, now exposed to callers
  that need to say so out loud rather than silently trust it (per P2).

### Open a shell in a workspace

- **Triggers:** the Operator asking for a new shell in a named workspace.
- **What it sends:** the workspace, the folder resolved above when one exists,
  and an explicit instruction not to move the desktop's focus (per R9, R10).
  When no folder resolves, the folder is **omitted** rather than substituted —
  the host then computes its own anchor for the new shell, the same
  computation it would run for a new terminal opened at the desktop itself
  (per P10, R17).
- **Blocked when:** the workspace is not known to the host.
- **What changes:** the host gains a tab holding one new shell pane.
- **Afterwards:** the caller holds the new tab's id and its pane's id, and can
  read that pane immediately — a pane that was just created is never reported as
  missing (per R12).

### Start an agent in a workspace

- **Triggers:** the Operator asking to start an agent in a named workspace.
- **What it sends:** a generated name, the command to run, the resolved folder,
  the workspace, and an explicit instruction not to move the desktop's focus. It
  deliberately does not name a tab or a split direction, so the host's own
  placement applies and a placement conflict cannot arise (per R11). Unlike
  opening a shell, the folder is never omitted here just because resolution
  came up empty — this operation's own fallback for a missing folder is the
  **host's own process directory**, unrelated to any workspace, so a caller
  with no resolved folder must refuse before ever reaching this operation (per
  P10, R17).
- **Blocked when:** the workspace is not known; the command to run is empty; or
  the workspace has no active tab, in which case there is nowhere to place the
  agent and nothing is created.
- **On a name collision:** a new name is generated and the attempt is repeated,
  up to five times, without the caller ever learning it happened. Exhausting the
  five is reported as a failure rather than retried further (per R13).
- **What changes:** the host gains an agent pane inside the workspace's active
  tab. No new tab is created.
- **Afterwards:** the caller holds the new pane's id and the name the agent
  actually ended up with, and can read that pane immediately. The agent is
  running, which is not the same as ready — readiness is only ever learned from a
  later snapshot (per R14).

### Read a pane's screen · deliver input to a pane

- **Triggers:** the terminal screen opening or refreshing; the Operator sending
  text or a navigation key.
- **Afterwards:** the Operator sees the pane's current visible screen, or is told
  the pane is gone when the host no longer knows it.
- Detailed behavior of these two operations predates this spec — see Open Gaps.

## Actors & Access

| Capability | Operator (via the phone) | Gateway | Terminal host (system) |
|---|---|---|---|
| Create workspaces | — | — | ✓ |
| Report what is alive | — | — | ✓ |
| Read the snapshot | — | ✓ | — |
| Read a pane's screen | ✓ (via the gateway) | ✓ | — |
| Send text and keys to a pane | ✓ (via the gateway) | ✓ | — |
| Open a shell in an existing workspace | ✓ (via the gateway) | ✓ | ✓ |
| Start an agent in an existing workspace | ✓ (via the gateway) | ✓ | ✓ |
| Rename or close anything | — | — | ✓ |

The gateway reads, types, and adds. It never renames, moves, or closes anything,
and it never creates a workspace — it can only add inside one the Operator
already has. Everything it knows, it learned from a snapshot.

## Business Rules

- **R1.** Every id is opaque and is read fresh from the current snapshot — never
  constructed, never remembered between reads.
- **R2.** The folder that seeds anything created in a workspace comes from that
  workspace's **own** active tab and that tab's **own** focused pane. The
  globally focused workspace, tab, and pane describe only the one thing the
  Operator is looking at, and using them would leave every other workspace — the
  ordinary case — with no folder at all (per D10).
- **R3.** A folder that cannot be resolved yields nothing. No step in the
  resolution substitutes a different pane, a parent folder, or a default (per D10).
- **R4.** Where a pane reports both a shell folder and a foreground folder, the
  foreground one governs — it is what the terminal host's own new-terminal
  behavior follows (per D5).
- **R5.** Panes, layouts, and agents are mandatory in a snapshot; their absence is
  a broken answer, not an empty one. Workspaces and tabs are tolerated as empty
  (per 32e1c056).
- **R6.** The host's protocol generation is matched exactly. A newer or older host
  is refused rather than partially trusted.
- **R7.** Joining ids across collections never fails loudly: an unresolvable label
  reads as blank and an unresolvable status as unknown.
- **R8.** A workspace's label and its resolved folder are computed by the host from
  **different** panes — the label from the workspace's first tab's root pane, the
  folder from the active tab's focused pane. Once the Operator changes directory in
  one of them the two legitimately disagree, and no reconciliation is correct.
  Where both are shown, the folder is the one that governs (per D11).

- **R9.** Anything created is given its folder explicitly, computed by the rules
  above. The host has its own policy for choosing a folder when none is named,
  but that policy answers "where is the desktop right now", which is meaningless
  for a request arriving from a phone (per D5).
- **R10.** Creating from the phone never moves the desktop's focus. The Operator
  may be sitting in front of the desktop working on something else; a phone
  action must not pull them elsewhere (per D6).
- **R11.** An agent is placed by naming only its workspace. Naming both a
  workspace and a tab creates a class of conflict the phone gains nothing from,
  and the phone has no concept of split direction, so the host's own placement is
  accepted.
- **R12.** A pane that was just created is immediately readable. Creating
  something and then being told it does not exist is never a valid outcome.
- **R13.** Agent names are generated, not chosen by the Operator, and a collision
  is resolved by generating another and retrying — invisibly, up to five times.
  The bound exists because a sixth consecutive collision means the generator
  itself is broken, and retrying harder would hide that (per D7, e511daeb).
- **R14.** A successful start means the agent's pane exists, never that the agent
  has finished starting. Nothing in this area reports readiness; only a later
  snapshot does.
- **R15.** A refusal from the host is understood, not flattened. The host names a
  reason for every refusal, and that name is preserved. Three reasons are
  distinguished by type because a caller acts on them — a name already in use, a
  workspace that no longer exists, and a command that is empty; every other
  reason keeps its name alongside its explanation. A refusal is never reported as
  a broken answer, and the host's own explanation always reaches the Operator
  even when the gateway has no special handling for that reason (per cbb74712).
- **R16.** Failing to reach the host and being refused by the host are different
  outcomes and are never conflated.
- **R17.** Opening a shell and starting an agent do not fall back alike when no
  folder resolves. The shell verb may omit the folder and let the host resolve
  its own workspace anchor — the same computation the desktop performs for a
  new terminal. The agent verb's own fallback for an omitted folder is the
  **host process's own directory**, unrelated to any workspace, so nothing in
  this codebase omits it there — a caller with no resolved folder refuses
  before this verb is ever called (per P10).

## Edge Cases Settled

- **A workspace the Operator is not looking at.** Resolves normally. This is the
  ordinary case, not the exception — verified against a live host where four of
  five workspaces were unfocused and all four resolved.
- **A workspace whose folder belongs to a plain shell.** Resolves normally. The
  anchor pane frequently has no agent attached, which is why panes cannot be
  approximated by the agent list.
- **A tab whose focused pane has no publishable id.** The host omits that tab's
  layout entry entirely; resolution yields nothing rather than falling through to
  another pane.
- **A workspace whose active tab names a tab no layout describes.** Possible
  because the host can report an active tab derived by position while real tab
  numbers come from a counter that never reissues a closed tab's number.
  Resolution yields nothing.
- **A pane reporting neither folder.** Yields nothing.
- **A workspace whose folder cannot be resolved, opening a shell there.** Never
  blocked. The folder is omitted rather than guessed; the host computes its
  own anchor for the new shell (per P10, R17).
- **Starting an agent in a workspace with no active tab.** There is no pane to
  place it beside, so the attempt is refused and nothing is created — no agent,
  no pane. Inventing a tab to hold it would leave a pane pointing at a tab that
  does not exist, a shape the host itself cannot produce.
- **An agent that has just been started.** It has no work in progress, which is
  what "idle" means. It is not "unknown" — that word is reserved for a status the
  host reported and the gateway failed to recognise, and applying it to a healthy
  new agent would state something false.
- **A refusal carrying no reason.** Still a refusal, reported with the host's
  explanation and an empty reason — never reclassified as a broken answer, which
  would replace the one sentence the Operator could act on.
- **Two workspaces with the same label and the same folder.** Legitimate and
  observed live: the Operator can have two workspaces open on one project. They
  are distinct workspaces with distinct ids. Anything that shows a chooser must
  therefore distinguish them by something other than label and folder — see Open
  Gaps.

## Open Gaps

- On Windows the foreground folder is never reported, and a pane's shell folder is
  only correct while the host's shell integration is active — the platform's own
  shell does not update its process directory on a directory change. What a
  destination chooser should show there is undecided. Answered by: a decision
  before any screen displays a folder to the Operator.
- Two workspaces can share a label and a folder; nothing yet distinguishes them to
  a person. The snapshot carries a per-workspace number that is a candidate.
  Answered by: the slice that builds a destination chooser.
- The screen-read and input-delivery operations predate this spec and are recorded
  here only at the trigger level. Answered by: a harvest pass over the terminal
  detail flow.

## Visuals

Not applicable — no screen. The screens that render this data are specced in
`switcher.md` and `terminal-detail.md`.

## Pointers (implementation)

- `src/herdr/mod.rs` — the `Herdr` trait: the application-facing contract both the
  real client and the test substitute implement; `HerdrError` implementing R15/R16;
  the bounded name-collision retry implementing R13; `tab_create`/`agent_start`
  take the folder as optional, implementing the asymmetric omit behavior in R17.
- `src/herdr/wire.rs` — `Snapshot` and its member types; the id-join helpers
  (`workspace_label_for`, `tab_label_for`, `workspace_status_for`) implementing R7;
  `Snapshot::anchor_for_workspace` implementing R2/R3/R4/P2 (folder plus
  whether it is the live directory); `Snapshot::anchor_cwd_for_workspace`
  delegates to it, discarding the flag, for callers that only need the folder.
- `src/herdr/socket.rs` — `SocketHerdr`, the live client; `parse_snapshot` is the
  single extraction path implementing R5.
- `src/herdr/fake.rs` — `FakeHerdr`, the substitute the whole application is
  demo-run and end-to-end tested against.
- `src/herdr/testdata/live-snapshot.json` — a real captured envelope, with
  `expected-anchors.json` recording the correct resolution for each of its
  workspaces.
- `upstreams/herdr/` — vendored copy of the terminal host's own source, and
  `.bee/spikes/pbi-001-events-subscribe/schema.json` its captured protocol schema.

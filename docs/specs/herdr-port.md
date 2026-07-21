---
area: herdr-port
updated: 2026-07-21
sources: [new-shell-new-agent, terminal-workspace-org]
decisions: [D5, D10, D11, 32e1c056]
coverage: partial
---

# Spec: herdr port (reading the terminal host)

The gateway does not own terminals. A separate desktop application — the terminal
host — runs the Operator's shells and agents, and the gateway is a client of it.
This area covers everything the gateway asks the host and how it interprets the
answers: what a snapshot contains, how workspaces, tabs and panes relate, which
folder a workspace is "in", and which pane's folder governs anything created
there. Every screen the Operator sees on their phone is a rendering of what this
area read.

## Entry Points & Triggers

- The switcher screen loading or refreshing → a full snapshot is requested.
- The notification watcher's poll interval elapsing → a full snapshot is requested.
- Opening a terminal, and every screen refresh while it is open → that pane's
  visible screen is requested.
- The Operator sending text or a navigation key from a terminal screen → that
  input is delivered to the pane.
- Startup and the diagnostic command → the host is asked to identify itself and
  its protocol generation.

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
  third outcome and no guess.

### Read a pane's screen · deliver input to a pane

- **Triggers:** the terminal screen opening or refreshing; the Operator sending
  text or a navigation key.
- **Afterwards:** the Operator sees the pane's current visible screen, or is told
  the pane is gone when the host no longer knows it.
- Detailed behavior of these two operations predates this spec — see Open Gaps.

## Actors & Access

| Capability | Operator (via the phone) | Gateway | Terminal host (system) |
|---|---|---|---|
| Create workspaces, tabs, panes | — | — | ✓ |
| Report what is alive | — | — | ✓ |
| Read the snapshot | — | ✓ | — |
| Read a pane's screen | ✓ (via the gateway) | ✓ | — |
| Send text and keys to a pane | ✓ (via the gateway) | ✓ | — |
| Rename or close anything | — | — | ✓ |

The gateway reads and types; it never renames, moves, or closes. Everything it
knows, it learned from a snapshot.

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
  real client and the test substitute implement.
- `src/herdr/wire.rs` — `Snapshot` and its member types; the id-join helpers
  (`workspace_label_for`, `tab_label_for`, `workspace_status_for`) implementing R7;
  `Snapshot::anchor_cwd_for_workspace` implementing R2/R3/R4.
- `src/herdr/socket.rs` — `SocketHerdr`, the live client; `parse_snapshot` is the
  single extraction path implementing R5.
- `src/herdr/fake.rs` — `FakeHerdr`, the substitute the whole application is
  demo-run and end-to-end tested against.
- `src/herdr/testdata/live-snapshot.json` — a real captured envelope, with
  `expected-anchors.json` recording the correct resolution for each of its
  workspaces.
- `upstreams/herdr/` — vendored copy of the terminal host's own source, and
  `.bee/spikes/pbi-001-events-subscribe/schema.json` its captured protocol schema.

# Brief — planning slices 3, 4 and 5

For a second session. Slices 1 and 2 have shipped; this is everything that
session needs to plan the rest without re-deriving it.

## Start here, mechanically

Run in this checkout, **in your own lane** — another session works the same
worktree:

```
node .bee/bin/bee.mjs state start-feature --feature new-shell-slices-3-5 --mode standard --phase planning --as-lane --session-id <your session id> --paths docs/history/new-shell-new-agent
```

Planning writes only `docs/` and `.bee/`, so a lane is enough separation; do not
touch `src/` from the planning session. The frozen `plan.md` maps all five
slices — **do not reopen it**. Cells for slices 3-5 do not exist yet and must not
be created until their slice is current (`plan.md:53,113`).

## What already shipped

| slice | what landed | commits |
|---|---|---|
| 1 | The snapshot's panes, layouts, per-workspace active tab and folder fields; the anchor-folder join proven against a real captured envelope | `f8eacfb`, `0caccf7` |
| 2 | `tab_create` and `agent_start` on the herdr port; typed error variants for the three refusals a caller acts on; `Agent.name` parsed | `80b391c`, `3a4b55c`, `e471deb` |

Read `docs/specs/herdr-port.md` before anything else — it is current as of slice
2 and states the create rules (R9-R16) in product language. 201 tests green.

**Slice 2 answered slice 4's dependency:** `HerdrError` has typed variants for
`AgentNameTaken`, `WorkspaceNotFound` and `InvalidAgentArgv`, plus
`Remote { code, message }` carrying every other herdr code verbatim. Slice 4's
HTTP mapping extends `src/web/screen.rs`'s existing 3-way match onto these.
The name-collision retry is **already handled inside the port** — slice 4 must
not reimplement it and must not surface it.

## Locked decisions the UI slices cannot renegotiate

From `docs/history/new-shell-new-agent/CONTEXT.md`:

- **D1** — one floating action button, bottom-right of Home, the single entry
  point for both new shell and new agent. No `+` in the header, none in workspace
  section headers.
- **D2** — one bottom sheet: destination selector on top, action rows below,
  `Shell` first then one row per configured agent preset. Not a two-step "pick a
  type, then fill a form" flow.
- **D3** — the destination is a single choice combining workspace and folder,
  shown as one row (label + path). Never two independent pickers.
- **D4** — the agent preset list (label + argv) lives in the gateway's
  `config.json` and is edited through `doctor`. The mobile UI only renders it.
- **D6** — after a successful create the phone navigates itself into the new
  pane's terminal detail. Two taps to a shell, two to an agent.
- **D7** — the agent name is auto-generated; the user never types it. There is no
  name field in the sheet.
- **D9** — the destination selector reserves a slot for a future "Other project…"
  entry, but ships without it.
- **D11** — where the workspace label and the resolved folder disagree, the
  **path wins**: it is what the sheet shows and what goes on the wire.

**The bottom sheet primitive already exists** — `web/src/views/terminal.ts:49-80`
(`.reply-sheet`, `.keys-pad`) with slide-up animation, safe-area insets and a
shared `.sheet-head`/`.sheet-x` header, styled at `web/src/styles.css:819-946`.
Slice 5 reuses it. The floating action button is the only new primitive.

## Settled today: the shell card on Home

Decisions `c8ccfda6` and `c2aab6b9`. Home will list shell panes, not only agents
(the current list drops any workspace whose panes are all plain shells — see
PBI-024 and the research already answered in `handoff-next-session.md`).

A shell card is:

- a rounded-border rectangle with **no elevated grey fill** — it sits on the page
  background while agent cards sit on an elevated surface. That contrast is what
  separates the two species at a glance;
- a shell icon on the left;
- line 1: the pane's folder, abbreviated to `~/…` when under the home folder,
  full path otherwise;
- line 2: `shell · {tab label}`, mirroring the agent card's `{kind} · {tab label}`;
- **no status badge and no status colour anywhere.**

**The shell program is not knowable and must not be claimed.** Verified in four
places: our `Pane` type has no such field; herdr's captured protocol schema gives
`PaneInfo` 19 fields with no `shell`/`program`/`argv`/`command`; herdr resolves a
shell at spawn time (`upstreams/herdr/src/pane.rs:1206-1226`) but never attaches
it to any serialized pane; and a real captured plain-shell pane carries only
`terminal_title`, a free-text prompt string. Inferring `zsh` vs `bash` from that
prompt was explicitly rejected — right most of the time, silently wrong
sometimes. Asking herdr upstream for a real field stays open as a later option.

Two knock-on items the design brief
(`plans/reports/design-brief-260721-1153-shell-card-on-home-screen.md`) also
covers and that slice 5 must answer: what the **workspace section header** shows
when a workspace has no agents to roll up, and how two workspaces sharing a label
and a folder are told apart. Both become visible only once shells are listed.

## Open questions each slice must answer

**Slice 3 — presets in config + doctor**
- Is an empty `argv` a config-load failure, or a preset that renders disabled?
  (`plan.md:88`.) Slice 2 made `InvalidAgentArgv` reachable, so either is now
  implementable; pick one deliberately.
- `agent_presets` must go through all five `config/mod.rs` sites — check what
  those are before estimating.

**Slice 4 — web endpoints**
- **Windows.** This is where the anchor path first becomes something a person
  reads. `foreground_cwd` is unix-only and the platform's own shell does not
  update its process directory on a directory change, so a Windows destination
  row can silently show the wrong folder. Undecided, and it must not be decided
  by accident (`CONTEXT.md:155-179`).
- Where is the destination list assembled — backend or frontend? Unresolved
  (`CONTEXT.md:177-179`); it decides the slice 4 / slice 5 boundary.
- Unauthenticated requests must get the same opaque 404 as every other route.

**Slice 5 — the FAB and the sheet**
- The shell-card work above lands here (or as its own small feature — decide
  with the user; PBI-024 is filed as `proposed`).
- Disambiguating two workspaces with an identical label and path.

## Standing hazards, all filed, none blocking

- The socket call loop has **no read timeout**; create calls spawn processes
  upstream and are the slowest requests this client makes. A test already wedged
  the suite for 10 minutes on this. Any test touching a real socket must be
  wrapped in a timeout.
- `config::secrets` tests race on process-global env vars — the full verify can
  fail for reasons unrelated to your change. Re-run before investigating, but do
  not normalise the habit.
- Three guards in `tests/rename_contract.sh` are `rg`-gated and silently no-op on
  this machine while reporting ok.

## The review discipline that caught four defects in slice 2

All four passed a green worker verify. Read
`docs/history/learnings/critical-patterns.md` — the short version: **diff the
fake's answers against the live client's**, do not read each in isolation. A fake
that is kinder than production makes the whole suite blind.

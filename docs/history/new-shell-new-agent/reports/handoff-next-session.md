# Handoff — next session

**Feature:** `new-shell-new-agent` · **Lane:** high-risk · **Phase:** `compounding-complete`
**Last commits:** `f8eacfb`, `0caccf7` (slice 1 cells), `b0d6c5b` (docs, spec, learnings)

Slice 1 of 5 is done, scribed, and compounded. Read
`docs/history/new-shell-new-agent/CONTEXT.md` (D1–D11 locked) and
`docs/specs/herdr-port.md` before touching anything.

---

## TASK 0 — The home list omits agentless workspaces

**Research is DONE** (decision `b9ed0723`, backlog PBI-024). What remains is one product
decision and then implementation. Read this before planning slice 2 — the answer
touches slice 4's API shape and slice 5's sheet.

### Answered — do not re-investigate

- **No new herdr call is needed.** `session.snapshot` already returns `panes[]`
  alongside `agents[]`, using the identical `PaneInfo` schema that `pane.list` would
  return (`.bee/spikes/pbi-001-events-subscribe/schema.json:9086-9091` vs `:8332-8349`).
  `pane.list` exists and filters only by `workspace_id` — it buys nothing here.
- **The gateway already parses it.** `Snapshot.panes` landed in slice 1
  (`src/herdr/wire.rs`). `GET /api/agents` (`src/web/api.rs:29-55`) simply iterates
  `snap.agents` while `snap.panes` sits in the same value from the same round trip.
- **Why shells vanish:** herdr's `agent.list` filters on `Terminal::is_agent_terminal`
  (`upstreams/herdr/src/terminal/state.rs:1333-1337`) — true only with an agent name, a
  detected agent label, or launch argv. A plain shell has none, so it is dropped from
  `agents[]` while remaining in `panes[]`.
- **herdr itself does not filter.** `render_workspace_list`
  (`upstreams/herdr/src/ui/sidebar.rs:1069-1108`) renders every workspace
  unconditionally. The gateway is currently narrower than the tool it fronts, which
  makes this a defect, not a design choice.
- **What a pane-centric list loses:** only `name` and `screen_detection_skipped`, both
  `AgentInfo`-only and both minor. `PaneInfo` conversely adds `label` and `scroll`.
- **Implementation shape:** iterate `snap.panes`, and widen `wire::Pane` (today only
  `pane_id, workspace_id, tab_id, cwd, foreground_cwd`) to also parse `agent`,
  `agent_status`, `display_agent`, `title` — all already in the payload. Remember the
  three-population-sites rule: `wire.rs`, `socket.rs`, `fake.rs`. Check
  `Snapshot::display_for` (used at `src/web/api.rs:46`) before reproducing today's
  `AgentRow.display`/`kind`/`status` semantics for a shell.

### The one open question — product, deferred by the user

**What does a shell pane look like on home?** A shell carries no `agent`,
`agent_session`, `display_agent`, `title` or `label` (the keys are omitted entirely,
not null) and its `agent_status` is `"unknown"`. But `docs/specs/switcher.md` defines
`unknown` as "herdr reported a value this app doesn't recognize" — so listing shells
as-is would state something false. A shell is not an unrecognised state; it is the
absence of an agent.

Three directions, not yet chosen:
1. A distinct `shell` badge instead of borrowing the agent status scale.
2. A visibly different card — a shell has nothing to monitor, so a status-shaped card
   misleads.
3. Leave home agent-only and surface agentless workspaces **only** in the Add New
   destination dropdown. This fixes the Add New gap — the actual harm — without
   changing what the home screen is.

Direction 3 is the smallest honest fix and is worth putting to the user first. The
switcher spec would need updating for 1 or 2, since it currently describes a list of
*agents*.

### Original report and evidence

Users report that home lists only workspaces that have a running agent. A workspace
whose panes are all plain shells never appears.

### What is already established — do not re-derive this

The tracked live capture committed in slice 1 confirms the report, and shows it is
worse than reported. From `src/herdr/testdata/live-snapshot.json` (real herdr 0.7.4,
protocol 16):

| workspace | label | panes | agents | shells |
|---|---|---|---|---|
| w3 | fgos-dev | 1 | 1 | 0 |
| w5 | forgent | 3 | 3 | 0 |
| w7 | herdr-gateway | 2 | 2 | 0 |
| w8 | design-extract | 1 | 1 | 0 |
| **wB** | **forgent** | **1** | **0** | **1** |

`wB` has no agent at all — and `focused_workspace_id` in that same capture is `wB`.
**The workspace the operator was actually sitting in is the one missing from home.**

Structurally: the gateway's list surface is built from the snapshot's `agents[]`
(`src/web/api.rs`, `AgentRow`), and the frontend groups those rows by workspace
(`groupByWorkspace`, `web/src/views/switcher.ts:44-60`). A workspace contributes rows
only through its agents, so zero agents means zero rows means invisible. Slice 1
already established that `panes[]` is a strict superset of `agents[]` and now parses
it — see `docs/specs/herdr-port.md` Data Dictionary rows 7 and 11.

### Knock-on effect worth knowing

**The duplicate-label problem changes shape.** `w5` and `wB` share the label `forgent`
and the same folder. Today only `w5` is visible, so the ambiguity is hidden. Fixing the
listing surfaces both and makes the disambiguator (decision `ab62f6e9`,
`WorkspaceInfo.number` is the candidate) load-bearing rather than theoretical.

### Where this work belongs

Decide with the user: its own small feature, folded into slice 5, or — if direction 3
wins — absorbed into slice 4's destination-list endpoint, where it is close to free.
PBI-024 is filed as `proposed`.

---

## TASK 1 — Plan slice 2: create verbs on the herdr port

Route through `bee-planning`. Gate 3 covered slice 1 only, so slice 2 needs planning →
validating → its own Gate 3. The frozen `plan.md` maps all five slices; do not reopen it.

**Scope:** `tab.create` and `agent.start` on the `Herdr` trait, `SocketHerdr`, and
`FakeHerdr`, plus the richer `HerdrError` variants the create error codes need.
Files: `src/herdr/mod.rs`, `src/herdr/socket.rs`, `src/herdr/fake.rs`.

**Decide early, because slice 4 depends on it:** does `HerdrError` grow one variant per
herdr error code, or one `Request { code, message }` carrying the code? The codes that
matter are `agent_name_taken`, `invalid_agent_argv`, `workspace_not_found`,
`agent_placement_not_found`, `agent_placement_conflict`, `agent_start_failed`,
`tab_create_failed` (`upstreams/herdr/src/app/agents.rs:208-253`,
`upstreams/herdr/src/app/api/tabs.rs:57,63,126`).

**Copy, do not reinvent:** the socket call shape at `src/herdr/socket.rs` (`send_input`
over the private `call` helper), and the now-extracted `parse_snapshot` seam as the
model for testing a live path without I/O.

---

## Slices 3–5 (not planned)

| # | Slice | Open question it must answer |
|---|---|---|
| 3 | `agent_presets` in config + a doctor editor | Is an empty `argv` a config-load failure, or a preset that renders disabled? |
| 4 | Destination list + the two create endpoints | **Windows:** `foreground_cwd` is unix-only and PowerShell's `cwd` needs shell integration. Must not silently show a wrong folder. |
| 5 | The FAB and the bottom sheet | Disambiguating two workspaces with identical label and path |

---

## Repo state the next session should know

**Two P1 verify-integrity items are open** (`.bee/backlog.jsonl`, filed this session).
Neither blocks this feature, both undermine trust in recorded evidence:

- `tests/rename_contract.sh:30,37,38` — three guards built on `rg`, which has no binary
  reachable from a non-interactive subprocess here. They silently no-op while the
  script reports `ok`, and the script runs inside `commands.verify`. Fix is `grep -E`;
  the previously recorded conclusion that this was unfixable accepted the wrong
  constraint.
- `.bee/cells/windows-support-5.json` — capped with `verify_passed: true`, but its
  verify re-run in a real shell exits 127. Two of its five `rg` calls are negated, so a
  missing tool inverts to success. The recorded evidence is not reproducible.

**Changed this session:** `.bee/config.json` `commands.verify` now runs
`cargo clippy --all-targets`, matching every CI job. Verified still green.

**Uncommitted:** `.bee/backlog.jsonl` and `.bee/state.json` could not be staged — the
bee write-guard blocks `git add` on CLI-owned files. The four friction rows exist in
the file but are not committed. A pile of unrelated `.bee/bin/**` and
`.agents/skills/**` modifications predate this session and were deliberately left alone.

**The feature is unreviewed.** Registered as a review candidate at `b0d6c5b`
(`new-shell-new-agent`, high-risk, 2 cells). That is the normal path, not a skipped
step — independent review runs only when the user asks for it.

## Process notes worth carrying

- Run a cell's `verify` before dispatching and require it to **fail**. A verify that is
  green on an untouched tree cannot tell done from not-started. Full rules:
  `docs/history/learnings/20260721-verify-commands-that-cannot-fail.md`.
- Pass `--lane <feature>` on `state`/`gate` calls whenever another session may be live,
  and treat "the next free PBI number" as a race, not a read. Both collided today.
- Execution workers cannot be dispatched as `bee-gather`/`bee-extract`/`bee-review` —
  those rendered agent types are read-only. Use a bare `model` param with the default
  subagent type and omit the tier marker.

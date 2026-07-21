# Handoff — next session

**Feature:** `new-shell-new-agent` · **Lane:** high-risk · **Phase:** `compounding-complete`
**Last commits:** `f8eacfb`, `0caccf7` (slice 1 cells), `b0d6c5b` (docs, spec, learnings)

Slice 1 of 5 is done, scribed, and compounded. Read
`docs/history/new-shell-new-agent/CONTEXT.md` (D1–D11 locked) and
`docs/specs/herdr-port.md` before touching anything.

---

## TASK 0 — Research first: the home list omits agentless workspaces

**Do this before planning slice 2.** It is investigation, not implementation, and its
answer may change slice 5's design and possibly slice 4's API shape.

### The report

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

### What to find out

1. **Confirm the cause end to end** rather than by inference: trace `/api/agents`
   from `src/web/api.rs` through `src/web/mod.rs` to the frontend, and state plainly
   whether any layer filters, or whether the list is simply agent-shaped from the
   start. Slice 1 parsed `panes[]` into `Snapshot` but did not change any list
   surface — confirm that is still true.
2. **Does herdr itself distinguish?** It has both `agent.list` and pane-level calls
   (`upstreams/herdr/docs/next/website/src/content/docs/socket-api.mdx:99-112`, and
   the captured schema at `.bee/spikes/pbi-001-events-subscribe/schema.json`). Report
   what herdr's own desktop UI shows — does it list agentless workspaces? A pane
   carries `agent`, `agent_session`, `agent_status`, and `display_agent`; find out
   what each means for a plain shell and whether "is an agent" is a real boolean or a
   soft, display-level notion.
3. **Does the distinction matter to what we are building?** Two ways it might:
   - **The Add New button loses its best case.** The destination dropdown (D3) is
     built from workspaces. If agentless workspaces are absent, the operator cannot
     start an agent in the project they are currently working in — exactly `wB` above,
     and exactly the moment "new agent here" is most wanted.
     Check whether the destination list will inherit the same agent-shaped source or
     can be built from `workspaces[]` + `panes[]` directly.
   - **The duplicate-label problem changes shape.** `w5` and `wB` share the label
     `forgent` and the same folder. Today only `w5` is visible, so the ambiguity is
     hidden. Fixing the listing surfaces both and makes the disambiguator
     (decision `ab62f6e9`, `WorkspaceInfo.number` is the candidate) load-bearing
     rather than theoretical.
4. **Is a shell-only pane worth showing on home at all, and as what?** This is a
   product question, not a technical one — bring options to the user rather than
   deciding. The switcher spec (`docs/specs/switcher.md`) currently describes a list
   of *agents*; showing shells changes what that screen is.

### Deliverable

A short findings write-up plus a recommendation on whether this becomes its own
feature, folds into slice 5, or is a tiny fix. If it turns out to be a real product
gap, file a PBI row (check `docs/backlog.md` for the next free number — PBI ids have
already collided once between concurrent sessions).

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

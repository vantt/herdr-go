# New Shell / New Agent — Context

**Feature slug:** new-shell-new-agent
**Date:** 2026-07-20
**Exploring session:** complete
**Scope:** Deep
**Domain types:** SEE, CALL, ORGANIZE

## Feature Boundary

From the phone, create two kinds of thing inside a project that is already open on the
desktop: a plain shell, and a named agent. One FAB on the switcher opens one sheet that
names the destination folder and offers Shell plus one row per configured agent preset.
It ends at the two `herdr` create calls (`tab.create`, `agent.start`) and the navigation
into the new pane. It does not create workspaces, does not browse the filesystem, and
does not rename, move, or close anything.

This is the first write capability in herdr-go beyond replying into an existing pane.
Everything the app does today is observe plus reply.

## Locked Decisions

These are fixed. Planning must implement them exactly — cited, never reinterpreted.
Changing one requires the user, a new D-ID or an explicit supersession note, never
a silent edit.

| ID | Decision | Rationale |
|----|----------|-----------|
| D1 | A FAB at the switcher's bottom-right is the single entry point for both new shell and new agent. No `+` in the app header, none in workspace section headers. | Creating is the app's only write action, so it earns the thumb zone. The header already holds `logout` (`web/src/views/switcher.ts:65-82`) — a `+` beside it is a mis-tap hazard. A per-workspace `+` has a dead zone at zero workspaces. The one real argument for a contextual `+` was context, and `focused_workspace_id` supplies that to a global button for free. |
| D2 | One bottom sheet: a destination selector on top, action rows below — `Shell` first, then one row per configured agent preset. Not a two-step "pick a type, then fill a form" menu. | `tab.create` has zero required params; `agent.start` requires `name` + `argv` (`.bee/spikes/pbi-001-events-subscribe/schema.json:1292-1295`) and, per D5, must additionally be given a `cwd` we compute. `cwd` itself is optional in the schema — the obligation comes from D5, not the protocol. A shared configure step makes the cheap action pay the expensive one's price. This layout keeps both at two taps. |
| D3 | The destination is a single choice combining workspace and folder, shown as one row (label + path). Never two independent pickers. | herdr derives an *unrenamed* workspace's label from its live folder (call sites `upstreams/herdr/src/workspace.rs:1043-1076`; `derive_label_from_cwd` itself is defined in `upstreams/herdr/src/workspace/git/discovery.rs:21`), so for the normal case workspace and project folder are one thing to the user; two pickers would invent a distinction the product does not have. This is a strong default, not an invariant — a `custom_name` set through `workspace.rename` stops the label tracking the folder entirely (`workspace.rs:1055-1057, 1069-1071`), which is exactly why the row carries the path as well as the label. |
| D4 | The agent preset list (label + `argv`) lives in the gateway's `config.json` and is editable through `doctor`. The mobile UI only renders it. | herdr has no startable-agent registry. `server.agent_manifests` returns only screen-detection rule metadata — `agent`, `source`, `source_kind`, versions, warnings — with no `argv`, no display name, no icon (`.bee/spikes/pbi-001-events-subscribe/schema.json:5679-5738`). `agent.start` takes caller-supplied `argv`, so the list must be ours. Config keeps it changeable without a release, and `doctor` is already the config surface (PBI-013). |
| D5 | Both `tab.create` and `agent.start` pass `cwd` explicitly, resolved as `foreground_cwd ?? cwd` of the destination workspace's **anchor pane** (see D10 for how the anchor is found). | `agent.start` does **not** use the follow-cwd policy: with `cwd` omitted it falls back to the herdr server process's own directory, then `/` (`upstreams/herdr/src/app/agents.rs:118-122`) — an agent silently started in the wrong repo. herdr's docs state explicit `cwd` takes precedence over `[terminal] new_cwd` (`upstreams/herdr/docs/next/website/src/content/docs/configuration.mdx:73-80`), so this is a supported path, not a workaround. The sheet promised a folder; predictability beats config fidelity. `foreground_cwd` first mirrors herdr's own `follow_cwd_for_pane` (`upstreams/herdr/src/workspace/tab.rs:575-583`). |
| D10 | The anchor pane of **any** workspace is found by this join, which reproduces herdr's own `focused_pane_cwd_in_workspace` exactly: `workspace_id` → `WorkspaceInfo.active_tab_id` → the `layouts[]` entry whose `workspace_id` **and** `tab_id` both match → its `focused_pane_id` → that pane in `panes[]` → `foreground_cwd ?? cwd`. The snapshot's top-level `focused_*_id` fields are **not** used for this. | `SessionSnapshot.focused_workspace_id/focused_tab_id/focused_pane_id` and `PaneInfo.focused` are all global — they describe only the one globally active workspace (`upstreams/herdr/src/app/api/session.rs:17-28`, `app/creation.rs:390-394`). Using them would leave every non-focused destination — the normal case — with no anchor at all. But each `Workspace` keeps its own `active_tab` index and per-tab layout focus, independent of global focus (`upstreams/herdr/src/workspace.rs:167, 419-420, 1123-1124`), and both are wire-visible: `active_tab_id` on `WorkspaceInfo` (`app/creation.rs:453-454`) and per-tab `focused_pane_id` in `layouts[]`, which herdr emits for **every** tab of every workspace, not just the active one (`app/api/session.rs:33-42`, `app/api/panes.rs:1680-1732`). This join computes precisely the `follow_cwd` input that `tab.create` derives server-side (`app/api/tabs.rs:65-67` → `app/creation.rs:55-58`) — but not necessarily where herdr would land: under any `[terminal] new_cwd` other than `follow`, `resolve_new_terminal_cwd` discards that input entirely (`app/creation.rs:13-31`), which is the second reason D5 sends `cwd` explicitly. The join can also come up empty — a layout entry is dropped whole when its focused pane has no public id (`app/api/panes.rs:1686`), and `active_tab_id` falls back to an id synthesized from the tab index plus one (`app/creation.rs:453-456`), which can disagree with the real public tab number because those come from a monotonic counter (`upstreams/herdr/src/workspace.rs:510-511, 978-979`, read back via `public_tab_number` at `:1025`) that never reissues a closed tab's number. It never yields the *wrong* pane, so an empty result degrades per the fallback pattern below, never guesses. |
| D11 | When the anchor pane's folder disagrees with the folder implied by the workspace label, the **path wins**: it is what the sheet shows as authoritative and what goes on the wire. Planning must not try to reconcile the two. | herdr computes them from different panes on purpose — the label comes from the first tab's root pane (`workspace.rs:1043-1052`, `resolved_identity_cwd_from`) while the create-follow comes from the active tab's focused pane (`app/creation.rs:55-58`). Once a user `cd`s in one pane, the two legitimately diverge, and no client-side reconciliation can be correct. Showing both, with the path as the thing that governs, is the only honest presentation, and it is what D3's row shape already provides. |
| D6 | New shell uses `tab.create`, never `pane.split`. Both create calls pass `focus: false`; the phone navigates itself into the new pane's terminal detail. | `pane.split` requires a `direction` whose right/down meaning is empty on a phone with no layout view, where every pane is already a full-screen card. `tab.create` has no required params and does not cram the desktop layout. `focus: false` stops a phone action from stealing the desktop viewport, while the phone still shows what it just made. |
| D7 | The agent `name` is auto-generated and retried on `agent_name_taken`. The user never types it. | `agent.start` requires a globally unique name and rejects duplicates with `agent_name_taken`, listing the conflicts (`upstreams/herdr/src/app/agents.rs:208-253`). Typing a unique name on a phone keyboard is a punishment, and the collision is machine-detectable, so the machine resolves it. |
| D8 | Slice 1 creates only inside workspaces that already exist. No directory browser, no `workspace.create`. Reaching another folder is done with the existing Type panel: open a shell, `cd`, and the destination follows. | In herdr a workspace's folder is emergent, not chosen — the desktop cannot pick a project either; it spawns from the active pane's cwd and you `cd` from there. Because `cwd` is read live from `/proc/<pid>/cwd` on every request (`upstreams/herdr/src/platform/linux.rs:270-275`), the `cd` lands immediately and the workspace label follows. A directory picker would make mobile a different product from the desktop and would add a directory-listing endpoint plus its security surface for a case the Type panel already covers. |
| D9 | The destination dropdown reserves a slot for a future "Other project…" entry, but slice 1 ships without it. | Keeps the layout stable when PBI-020 lands, without paying for it now. |

### Agent's Discretion

The user delegated the UX shape (D1, D2, D3, D5, D6) to the agent with one constraint,
stated in the request: a new agent must always be created inside some workspace/project,
with the context either chosen or detected. D3 plus D5 satisfy that — the destination is
always explicit in the sheet and always sent on the wire.

Left to planning: the exact FAB iconography and sheet animation, the auto-generated name
format, how many presets render before the list scrolls, and the error-copy wording.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Destination | One workspace together with its live folder, presented as a single choice. Resolves to `workspace_id` + `cwd` on the wire. |
| Anchor pane | The pane whose folder seeds anything created in a destination workspace: that workspace's own active tab's layout-focused pane, found by the D10 join. Not the globally focused pane. Its `foreground_cwd ?? cwd` is the folder the sheet displays and sends. |
| Preset | A `{label, argv}` pair in the gateway's `config.json` that turns one sheet row into one `agent.start` call. |
| Shell | A pane with no agent attached — `tab.create`, not `agent.start`. |

## Specific Ideas And References

- The user framed this against the herdr desktop client, which has three "new" actions
  (workspace, tab, pane) whose meaning shifts with context. Mobile deliberately collapses
  these to two — new shell, new agent — because the phone has no layout view, so
  tab-versus-pane is invisible to the user and only affects the desktop's own layout.
- The user's model of workspace-folder identity — "herdr can't create a project either;
  it makes a workspace from the active folder, then you `cd` and the workspace settles
  there" — was verified against herdr's source and is correct. It is the basis of D8.

## Existing Code Context

From the scout only. Downstream agents read these before planning.

### Reusable Assets

- `web/src/views/switcher.ts:44-60` — `groupByWorkspace` already groups agents by
  `workspace_id`; the destination list can be built from the same grouping.
- `web/src/views/switcher.ts:65-82` — existing header with the `icon-btn` pattern
  (refresh, logout). The FAB is a new pattern beside it, not a replacement.
- `web/src/views/terminal.ts` + `docs/specs/terminal-detail.md:9-19` — the Type panel
  that sends text into a pane is the D8 escape hatch. It already exists and needs no
  change.
- `src/herdr/socket.rs:226-326` — the existing request/response call sites for
  `session.snapshot`, `ping`, `pane.read`, `pane.send_input`, `pane.send_keys`. The two
  new create calls follow the same shape.

### Established Patterns

- Opaque ids are read fresh from snapshots and never constructed or cached
  (`docs/specs/system-overview.md:47`). `workspace_id` and the anchor pane's `cwd` follow
  that rule — read them from the snapshot at the moment the sheet opens, not from
  remembered state.
- Snapshot joins fall back rather than panic (`src/herdr/wire.rs:133-151`,
  `workspace_label_for` / `tab_label_for`). A destination whose anchor pane has no
  resolvable cwd must degrade the same way, not throw.

### Integration Points

- `src/herdr/wire.rs:45-78` — `RawAgent` parses seven fields (`pane_id`, `workspace_id`,
  `tab_id`, `agent`, `name`, `agent_status`, `terminal_title_stripped`) and silently drops
  everything else, including `cwd` (visible in the captured fixture at `wire.rs:221-237`).
  `Snapshot` itself (`wire.rs:112-120`) keeps only `agents`, `workspaces`, `tabs` and
  discards the rest of the envelope.
  Enabling work, all of it in this file: parse `cwd` and `foreground_cwd` on panes; add
  `WorkspaceInfo.active_tab_id`; and add the two arrays the D10 join needs but the
  gateway does not read at all today — `layouts[]` (for `focused_pane_id` per tab) and
  `panes[]`. `panes[]` is not optional: the anchor is frequently a plain shell, which
  never appears in `agents[]`. The top-level `focused_*_id` fields are worth parsing only
  to preselect the default destination, never to resolve an anchor (D10).
- `src/herdr/mod.rs:34-53` — the `Herdr` trait's five methods. Two more are needed
  (`tab.create`, `agent.start`); `FakeHerdr` must grow with it.
- `src/web/api.rs:18-52` — `AgentRow`, the JSON shape the frontend consumes. The
  destination list and its folder paths ride on this surface or a sibling of it.
- `src/config/mod.rs` — home of the new `agent_presets` config field (D4). Follow how
  `allowed_roots` threads through all five sites, not just one: the `Config` field at
  `:30`, the raw parse form at `:95`, validation at `:180-204`, the `ConfigError` Display
  arm at `:141`, and the default-JSON template `ensure_config` writes at `:750-763`. The
  error arm matters here: validating a malformed preset (`invalid_agent_argv`, below)
  needs its own variant.
- `src/doctor/` — the presets need an edit path here per D4 and PBI-013.

## Canonical References

- `.bee/spikes/pbi-001-events-subscribe/schema.json` — captured herdr protocol 16 schema
  (herdr 0.7.4). `AgentStartParams` at `:1242-1297`, `TabCreateParams` at `:3620-3652`,
  `SessionSnapshot` at `:9054-9123`, `PaneInfo` at `:6779-6897`, `WorkspaceInfo` at
  `:9170-9234`, `PaneLayoutSnapshot` at `:6954-6993`, `agent_started` response at
  `:8274-8296`, `tab_created` at `:8219-8237`.
- `upstreams/herdr/src/app/api/session.rs:17-42` — how the snapshot's global `focused_*`
  fields and the per-tab `layouts[]` are built. The distinction between the two is the
  whole of D10.
- `upstreams/herdr/src/app/creation.rs:55-58` with `upstreams/herdr/src/app/api/tabs.rs:65-67`
  — `focused_pane_cwd_in_workspace`, the server-side function D10 reproduces on the client.
- `upstreams/herdr/docs/next/website/src/content/docs/socket-api.mdx` — the hand-written
  method table (`:99-112`) and the `cwd` versus `foreground_cwd` note (`:616`).
- `upstreams/herdr/docs/next/website/src/content/docs/configuration.mdx:73-80` — the
  `[terminal] new_cwd` policy and the statement that explicit `cwd` wins.
- `upstreams/herdr/src/app/agents.rs:98-253` — `agent.start` placement resolution, cwd
  fallback, and the full error-code set.
- `upstreams/herdr/src/app/creation.rs:13-31` — `resolve_new_terminal_cwd`.
- `upstreams/herdr/src/workspace.rs:1043-1076` — workspace label derived from its live
  folder.
- `docs/specs/switcher.md`, `docs/specs/terminal-detail.md` — the two screens this
  feature touches.
- `docs/DISCOVERY.md` — the live protocol findings this repo already recorded.

## Outstanding Questions

### Resolve Before Planning

- [ ] None. The gray areas that blocked planning are locked in D1–D9.

### Deferred To Planning

- [ ] **Windows destination accuracy.** `foreground_cwd` is unix-only — it returns `None`
      under `#[cfg(not(unix))]` (`upstreams/herdr/src/pane.rs:2717-2740`) — and on Windows
      PowerShell the plain `cwd` is only correct when herdr's OSC 9;9 shell integration is
      active, because PowerShell never updates its Win32 process cwd on `Set-Location`
      (`upstreams/herdr/src/pane.rs:1333-1338`). herdr-go ships Windows support (PBI-012).
      Planning must decide what the sheet shows when the anchor cwd is missing or stale on
      Windows: degrade to workspace label only, show the path with a caveat, or block the
      action. Do not silently display a wrong folder — D5 exists precisely to stop an
      agent starting in the wrong repo.
- [ ] **Auto-generated name format** and its collision-retry strategy (D7) — needs the
      `agent_name_taken` error payload shape confirmed against a live herdr.
- [ ] **The rest of the `agent.start` error surface.** D7 only locks the
      `agent_name_taken` retry. `agent.start` also emits `invalid_agent_name`,
      `invalid_agent_argv`, `agent_placement_not_found`, `agent_placement_conflict` and
      `agent_start_failed` (`upstreams/herdr/src/app/agents.rs:208-253`), and `tab.create`
      emits `workspace_not_found`
      (`upstreams/herdr/src/app/api/tabs.rs:57, 63`) and `tab_create_failed`
      (`upstreams/herdr/src/app/api/tabs.rs:126`). `invalid_agent_argv` is reachable
      from a malformed operator-written preset (D4), so it is a validation question for
      the config surface, not only error copy. A stale destination — the workspace closed
      on the desktop between opening the sheet and tapping — surfaces as
      `workspace_not_found` / `agent_placement_not_found` and needs a defined behaviour.
- [ ] **Where the destination list is assembled** — backend (`src/web/api.rs`) or frontend
      from the existing agent rows. Depends on whether the frontend needs per-pane cwd for
      anything else.

## Deferred Ideas

- **Create a workspace for a project that is not open yet** — browse under
  `allowed_roots`, then `workspace.create{cwd, label}`. Deferred per D8; the slot is
  reserved per D9. Note this would make mobile *more* capable than the herdr desktop,
  which cannot pick a folder at all — so it is a product question, not a technical debt.
  Backlog: PBI-020.
- **"New shell here" from terminal detail** — the highest-context place to create, since
  you are already looking at the project. Deferred: with D1's destination prefilled from
  `focused_workspace_id`, it saves at most one tap and costs a second UI path.
  Backlog: PBI-021.
- **Naming a workspace from the phone** (`workspace.rename` / `custom_name`) — surfaced
  while reading `derive_label_from_cwd`. Adjacent, out of scope, and overlaps PBI-008.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked
decisions, code context, canonical references, and deferred-to-planning questions.
Validating and reviewing use locked decisions for coverage and UAT.

The single largest piece of enabling work is in `src/herdr/wire.rs`: the gateway already
receives `focused_*_id`, `cwd`, and `foreground_cwd` in every snapshot it polls, and
throws all of them away. Nearly every decision above rests on parsing them.

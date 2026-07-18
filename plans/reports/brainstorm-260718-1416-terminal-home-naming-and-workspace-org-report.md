# Brainstorm — Terminal home naming & workspace org (PBI-006, PBI-007)

Standalone analysis, no source changes made. herdr socket probed live (read-only session.snapshot) to verify claims against real running data, not just code reading.

## 1. Current data model — findings

**The critical finding: workspace/tab human-readable names already exist on the herdr socket and are already flowing over the wire — herdr-gateway's own Rust layer is currently discarding them.**

Chain of evidence:

- src/herdr/wire.rs:46-58 — the Agent struct we deserialize into only keeps 5 fields from the wire: pane_id, workspace_id, tab_id, kind (agent), status (agent_status), title (terminal_title_stripped). workspace_id/tab_id here are opaque IDs (w3, w3:t6), not names.
- src/herdr/wire.rs:61-64 — Snapshot { agents: Vec<Agent> } is the only field pulled from the socket response's result.snapshot object. Serde silently drops everything else in that object (no deny_unknown_fields).
- Live probe of the real running herdr socket (session.snapshot, read-only) shows result.snapshot actually contains 9 top-level keys: agents, focused_pane_id, focused_tab_id, focused_workspace_id, layouts, panes, protocol, tabs, version, workspaces. Only agents is consumed today.
  - workspaces[] items look like: workspace_id "w7", number 3, label "herdr-gateway", focused true, pane_count 4, tab_count 2, active_tab_id "w7:t1", agent_status "working" — label is a real human name (observed values: "fgos-dev", "forgent", "herdr-gateway" — these are project names, exactly what PBI-006 wants).
  - tabs[] items look like: tab_id "w3:t6", workspace_id "w3", number 6, label "ui", focused false, pane_count 1, agent_status "idle" — tabs also carry a human label ("ui", "chat", "workers-2").
  - panes[] items carry cwd, foreground_cwd, terminal_title/terminal_title_stripped, and optionally their own label (e.g. "session D", set via a pane rename call) — a pane can be manually named, separate from its agent's terminal title.
- Docs corroboration (docs/distillery/deep-dives/how-to-use-herdr.md:351-354): herdr's socket API exposes workspace rename, tab rename (keybinding prefix+shift+t), and pane rename — these labels are first-class, user-settable, and already how the herdr TUI sidebar itself renders (ui.sidebar.agents config: rows = state_icon, workspace, tab / agent — line 222 — herdr's own sidebar already shows workspace+tab as name tokens, not raw IDs).
- On the herdr-gateway side, downstream of Agent:
  - src/web/api.rs:16-23 AgentRow exposes pane_id, workspace (=workspace_id, still opaque), display, kind, status, title to the web client.
  - src/web/api.rs:37-48 builds display via Snapshot::display_for() (src/herdr/wire.rs:66-75): "{kind} dot {title-or-kind}", e.g. "claude dot Kiem tra plan". No workspace or tab name anywhere in this string.
  - web/src/api.ts:5-14 mirrors the same shape in TS.
  - web/src/views/switcher.ts:81-97 (the home list, agent-list ul) renders only row.display (primary line) and row.kind (secondary caption, monospace). row.workspace is fetched into every row but never read anywhere in switcher.ts or terminal.ts — confirmed via grep, zero references. It's dead data today, and even if rendered it's just the opaque workspace_id, not a name.

Bottom line: nothing needs to be invented for PBI-006. The workspace label and tab label already exist upstream, already update live (herdr emits workspace.renamed/tab.renamed events per the event family list), and just need to be: (a) added to the Agent/Snapshot wire structs by parsing workspaces[]/tabs[] alongside agents[] and joining on workspace_id/tab_id, (b) threaded through AgentRow, (c) rendered in the switcher list.

## 2. PBI-006 — naming/display options

Fields available once wired through: workspace_label, tab_label, pane_label (optional, rare), kind, title (terminal_title_stripped). Current single line is "kind dot title".

**Option A — Breadcrumb replaces the single title line**
"workspace dot tab dot title" (e.g. "herdr-gateway dot ui dot Kiem tra plan"), kind stays as the existing secondary caption line.
- Pro: minimal DOM/CSS change — same two-line card, just changes what populates agent-path.
- Con: three segments plus separators can get long: agent-path already has text-overflow ellipsis (styles.css:454-456), so it degrades gracefully, but if all agents share the same workspace (common case — solo dev on one project) the workspace segment adds width for zero disambiguation value.

**Option B — Workspace as a small badge/label, title stays primary**
Primary line stays "kind dot title" (current). Add a small pill/badge (like status-badge, styles.css:466-474) showing workspace label only when it's not the only workspace, tab shown as a lighter sub-caption.
- Pro: doesn't penalize the common single-workspace case; workspace becomes a scanning aid only when it's actually disambiguating.
- Con: more markup/CSS states than Option A; deciding when to show it is a rule that has to be gotten right (e.g. show only if more than one distinct workspace across the current list).

**Option C — Two-line hierarchical: workspace/tab on top (muted), title+kind below (primary)**
Inverts current hierarchy — treat workspace as the "folder" and title as the "file", agent-kind-style muted caption on top, agent-path-style bold text below unchanged.
- Pro: matches the mental model PBI-007 wants (workspace-first organization) — sets up naturally for grouping below.
- Con: bigger visual change than A/B; needs both switcher.ts markup and possibly new CSS classes (not just re-populating existing ones).

**Option D — Full path breadcrumb identical to herdr's own sidebar token order**
"workspace, tab, pane-label-or-title" mirroring herdr's own sidebar row convention exactly, so a user who knows herdr's own UI sees the same mental grouping in the web view.
- Pro: consistency with the tool the data comes from — no cognitive translation needed for existing herdr users.
- Con: still has the single-workspace-clutter problem of Option A; requires pulling in pane_label too (a third field, usually empty).

Fallback rule needed regardless of option: pane_label is rare (only set via explicit rename) so fall back to title (terminal_title_stripped), then fall back to kind alone, exactly as display_for() already does today for title/kind. tab_label/workspace_label are always present in the herdr payload observed live (herdr auto-assigns a label on create), so no empty-string fallback needed there in practice — but code defensively for it anyway since it's user-renameable to blank in theory.

Recommendation: Option B, evolving toward Option C if/when PBI-007 grouping lands. Reasoning: the backlog's actual pain point is "hard to tell terminals apart," which is a disambiguation problem, not a hierarchy display problem — most days a solo developer's whole list is one workspace, and forcing "workspace dot tab dot title" onto every row (Option A/D) adds visual noise for zero information in that dominant case. Option B only spends pixels on workspace/tab identity when there's more than one workspace active, which is exactly when it's needed. It's also the cheapest to ship standalone before PBI-007 grouping exists (KISS — doesn't presuppose a grouped layout that PBI-007 might still redesign).

Data model changes needed: none are additions to herdr's data — everything needed already exists upstream. The work is entirely in herdr-gateway's own parsing/threading: extend Agent/Snapshot (src/herdr/wire.rs) to also deserialize workspaces[]/tabs[] and resolve workspace_label/tab_label/pane_label per agent, then extend AgentRow (src/web/api.rs) and the TS AgentRow type (web/src/api.ts) with the new fields, then update switcher.ts rendering.

## 3. PBI-007 — workspace grouping/filtering options

**Option 1 — Grouped sections with sticky/collapsible headers**
Home list becomes N sections, one per workspace, header equals workspace label plus rolled-up status dot (herdr already computes workspace-level agent_status rollup server-side — see workspaces[].agent_status in the probe above, working/done/idle per workspace — free rollup, no client aggregation needed). Each section lists its agent cards as today.
- Pro: matches herdr's own mental model 1:1 (workspace to tab to pane rollup is literally how herdr's sidebar already works); the rollup status is already computed upstream so this is cheap; naturally extends to a machine outer grouping later (one more nesting level) without restructuring.
- Con: more list-rendering logic than a flat list; collapse state needs to persist across refreshes (localStorage) or it resets every poll, which would be annoying with polling-based refresh.

**Option 2 — Filter chips/dropdown above a still-flat list**
Add a workspace filter control (chips or a select) above the existing flat list; selecting a workspace filters the array client-side before rendering. No change to the list rendering itself.
- Pro: simplest possible implementation — a filter step before the existing renderList(), zero change to card markup; trivially fast to build.
- Con: doesn't solve "sort/organize" as directly as grouping — user still has to pick a filter to see structure, rather than seeing it all at a glance; less "professional" looking for the stated goal (backlog note quotes the user asking for whichever is more professional).

**Option 3 — Sidebar tree (workspace to tab to pane), detail panel on the right**
Full nested tree navigation, closer to a file-explorer or herdr's own TUI. Selecting a leaf opens the terminal detail view.
- Pro: most scalable/professional-looking for a genuinely large number of workspaces/machines; matches the target machine/workspace/tab/pane hierarchy exactly.
- Con: significant layout rework — this app is mobile-first (switcher.ts has pull-to-refresh and swipe-gesture support; styles.css variables suggest a card-based mobile list, not a desktop sidebar layout). A tree sidebar is a poor fit on phone width and would need a responsive fallback (e.g. become Option 1 on mobile) — real added complexity for what's currently a small dataset (4 workspaces, 11 panes observed live).
- Violates YAGNI/KISS given the current scale (single machine, small agent counts observed).

**Option 4 — Tabs-per-workspace (top-level tab bar, one tab equals one workspace)**
Horizontal tab bar swaps the whole list view per workspace.
- Con: actively worse than Option 1/2 for the stated goal — hides other workspaces' rolled-up status entirely (can't glance-scan which workspace needs attention across all of them, which is the whole point of herdr's rollup design). Not recommended.

Recommendation: Option 1 (grouped sections), built directly on top of the PBI-006 Option B work (same new workspace_label/rollup-status fields, no additional data plumbing). Reasoning: it's the one option that is genuinely simple to build now (group-by is a one-pass reduce over the existing flat array, no framework/routing change), uses data herdr already computes for free (per-workspace rollup status), and is the natural on-ramp to a machine outer layer later (nest one more level, don't redesign) — satisfying the professional-and-future-proof ask without gold-plating it today.

Does a "machine" concept exist today? No — confirmed nowhere in the wire protocol. session.snapshot and every workspace/tab/pane/agent method (docs/distillery/deep-dives/how-to-use-herdr.md:351-360) are scoped to one herdr server process talking over one local Unix socket (~/.config/herdr/herdr.sock). herdr does have a "named sessions" concept (separate runtime namespaces, own panes/tabs/workspaces/sockets — how-to-use-herdr.md:181), but that's still single-machine (multiple herdr sessions on the same box), not cross-machine. herdr-gateway itself (this repo) is also currently single-instance: one gateway process talks to one herdr socket and serves one web UI — there is no concept anywhere in src/ of which machine a snapshot came from.

Minimal non-disruptive schema note (not recommended to build now): if this ever needs a machine layer, the smallest addition would be a single config-driven machine_label (e.g. from herdctl config, defaulting to hostname) stamped onto every AgentRow/WorkspaceGroup server-side — a scalar field, not a restructuring — with actual cross-machine aggregation handled by a future gateway-of-gateways proxy or the web client fetching multiple /api/agents endpoints and merging client-side. Grouping the current home list by workspace (Option 1) already produces the right shape to extend to a machine-then-workspace nesting later without touching the workspace-grouping logic itself — so building Option 1 now doesn't paint anyone into a corner. Do not build the machine field or multi-gateway aggregation now — no second machine exists yet to validate the design against (YAGNI).

## 4. Open questions for later (not blocking, for a human to weigh in on eventually)

1. Collapse-state persistence for Option 1's group headers — per-session only, or persisted (localStorage) across visits?
2. Sort order within a workspace group, and sort order of the groups themselves (alphabetical by label? most-recently-active? blocked/working workspaces first, mirroring herdr's own attention-triage purpose)?
3. Should a workspace with all agents done/idle auto-collapse by default, surfacing only workspaces needing attention (working/blocked) expanded — leaning further into the at-a-glance goal PBI-007 implies?
4. Pane rename is rare in the live data observed (1 of 11 panes) — is it worth exposing pane-level renaming as a herdr-gateway feature later (calling the pane rename method from the web UI), or is that purely a herdr-native workflow this app shouldn't duplicate?
5. When/if multi-machine aggregation is eventually built: does that live in this gateway (federate multiple herdr sockets), a separate aggregator service, or purely client-side (web UI polling multiple gateway URLs)? Explicitly deferred — no second machine to design against yet.
6. Should the workspace status rollup (already computed by herdr) also feed a "needs attention" home-level summary/banner independent of the list itself, or is the grouped list enough signal on its own?

---

Summary for chat: Good news — PBI-006's ask (name terminals by workspace and tab, not just description) needs zero new data from herdr: a live probe of the running socket confirmed session.snapshot already returns human-readable label fields for every workspace and tab (for example "herdr-gateway" and "ui"), and herdr-gateway's own Rust layer just isn't parsing or forwarding them yet (src/herdr/wire.rs only keeps opaque IDs today). Recommended display: keep the current title as primary and add workspace/tab as a secondary badge only when more than one workspace is present, rather than always prepending a breadcrumb (avoids clutter in the common single-workspace case). For PBI-007, recommend grouping the home list into collapsible per-workspace sections using herdr's already-computed per-workspace status rollup — it's the simplest option that also sets up cleanly for a future machine outer layer, which does not exist in herdr's protocol or this app today and should stay unbuilt until a second machine actually needs it. Full findings, all four display/grouping options considered, and open product questions are in the report file.

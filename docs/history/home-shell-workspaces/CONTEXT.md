# Home Shell Workspaces — Context

**Feature slug:** home-shell-workspaces
**Date:** 2026-07-21
**Exploring session:** complete
**Scope:** Standard
**Domain types:** SEE, CALL

## Feature Boundary

A workspace whose panes are all plain shells (no agent attached) is
currently invisible on the home/switcher screen, because that screen is
built entirely from `snap.agents`. This feature makes each such shell pane
appear as its own row on home, tappable into its terminal detail, using the
exact same navigation mechanism `web-create-sheet` already built for a
newly-created pane with no agent record. It does not touch any workspace
that already has at least one agent showing — a shell pane living alongside
agents in the same workspace stays exactly as invisible as it is today (out
of scope, per D3).

## Locked Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| D1 | Each shell pane in an agentless workspace gets its own row on home, at the same granularity as today's agent rows (one card per pane, not one card per workspace). Tapping a shell row opens that specific pane's terminal detail. | User's explicit choice over grouping into one workspace-level row, despite the workspace-level option being simpler to build and matching the app's existing coarse-grained navigation precedent (D8 of `new-shell-new-agent`). The user wants direct, per-pane access when a workspace has more than one shell. |
| D2 | A shell row is visually distinct from an agent card: no status badge at all (not `unknown`, not a new "shell" status value slotted into the same badge). | User's explicit choice, made after being shown the alternative (a dedicated "Shell" badge reusing the existing badge slot). Also avoids the exact category error `critical-patterns.md` already records happening twice: never borrowing `unknown` (herdr-reported-but-unrecognized) to mean "there is nothing here." |
| D3 | This feature only surfaces shell panes belonging to a workspace with **zero** agents. A shell pane inside a workspace that already shows agent cards is unaffected — still invisible, unchanged. | The user's D1 answer was scoped to "workspace chỉ có shell (không có agent)" in the question itself, matching PBI-024's original bug report exactly (a workspace with zero agents, not every shell pane everywhere). |
| D4 | The existing `groupByWorkspace` grouping is reused unchanged for forming groups. A shell-only workspace naturally forms its own group when 2+ distinct workspaces are currently visible. | `groupByWorkspace` already groups any row carrying a `workspace_id`, agent or not — no new grouping logic needed. Header badge behavior for a zero-agent group is D7, not this decision. |
| D7 | A group's header status badge is hidden entirely when that group has zero agent rows (i.e. every row in it is a shell entry). This is a client-side check against the rows already fetched — no new backend field, no herdr status resolved or requested for the group. | Verified against herdr's own source: `Workspace::aggregate_state` (`upstreams/herdr/src/workspace/aggregate.rs:91-105`) explicitly returns `AgentState::Unknown` when a workspace has zero agents, which maps to `AgentStatus::Unknown` on the wire. Showing that value on a group header would be exactly the category error D2 exists to avoid, one level up — herdr's `Unknown` here genuinely means "nothing to aggregate," not "an unrecognized value," but displaying it verbatim still misleads. User's explicit choice over showing herdr's literal value or never grouping shell-only workspaces at all. |
| D5 | Tapping a shell row navigates into terminal detail using `web-create-sheet`'s `NewPaneRef` (`pane_id`, `workspace_id`, `label`), not a full `AgentRow`. | A shell pane has no agent record to fetch — no `kind`, `status`, or agent `title` exist for it. This is exactly the shape `NewPaneRef` was built for (S5 of `web-create-sheet`); reusing it avoids a second minimal-reference type. |
| D6 | A shell row's primary line is the pane's folder path (falling back to "no folder yet" when unresolved, matching `create-sheet.md`'s existing wording); its caption is `"Shell · <tab label>"`, mirroring `switcher.md`'s title/kind-caption row shape. No kind watermark (there is no kind for a plain shell). | Directly extends already-specced copy patterns (`switcher.md` rows 1-2, `create-sheet.md`'s path fallback) instead of inventing new wording. The path — not the workspace label — is what distinguishes two shell panes inside the same workspace. |

### Agent's Discretion

Exact spacing/visual weight of the no-badge shell row relative to an agent
card, and whether the folder path truncates/wraps on long paths, are left to
implementation — neither changes what data is shown or what tapping does.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Shell entry | A home-screen row representing one plain-shell pane (no agent attached) inside an otherwise-agentless workspace. Distinct from "Shell" the create-sheet action (which creates a new one) and from an agent card (which always has a status badge). |

## Specific Ideas And References

- None beyond the two Socratic answers above — no external mockup or
  reference was supplied.

## Existing Code Context

From the quick scout only. Downstream agents read these before planning.

### Reusable Assets

- `web/src/main.ts` — `NewPaneRef` (S5 of `web-create-sheet`), the minimal
  reference type this feature's D5 reuses for navigation.
- `web/src/views/terminal.ts` — `terminalHead()`, the pure function already
  handling `AgentRow | NewPaneRef` for the terminal-detail title/kind; a
  shell entry's tap target needs no new terminal-detail logic.
- `web/src/views/switcher.ts` — `groupByWorkspace`, `renderAgentCard` (the
  row shape D6 partially mirrors), `renderWorkspaceSection`.
- `src/herdr/wire.rs` — `Pane { pane_id, workspace_id, tab_id, cwd,
  foreground_cwd }` already exists and is already parsed from every
  snapshot (`new-shell-new-agent` slice 1); no new herdr-side parsing is
  needed for the raw data.

### Established Patterns

- `unknown` is reserved for "herdr reported a value this app doesn't
  recognize" — never repurposed for "there is nothing to report"
  (`switcher.md` Data Dictionary row 3; the same category error already
  recurred twice per `critical-patterns.md`). D2 exists specifically to
  avoid a third occurrence.
- Opaque ids are read fresh from the snapshot, never cached
  (`system-overview.md`).

### Integration Points

- `src/web/api.rs:29-57` — the `agents()` handler (`GET /api/agents`) builds
  its response by iterating `snap.agents` only; `snap.panes` (same round
  trip, already parsed) is not read here today. This is the backend
  enabling work: either widen this handler's response or add the
  shell-pane rows some other way — the exact shape is planning's call.
- `web/src/views/switcher.ts` — where shell rows join the existing render
  path (`renderList`/`renderAgentCard`/`groupByWorkspace`).
- `web/src/api.ts` — `AgentRow`'s type may need a sibling or a union to
  carry a shell row's shape (`pane_id`, `workspace_id`, `workspace_label`,
  `path`, `tab_label`) without agent-only fields (`kind`, `status`,
  `display`, `title`, `workspace_status`) — per D7, a shell row never
  carries or needs a `workspace_status` value.
- `web/src/views/switcher.ts:55,172-175` — `renderWorkspaceSection`'s
  header-badge render, the site D7's hide-when-zero-agents check attaches
  to (a client-side count over `group.rows`, not a new fetch).

## Canonical References

- `docs/backlog.md` PBI-024 — the original bug report and its confirmed
  live evidence (`wB`, the focused workspace, invisible with 1 shell pane
  and 0 agents).
- `docs/specs/switcher.md` — the screen this feature extends.
- `docs/specs/create-sheet.md`, `docs/specs/terminal-detail.md` — the two
  screens D5/D6 cite for reused shapes and wording.
- `docs/history/web-create-sheet/CONTEXT.md` S5 — `NewPaneRef`'s origin.
- `docs/history/learnings/critical-patterns.md` — the `unknown`-misuse
  lesson D2 avoids repeating.

## Outstanding Questions

### Resolve Before Planning

- [ ] None. D1-D7 cover every gray area material to this slice.

### Deferred To Planning

- [ ] Exact backend response shape for shell-pane rows: widen
      `GET /api/agents`'s existing response, or a separate endpoint/field.
      Either is compatible with D1-D7; the tradeoff (one round trip vs a
      cleaner type split) is an implementation call.
- [ ] Whether the frontend union type for "a home row" (`AgentRow` or a new
      shell-row shape) lives in `api.ts` alongside `AgentRow`, or as its own
      type — implementation detail, not scope.

## Deferred Ideas

- **Full pane/agent/terminal terminology standardization across the whole
  app** (already tracked as `docs/backlog.md` PBI-026). This feature adds
  one narrowly-scoped term ("Shell entry") without attempting that broader
  cleanup — PBI-026 stays a separate, deferred effort.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs D1-D7 are stable. Planning
reads locked decisions, the Integration Points (especially the backend
response-shape question left open above), and Canonical References.

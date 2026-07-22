# PBI-046 Shell Card Group — Context

**Feature slug:** pbi-046-shell-card-group
**Date:** 2026-07-22
**Exploring session:** complete
**Scope:** Standard
**Domain types:** SEE, ORGANIZE

## Feature Boundary

On the home switcher, a shell row gets a leading distinguishing icon and a solid-black card background (visually distinct from agent cards), and a shell's group merges into the agent group that shares its `workspace_label` — frontend-only, no backend/data-model change.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | A shell's group merges into an agent group when their `workspace_label` strings are exactly equal — a frontend-only change in `buildHomeGroups` (`web/src/views/switcher.ts:89-115`). No backend/data-model change. Grouping is NOT by `workspace_id` (that overlap case is already impossible: server-side filtering in `src/web/api.rs:80-96` drops any shell pane whose `workspace_id` also has an agent, proven by the passing test `homeshell_workspace_with_agents_contributes_no_shell_rows`, `src/web/api.rs:246-254`). Accepted risk: two unrelated workspaces whose labels coincidentally collide (label is not a unique key — `src/herdr/wire.rs:111`, defaults to directory basename but is user-renameable, per `docs/specs/herdr-port.md:44`) would be incorrectly merged into one home section. | User re-checked their observed case and confirmed it involves two different workspace paths (e.g. a worktree vs. its main checkout) that happen to share a `workspace_label` — not the same `workspace_id`. The data model has no "project"/repo-root concept to key on instead (`Workspace` struct, `src/herdr/wire.rs:109-117` — only `workspace_id`, `label`, `agent_status`, `active_tab_id`). User explicitly chose the cheap label-match approach over descoping this item or building a new backend project-identity concept, accepting the collision risk. |
| D2 | The original PBI-046 wording ("gộp chung group với agent cùng workspace/project", citing `buildHomeGroups`'s D3 comment as a wrong assumption) is corrected: the code comment at `switcher.ts:81-88` is accurate ABOUT TODAY'S CODE — a shell pane in a workspace that also has an agent is already filtered out server-side and never reaches the frontend. The actual gap D1 fixes is cross-workspace grouping by shared label, not same-workspace overlap. Once D1 lands, the comment's claim "shell rows never land in a group that has an agent card" becomes false (a label-merged group now holds both) — planning/execution must update that comment as part of implementing D1, not leave it stale. | Verified via the Rust filter and its passing test (see D1). PBI-024's D3 (`docs/history/home-shell-workspaces/CONTEXT.md`) stands unchanged and is not being reversed — this feature adds a new, separate label-based merge on top of it. |
| D3 | A shell row gets a leading (leftmost, in-flow) small monochrome icon — a distinct visual element from the agent card's existing background watermark (`.agent-watermark`, `styles.css:502-514`, a large faded letter, not a leading icon). Neutral/muted color, not the kind-hash accent color (`kindAccentColor`), since shells carry no `kind`. | Agent's Discretion (exact glyph/SVG path, sizing) — see below. |
| D4 | Shell card background becomes solid black, overriding the shared `--bg-elevated` currently inherited from `.agent-card` (`styles.css:454-463`) — scoped to `.shell-row` only, so agent cards keep `--bg-elevated` unchanged. | No existing "pure black" token exists in `styles.css` today; planning/execution introduces the literal value or a new token, whichever fits the existing token conventions there. |

### Agent's Discretion

- Exact SVG glyph for the shell icon (D3) and its precise size/spacing, as long as it sits at the leftmost in-flow position of the shell row and is visually distinct from the agent watermark treatment.
- Exact black value/token for D4 (e.g. a literal `#000` vs. a new named CSS variable), matching whatever convention the rest of `styles.css`'s color tokens already follow.
- Whether label-matching (D1) is case-sensitive/exact-string only, or should trim whitespace — default to exact string equality unless planning finds an existing normalization helper to reuse.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| workspace_label | A display string reported by herdr per workspace, defaulting to the directory basename but independently user-renameable (`src/herdr/wire.rs:111`, `docs/specs/herdr-port.md:44`). Not unique across `workspace_id` values. |

## Existing Code Context

### Reusable Assets

- `web/src/views/switcher.ts:89-115` — `buildHomeGroups`, the function D1 changes.
- `web/src/views/switcher.ts:209-222` — `renderShellRow`, where D3's icon is added.
- `web/src/styles.css:454-463` — `.agent-card` shared class; D4 needs a `.shell-row`-scoped override, not a change to this shared rule.
- `web/src/views/switcher.ts:230-237` — `renderGroupBadge` (PBI-024 D7): already hides the header badge based on `rows.some(agent)`, so a merged group (D1) correctly shows the agent's status badge with no extra change needed — PROVIDED the merge folds shell rows into the existing agent-group object (keeping its non-null `workspace_status`) rather than building a new group, since a shell-only group carries `workspace_status: null` (`switcher.ts:104`).

### Established Patterns

- `kindAccentColor` (`switcher.ts:50-57`) — deterministic per-`kind` accent color used for agent watermarks; explicitly NOT reused for the shell icon (D3) since shells have no `kind`.
- `.agent-watermark` (`styles.css:502-514`) — background decorative letter, absolutely positioned on the right; D3's shell icon is a different, in-flow leading element, not a variant of this pattern.

### Integration Points

- `src/web/api.rs:80-96` — backend `agents()` handler; confirmed NOT to be touched by this feature (D1/D2 are frontend-only).

## Canonical References

- `docs/history/home-shell-workspaces/CONTEXT.md` — PBI-024's original D1-D7, including D3 (server-side zero-agent filter), which this feature builds on top of, not against.
- `src/web/api.rs:246-254` — `homeshell_workspace_with_agents_contributes_no_shell_rows`, proof the same-`workspace_id` overlap case is already handled.

## Deferred Ideas

- A real backend "project identity" concept (e.g. canonicalizing a path modulo a `--wt--<slug>` worktree suffix, or matching by git remote/root) so cross-worktree grouping is correct instead of label-based best-effort. Deferred because it is materially bigger than this UI-polish item and needs its own exploration pass. Filed to `docs/backlog.md` as a new proposed item.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked decisions, code context, canonical references. D1/D2 replace the original backlog item's premise about `workspace_id` overlap — implement per this CONTEXT.md, not the original PBI-046 wording.

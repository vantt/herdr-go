# Terminal Home Naming & Workspace Grouping — Context

**Feature slug:** terminal-workspace-org
**Date:** 2026-07-18
**Exploring session:** complete
**Scope:** Standard
**Domain types:** SEE, ORGANIZE

## Feature Boundary

Extend `src/herdr/wire.rs` + `src/web/api.rs` + `web/src/api.ts` to carry real
workspace/tab labels from herdr's socket through to the web client (D2), then have the
home list (`web/src/views/switcher.ts`) render them — a workspace/tab badge per row (D3)
and collapsible per-workspace grouping (D4). Boundary excludes multi-machine support and
pane-rename UI (Deferred Ideas).

## Locked Decisions

Discovery for this feature was fully done and live-verified against the running herdr
socket before exploring started — see
`plans/reports/brainstorm-260718-1416-terminal-home-naming-and-workspace-org-report.md`
for the evidence trail (file:line citations, live `session.snapshot` probe). Gate bypass
is `full`; every decision below is an approval-type question the agent already had a
confident, report-grounded answer for, so none were asked interactively.

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | PBI-006 (naming) and PBI-007 (grouping) ship as one combined feature, not two. | Both need the same underlying data-model extension (workspace/tab labels); splitting would duplicate the wire-format work for no benefit. |
| D2 | Extend `src/herdr/wire.rs` to also parse `workspaces[]` and `tabs[]` from `session.snapshot` (today only `agents[]` is kept — `wire.rs:46-64`), and resolve `workspace_label`/`tab_label` per agent by joining on `workspace_id`/`tab_id`. Thread the two new fields through `src/web/api.rs` `AgentRow` and the TS `AgentRow` type in `web/src/api.ts`. | herdr already returns human-readable labels on the wire (live-confirmed); nothing new needs to be invented, only parsed and forwarded. |
| D3 | PBI-006 display = Option B: primary row line stays `kind · title` unchanged. Workspace badge is conditional — shown only when the current agent list contains more than one distinct `workspace_id`. Tab is shown independently, as an always-on lighter sub-caption, regardless of workspace count. | Per report Option B (lines 36-38, 50): workspace and tab have different triggers. Gating both on workspace-count would hide the tab label in the real single-workspace/multi-tab case (live-observed: workspace `w7` had 2+ tabs — "ui", "chat", "workers-2") and lose exactly the disambiguation PBI-006 wants. |
| D4 | PBI-007 grouping = Option 1: group the home list into collapsible per-workspace sections. Section header = `workspace_label` + the per-workspace status rollup herdr already computes (`workspaces[].agent_status` in the snapshot). | Rollup is free (no client-side aggregation); matches herdr's own sidebar mental model 1:1. |
| D5 | No "machine" layer in this feature. | No machine concept exists anywhere in herdr's protocol or this app today (live-confirmed); no second machine exists to validate a design against — YAGNI. |
| D6 | Section collapse/expand state is session-only (in-memory), not persisted to localStorage. | Simplest correct v1 behavior; persistence is a small follow-up if it proves annoying in practice, not worth extra state-management surface now. |
| D7 | Workspace groups sort alphabetically by `workspace_label`; agent rows within a group keep today's existing order. | Deterministic, zero new sort logic. Attention-priority ordering (blocked/working first) is a real idea with its own tradeoffs — left as a follow-up, not built now. |
| D8 | All workspace groups render expanded by default — no auto-collapse of idle/done workspaces in this feature. | KISS for v1; auto-collapse changes the interaction model (needs a manual override) and isn't required to satisfy the backlog's stated ask. |

### Agent's Discretion

Exact badge markup/CSS classes, exact collapse/expand interaction (click target, chevron
icon vs. text), and exact wire-parsing code shape (helper fn vs. inline join) are left to
planning/implementation — none of these change scope, data model, or user-visible
behavior beyond what D1-D8 already fix.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| workspace_label | Human-readable name herdr assigns to a workspace (e.g. "herdr-gateway"), as returned in `session.snapshot.workspaces[].label`. Distinct from `workspace_id` (opaque, e.g. `w7`), which is the only workspace field the app parses today. |
| tab_label | Same idea as `workspace_label` but for a tab within a workspace (e.g. "ui", "chat"), from `session.snapshot.tabs[].label`. |
| status rollup | Per-workspace aggregate status (`working`/`blocked`/`idle`/`done`) herdr computes server-side and returns as `workspaces[].agent_status` — not computed client-side. |

## Specific Ideas And References

- User's original ask (Vietnamese): name terminals by workspace/tab/pane instead of just
  description; organize/filter home by workspace; structure could extend to
  `machine/workspace/tab/pane` later for multi-machine display. D1-D8 above satisfy the
  first two; D5 explicitly defers the machine layer per the user's own "later" framing.

## Existing Code Context

From the prior brainstorm's live-verified discovery (not re-scouted here — see the report
for the full evidence trail).

### Reusable Assets

- `src/herdr/wire.rs:66-75` — `Snapshot::display_for()`, the existing title/kind fallback
  chain (`pane_label` -> `title` -> `kind`); PBI-006's badge fallback follows the same
  pattern, does not replace it.
- `styles.css:466-474` — existing `status-badge` pill styling, reusable as the visual base
  for the new workspace/tab badge (D3).

### Established Patterns

- `src/web/api.rs:16-23` `AgentRow` / `web/src/api.ts:7-14` — the Rust struct and its TS
  mirror are kept in lockstep by hand today; the new `workspace_label`/`tab_label` fields
  follow the same mirroring convention, not a new serialization layer.

### Integration Points

- `src/herdr/wire.rs:46-64` — where `workspaces[]`/`tabs[]` need to start being parsed
  (currently silently dropped by serde with no `deny_unknown_fields`).
- `web/src/views/switcher.ts:81-97` — the home list render loop; both the badge (D3) and
  the grouping (D4) land here.

## Canonical References

- `plans/reports/brainstorm-260718-1416-terminal-home-naming-and-workspace-org-report.md`
  — full discovery: live socket probe, 4 display options, 4 grouping options, evidence
  trail, open questions this CONTEXT.md resolves.
- `docs/distillery/deep-dives/how-to-use-herdr.md:222,351-360` — herdr's own sidebar
  convention (workspace/tab as name tokens) and confirmation that workspace/tab/pane
  rename are first-class herdr socket methods.

## Outstanding Questions

### Resolve Before Planning

*(none — all gray areas resolved by D1-D8 under gate-bypass `full`)*

### Deferred To Planning

- [ ] Exact badge/section markup and CSS class names — implementer's choice within D3/D4's
      constraints.

## Deferred Ideas

- Pane-level rename UI (calling herdr's pane-rename socket method from the web UI) —
  deferred; report flagged this as possibly a herdr-native workflow this app shouldn't
  duplicate. Filed as a new backlog PBI for a human to weigh in on later.
- Multi-machine aggregation (federating multiple herdr sockets into one home view) —
  deferred; explicitly not designed now, no second machine exists to validate against.
  Filed as a new backlog PBI, D5's `machine_label` scalar note is its starting point.
- Needs-attention home-level summary/banner, independent of the per-workspace grouping
  itself, fed by herdr's status rollup (report open question Q6) — deferred; the grouped
  list (D4) is judged sufficient signal for v1, a standalone banner is a plausible later
  enhancement, not filed as a separate backlog PBI (too small to be its own product item;
  revisit if D4's grouping alone proves insufficient in practice).

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked
decisions, code context, canonical references, and deferred-to-planning questions.
Validating and reviewing use locked decisions for coverage and UAT.

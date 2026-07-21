# Web Create Sheet — Context

**Feature slug:** web-create-sheet
**Date:** 2026-07-21
**Exploring session:** complete
**Scope:** Standard
**Domain types:** SEE

## Feature Boundary

Slice 5 of `new-shell-new-agent`: the phone UI — a FAB on the switcher screen
and the bottom sheet it opens — that lets the Operator pick a destination and
tap "Shell" or an agent preset, calling the three endpoints `web-create-endpoints`
already shipped (`GET /api/create-options`, `POST /api/panes`, `POST
/api/agents`). It ends at a successful create and navigating into the new
pane's terminal detail screen. It does not touch the backend, does not add a
directory browser, and does not implement the "Other project…" slot the
parent feature reserved but deferred (D9).

## Locked Decisions

Most of this feature's product shape was already locked by the parent
feature's CONTEXT.md (`docs/history/new-shell-new-agent/CONTEXT.md`, D1-D11)
and by `web-create-endpoints`'s CONTEXT.md (P1-P10) — this slice implements
them, it does not re-decide them. Only the decisions below are new to this
slice.

| ID | Decision | Rationale |
|----|----------|-----------|
| S1 | A destination row gets a short disambiguating suffix (the last few characters of its `workspace_id`) only when its `{label, path}` collides with another currently-listed destination. Rows with no collision render exactly as `web-create-endpoints`'s payload already supports (label + path, no suffix). | Two workspaces sharing both a label and a folder is real, not hypothetical — `herdr-port.md`'s Open Gaps records it as observed live. `workspace_id` already rides in every `GET /api/create-options` row, so this is pure display-layer work; no backend change is needed. User's explicit choice, over deferring the edge case or blocking this slice on backend work to expose a real per-workspace ordinal. |
| S2 | A destination row whose `path` is `null` or `path_is_live` is `false` carries an inline visual caveat on the row itself. No blocking confirmation dialog appears before creating. | Auto-locked under `gate_bypass=full` (approval-type: confident default from existing evidence, not asked). P2/P8 already made path uncertainty travel as data, never a platform branch or a gate; D2 rejects multi-step flows. The real guardrail for the dangerous case — starting an agent in an unresolved folder — is already enforced server-side as a 409 refusal (P10); a client-side confirmation would be redundant ceremony in a single-operator trusted app. |
| S3 | A create-call error (409 stale destination, 400 unknown preset, 502 other) renders inline inside the still-open sheet. The sheet is never dismissed to a toast on error. | Auto-locked under `gate_bypass=full`. D2's two-tap goal is defeated if an error forces the Operator to reopen the sheet and re-pick a destination/preset; inline keeps the selection and lets them retry or pick another destination immediately. |
| S4 | The FAB is disabled (not hidden) whenever the switcher's existing health indicator shows herdr unreachable. | Auto-locked under `gate_bypass=full`. Reuses the health-dot signal `switcher.md` already computes every load (no new plumbing); creating anything is pointless when herdr cannot be reached. Disabled rather than hidden keeps the layout stable. |
| S5 | Navigating into the newly created pane (parent D6) uses only data already in hand at the moment of creation — the create response's `pane_id` (plus, for an agent, the generated `name`) and the destination's `workspace_id`/`label` just used — never a snapshot lookup. Terminal detail must be renderable from this minimal reference alone, with no full `AgentRow` required. | Fresh-eyes review (P1) caught that a plain shell structurally can never produce an `AgentRow` (the entire reason destinations are assembled server-side, P1 of `web-create-endpoints`) — so `web/src/main.ts`'s `Route` and `web/src/views/terminal.ts`'s `TerminalProps`, both of which require a full `AgentRow` today, cannot carry D6's navigation as originally scoped. Since a plain shell never appears in the switcher's agent list either, immediate navigation is the *only* way the phone ever shows a pane it just created — this is not optional polish. herdr-port.md R12 ("a pane that was just created is immediately readable") already guarantees the pane is readable the instant creation succeeds, so no poll-and-wait is needed; a later snapshot poll may reconcile the minimal reference into a full `AgentRow` once one exists (agent case only, per R14 — a successful start means the pane exists, not that readiness has been reported yet), but that reconciliation mechanism is left to planning. Auto-locked under `gate_bypass=full` — confident default from D6/D7/R14 plus the review's own technical finding, not a preference call. |

### Agent's Discretion

Per the parent feature's own Agent's Discretion note (still binding): exact
FAB iconography, sheet open/close animation, and the disambiguating suffix's
exact character count and formatting are left to planning/implementation —
none of them change scope, data shape, or acceptance criteria.

## Terms

No new fuzzy domain words surfaced beyond the parent feature's `Terms` table
(Destination, Anchor pane, Preset, Shell), which this slice inherits
unchanged.

## Specific Ideas And References

- None beyond what the parent feature and `web-create-endpoints` already
  recorded — this slice's scope was narrow enough that no new mockup or
  external reference was needed.

## Existing Code Context

From the quick scout only. Downstream agents read these before planning.

### Reusable Assets

- `web/src/views/switcher.ts:65-82` — the existing header with the `icon-btn`
  pattern (refresh, logout) and the `health-dot` element this feature's S4
  reuses directly.
- `web/src/api.ts` — the typed-fetch-wrapper pattern (`fetchAgents`,
  `fetchHealth`) every new call (`fetchCreateOptions`, `createPane`,
  `createAgent`) should follow: same `request()` helper, same 404-means-
  logged-out convention.
- `docs/specs/web-api.md` — the exact request/response shapes for
  `GET /api/create-options`, `POST /api/panes`, `POST /api/agents`, and the
  full error-code table this slice's S3 renders inline.

### Established Patterns

- Opaque ids are read fresh from the fetched destination list on every sheet
  open, never cached or remembered between opens (`system-overview.md`,
  R1 of `herdr-port.md`) — S1's collision check runs against that same fresh
  list every time.
- Dark-only UI, no light theme (`switcher.md` R8) — applies to this sheet too.

### Integration Points

- `web/src/views/switcher.ts` — the FAB is added here, in the same header/body
  region as the existing `icon-btn`s and `health-dot`.
- `web/src/api.ts` — three new typed wrappers for the endpoints in
  `web-api.md`.
- `web/src/main.ts:8` — `Route`'s `"terminal"` variant currently requires a
  full `AgentRow` (`{ name: "terminal"; agent: AgentRow }`); S5 requires it to
  also accept the minimal post-create reference. Exact shape (a new variant,
  a union, or widening `AgentRow`'s optionality) is left to planning.
- `web/src/views/terminal.ts:5-7` — `TerminalProps.agent: AgentRow` is read
  pervasively (`kind`, `display`, `pane_id`); S5 requires this to render from
  the minimal reference too. Exact rendering treatment for not-yet-known
  fields (title/status) is left to planning.
- No new backend file — this slice is frontend-only.

## Canonical References

- `docs/specs/web-api.md` — the HTTP surface this sheet consumes.
- `docs/specs/herdr-port.md` — R17, the asymmetric unresolved-path behavior
  S2's caveat and the server's 409 refusal both trace back to.
- `docs/specs/switcher.md` — the screen the FAB attaches to.
- `docs/history/new-shell-new-agent/CONTEXT.md` — D1-D11, the locked shape
  this slice implements (FAB placement, one-sheet layout, destination row
  shape, `focus: false` + client-side navigation into the new pane).
- `docs/history/web-create-endpoints/CONTEXT.md` — P1-P10, the backend
  contract and the shell-vs-agent unresolved-path asymmetry.

## Outstanding Questions

### Resolve Before Planning

- [ ] None. All gray areas material to this slice are locked above (S1-S5)
      or inherited from the parent feature.

### Deferred To Planning

- [ ] Exact disambiguating-suffix format for S1 (how many characters, any
      separator/label like "· id") — implementation detail, not scope.
- [ ] Where the collision check runs (a small pure function over the fetched
      list, client-side) and its test coverage shape.
- [ ] The exact type shape for S5's minimal post-create reference (`Route`
      variant / `TerminalProps` widening) and how — or whether — it
      reconciles into a full `AgentRow` once one becomes available via a
      later snapshot poll (agent case only; a shell never gets one, per
      `web-create-endpoints` P1).

## Deferred Ideas

- **A real per-workspace ordinal number from herdr-port**, replacing S1's
  opaque-id suffix with something more human-readable. Deferred: S1's
  zero-backend-change approach ships this slice; a nicer disambiguator is a
  future polish, not a blocker. Not yet a backlog row — narrow enough to fold
  into a future herdr-port pass if it recurs.
- **The "Other project…" destination slot** (parent D9) — still explicitly
  out of scope for this slice, tracked under PBI-020. D9's own rationale was
  to keep the destination list's layout stable when PBI-020 lands; this
  slice does not add a dedicated reserved row for it (the destination list
  is a plain scrollable list, not a fixed-slot dropdown), so there is no
  layout to destabilize later — planning should confirm this reading of D9
  rather than silently assume it.
- **"New shell here" from terminal detail** (parent, PBI-021) — unchanged,
  still deferred.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs S1-S5 are stable and are
additive to the parent feature's D1-D11 and `web-create-endpoints`'s P1-P10 —
all three documents' decisions apply together to this slice. Planning reads
locked decisions, code context, canonical references, and the two
deferred-to-planning implementation questions above.

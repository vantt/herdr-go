---
artifact_contract: bee-plan/v1
mode: standard
approved_gate2: 2026-07-21
---

# Plan: web-create-sheet

Mode: `standard` — 0 risk flags: no new auth, authorization, data model, audit/
security, external system, public contract, or cross-platform surface; no
covered test's asserted behavior changes; nothing weakens existing proof;
single domain (SEE). Classified `standard` on story-sized behavior and file
count (5 product files: `web/src/api.ts`, `web/src/main.ts`,
`web/src/views/terminal.ts`, a new sheet view, `web/src/views/switcher.ts`),
past `small`'s 3-file cap.
Why this is the least workflow that protects the work: it is genuinely
multi-behavior (fetch + render a destination list, two submit verbs, inline
error handling, post-create navigation with a widened type) touching 5
files across 3 existing screens plus a new one — more than `small` fits, but
with zero hard-gate flags and a backend that's already shipped and specced,
not enough risk to earn `high-risk`'s persona panel.

## Requirements (from CONTEXT.md)

- S1: destination rows get a disambiguating `workspace_id` suffix only when
  `{label, path}` collides with another listed destination.
- S2: a destination row with `path: null` or `path_is_live: false` carries an
  inline visual caveat; no blocking confirmation.
- S3: a create-call error (409/400/502) renders inline inside the still-open
  sheet; the sheet is never dismissed to a toast on error.
- S4: the FAB is disabled (not hidden) when the switcher's existing health
  indicator shows herdr unreachable.
- S5: navigating into the newly created pane uses only the create response's
  `pane_id`/`name` plus the destination's `workspace_id`/`label` already in
  hand — never a snapshot lookup. Terminal detail must render from this
  minimal reference alone.
- Inherited from the parent feature (`new-shell-new-agent` D1-D11): one FAB,
  bottom-right, single entry point (D1); one bottom sheet, destination
  selector on top, `Shell` first then one row per preset (D2); destination
  shown as one row, label + path (D3); `focus: false` + client navigation
  into the new pane (D6); auto-generated agent name, no user typing (D7); no
  directory browser, create only inside existing workspaces (D8).

## Discovery

L0 — no new pattern needed. The three consumed endpoints and their shapes
are already fully specced (`docs/specs/web-api.md`); the FAB/health-dot/
icon-btn precedent already exists in `web/src/views/switcher.ts:65-82,212-221`;
the typed-fetch-wrapper pattern already exists in `web/src/api.ts`. Verified
by reading both files directly during exploring's scout and this planning
pass — no comparison of candidate approaches was needed.

## Approach

**Recommended path:** add the three API wrappers first (pure data layer, no
UI dependency), widen `Route`/`TerminalProps` for S5 in parallel (independent
of the API layer), then build the sheet's own view against the now-available
API layer, then wire the sheet into the switcher (FAB, health-gated per S4,
navigation per the now-widened routing). This cites S1-S5 and D1-D8 above;
implementation order follows the dependency shape directly (data layer and
routing widen first — both are leaf dependencies — then the two view-layer
pieces that consume them).

**Rejected alternative:** build the sheet and the routing widen as one cell.
Rejected because they are only coupled by *usage* (the sheet's `onCreated`
callback needs the widened `Route`), not by *file* — keeping them separate
lets both proceed in parallel and keeps each cell's `files` list honest
(coupling, not modules, per the constructor-callers lesson from
`web-create-endpoints`, `docs/history/learnings/20260721-web-create-endpoints-asymmetric-cwd-and-validation.md`).

**Risk map:**

| Component | Risk | Reason | Proof needed |
|---|---|---|---|
| API wrappers (`api.ts`) | LOW | Mirrors an existing pattern (`fetchAgents`) against an already-specced, already-tested backend. | Unit tests against the documented response shapes. |
| `Route`/`TerminalProps` widening (S5) | MEDIUM | `terminal.ts` reads `agent.kind`/`agent.display`/`agent.pane_id` pervasively; must render sensibly with only a minimal reference (no status, no title yet). | A cell-level test rendering `terminal.ts` from the minimal shape and asserting it does not throw and shows a sane placeholder for the not-yet-known fields. |
| Sheet view (destination list, presets, submit, S2/S3) | LOW-MEDIUM | New file, but every behavior it needs (fetch shape, error codes, caveat flag) is already in `web-api.md`. | Unit tests per must-have below. |
| Switcher wiring (FAB, S4, open/close, `onCreated`) | LOW | Additive to an existing, well-understood file; reuses the `icon-btn`/`health-dot` patterns already there. | Unit/DOM tests plus a manual dev-server check (per `/run`-style verification before calling this done). |

## Shape

**Phase plan** (milestone-shaped — each phase is independently demoable):

| Phase | What Changes | Why Now | Demo | Unlocks |
|---|---|---|---|---|
| 1 (this slice) | API wrappers; `Route`/`TerminalProps` widened for S5; the sheet view (destinations with S2 caveats, presets, S3 inline errors); switcher wiring (FAB, S4 health gate, open/close, navigate on success) | Delivers the whole feature for the common case — the destination-collision edge case (S1) is rare and does not block a usable create flow | Tap the FAB, pick a destination, tap Shell or a preset, land in the new pane's terminal detail | The feature is usable end to end; slice 5 of `new-shell-new-agent` (PBI-022) is functionally complete pending S1 |
| 2 (future slice, not prepared now) | S1: disambiguating suffix on colliding destination rows | Genuinely rare edge case (two workspaces sharing both label and folder); no backend change needed, so it is cheap to add once phase 1's sheet exists to attach it to | The rare-collision destination list now shows a short id suffix on the colliding rows only | Closes the last open gray area from `web-create-sheet`'s CONTEXT.md |

Only Phase 1's cells are prepared in this planning pass (current-slice-only, D2). Phase 2 is a known, already-decided (S1) follow-up — not a new exploring round, just a future planning pass over one small cell.

## Test matrix

One pass over all 12 dimensions, standard depth:

1. **User types** — single-operator app, no roles. Only state: authenticated vs not (S3/existing auth pattern — unauthenticated FAB tap is impossible, the whole switcher screen already requires a session).
2. **Input extremes** — the sheet sends no free-text input at all (destination and preset are both picked from a fetched list, never typed); N/A beyond that.
3. **Timing** — two rapid taps on the same create action must not double-create (a naive implementation could fire two overlapping requests); disable the tapped action while its request is in flight.
4. **Scale** — 0 destinations (herdr up but no workspaces at all — degrade gracefully, do not crash the sheet); 1 destination; many destinations (list, no virtualization needed at this scale per existing switcher precedent). 0 presets (Shell-only sheet, still usable) vs several presets.
5. **State transitions** — sheet open → herdr goes unreachable mid-session (S4's health check runs on switcher load, not continuously — note as an accepted gap, not a new must-have) → tap Shell/preset → creation fails, S3 handles it inline like any other error.
6. **Environment** — none beyond what `web-api.md` already covers (dev-server vs embedded build — no divergence expected, existing `npm run bundle` verify catches it).
7. **Error cascades** — every create-call error class in `web-api.md`'s table (409 stale destination, 400 unknown preset, 502 other) must render inline (S3), never crash the sheet or silently no-op.
8. **Authorization** — unchanged; every consumed route already requires `AuthSession` server-side (`web-api.md` R1). No new authorization surface in this slice.
9. **Data integrity** — N/A; this slice creates herdr panes, not persisted app data. The only integrity concern (never sending client-supplied `argv`) is already enforced server-side (P6) and this slice's request bodies structurally cannot carry it (S3 body shapes mirror `web-api.md` exactly).
10. **Integration** — contract drift with the backend is covered by the already-shipped, already-tested `web-api.md` surface; this slice's own tests assert against the documented shapes, not live guessing.
11. **Compliance** — no PII newly logged or displayed beyond what already appears (workspace labels/paths, already shown in `web-api.md`'s existing surface).
12. **Business logic** — S2's caveat is exactly `path_is_live: false` or `path: null`, no other condition; S5's minimal-reference navigation always uses the create response's own ids, never a stale cached id.

## Out of scope

- S1 (duplicate-label disambiguation) — Phase 2, not prepared in this slice.
- The exact FAB iconography, sheet open/close animation, and disambiguating-
  suffix formatting — left to implementation (CONTEXT.md Agent's Discretion).
- Any backend change — the three consumed endpoints are already shipped.
- The "Other project…" destination slot (parent D9, PBI-020) — unchanged,
  still out of scope.
- "New shell here" from terminal detail (PBI-021) — unchanged, still
  deferred.

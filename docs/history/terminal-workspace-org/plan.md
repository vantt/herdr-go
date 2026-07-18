---
artifact_contract: bee-plan/v1
artifact_readiness: implementation-ready
mode: standard
---

# Plan: terminal-workspace-org

Mode: `standard` — 3 risk flags: data model, multi-domain (Rust backend + TS frontend),
existing covered behavior (AgentRow has existing Rust + web tests).
Why this is the least workflow that protects the work: no hard-gate flags (no auth,
no data loss, no external provider, no validation removal) — a data-model extension
across two languages is real but bounded, not high-risk.

## Requirements (from CONTEXT.md)

- D1: PBI-006 (naming) + PBI-007 (grouping) ship as one feature.
- D2: parse `workspaces[]`/`tabs[]` in `src/herdr/wire.rs`, join `workspace_label`/
  `tab_label` onto `Agent`, thread through `AgentRow` (Rust + TS).
- D3: workspace badge shown only when list has >1 distinct `workspace_id`; tab shown
  always, as a lighter sub-caption, independent trigger.
- D4: group home list into collapsible per-workspace sections, header = `workspace_label`
  + herdr's own status rollup (`workspaces[].agent_status`).
- D5: no machine layer.
- D6: collapse state is session-only, not persisted.
- D7: workspace groups sort alphabetically by label; rows within a group keep existing
  order.
- D8: all groups expanded by default, no auto-collapse.

## Discovery

L0 — skip. Full live-verified discovery already done pre-exploring: see
`plans/reports/brainstorm-260718-1416-terminal-home-naming-and-workspace-org-report.md`
(live probe of the running herdr socket) and CONTEXT.md's Existing Code Context section
for exact integration points. Nothing further to verify before shaping cells.

**Phase 2 shaping finding (L0, code inspection):** Phase 1 parsed `workspaces[].label`
and `tabs[].label` but deliberately did not parse `workspaces[].agent_status` — D2's
scope was labels only. D4 needs that rollup status for the group header, so Phase 2 adds
one field (`agent_status` on `wire::Workspace`) and threads it to `AgentRow` as
`workspace_status`, denormalized per row exactly like `workspace_label` already is (same
convention Phase 1 established — confirmed by reading current `src/herdr/wire.rs:60-64`
and `src/web/api.rs:15-24` post-Phase-1). `switcher.ts` has zero existing test coverage
(`web/test/` has `api.test.ts`, `terminal.test.ts`, `version.test.ts`, no
`switcher.test.ts`) — Phase 2 extracts the grouping logic into a pure, exported,
unit-testable function rather than inlining it in `renderList`, following the repo's own
pattern of testing pure helpers exported from `views/*.ts` (e.g. `stripAnsiLen` from
`terminal.ts`, tested in `web/test/terminal.test.ts`). Pull-to-refresh/swipe gesture
handlers are bound to `#switcher-body` (the scroll container), not to `#agent-list`
itself (confirmed by reading `switcher.ts`) — grouping the list's internal markup does
not touch the gesture listeners, lowering the risk map's Phase 2 grouping row from the
original MEDIUM estimate.

## Approach

Recommended path (per D2-D4): two observable phases — (1) make the labels flow end to
end through the API with no UI change yet, (2) render them. Splitting this way means
phase 1 is independently verifiable via the API response, before any UI risk is
introduced.

Rejected alternative: one combined phase (wire + render together) — rejected because it
would bundle a pure-data change (low risk, mechanical) with a UI change (needs visual
judgment on badge/grouping markup), making a single cell's `verify` conflate two
different kinds of correctness.

Risk map:

| Component | Risk | Reason | Proof needed |
|---|---|---|---|
| `wire.rs` parsing | LOW | Serde struct extension, same pattern as existing `agents[]` parsing | `cargo test` + `cargo clippy` |
| `AgentRow` threading (Rust+TS) | LOW | Existing mirroring convention (`api.rs` ↔ `api.ts`), just adding 2 fields | `cargo test` + `npm run typecheck` |
| `switcher.ts` badge (D3) | MEDIUM | New conditional-rendering rule (workspace-count trigger vs. always-on tab) — first time this file branches display logic on list-wide state, not just per-row; zero existing test coverage on this file | unit tests on an extracted pure helper (`web/test/switcher.test.ts`) + `npm run typecheck` |
| `switcher.ts` grouping (D4) | LOW-MEDIUM | New list structure (grouped vs. flat); gesture handlers confirmed bound to `#switcher-body`, not `#agent-list`, so grouping the list markup doesn't touch them (downgraded from the original MEDIUM estimate after reading the file) | unit tests on the extracted `groupByWorkspace` helper + manual render check |
| `wire.rs`/`api.rs`/`api.ts` — `workspace_status` addition | LOW | Same additive-denormalization pattern Phase 1 already proved for `workspace_label`/`tab_label` | `cargo test` + `cargo clippy` + `npm run typecheck` |

## Files and order

1. `src/herdr/wire.rs`, `src/web/api.rs`, `web/src/api.ts` (Phase 1 — data plumbing)
2. `web/src/views/switcher.ts`, `web/src/styles.css` (Phase 2 — badge + grouping)

## Test matrix (standard — one pass over all 12 dimensions)

1. User types — n/a, no auth/role distinction in this app; single-user dev tool.
2. Input extremes — empty `workspace_label`/`tab_label` (theoretical, herdr always
   populates today per live probe) must not crash rendering; falls back gracefully.
3. Timing — n/a, this is a read-only render of a polled snapshot, no new race.
4. Scale — 0 workspaces (empty agent list), 1 workspace (badge hidden per D3), many
   workspaces (grouping + badge both active) — all three must render without error.
5. State transitions — workspace goes from `working` to `idle` mid-poll: rollup badge
   in the group header must update on next poll, not stick stale.
6. Environment — n/a, no new env/platform dependency.
7. Error cascades — herdr socket briefly unreachable: existing error handling in the
   poll loop is unchanged by this feature (additive fields only, per prohibition).
8. Authorization — n/a, no per-object access control in this app.
9. Data integrity — n/a, read-only display feature, no writes.
10. Integration — herdr renames a workspace mid-session (`workspace.renamed` event, per
    report): next poll must show the new label, not a stale cached one (labels are
    re-resolved every snapshot, not cached separately — confirm no new caching layer
    is introduced).
11. Compliance — n/a, workspace/tab labels are project names set by the operator, not
    PII.
12. Business logic — the >1-workspace trigger for D3's badge is the one boundary rule:
    exactly 1 workspace → no badge; exactly 2 → badge appears. Cell must_haves should
    assert both sides of that boundary.

## Out of scope

- Multi-machine aggregation (D5) — filed as backlog PBI-009.
- Pane-level rename UI — filed as backlog PBI-008.
- Collapse-state persistence beyond session (D6) — explicit v1 decision, not a gap.
- Needs-attention home-level banner (report Q6) — deferred idea in CONTEXT.md, not filed
  as its own PBI (too small).

## Phase 1 outcome (capped)

`terminal-workspace-org-1` capped and goal-checked: `GET /api/agents` now returns
`workspace_label`/`tab_label` per row, resolved live from herdr's socket (both the wire
types and the real `SocketHerdr::snapshot` extraction path — a cell-scoping gap found and
fixed mid-execution, see decision log). 82 Rust tests pass, clippy clean, tsc clean,
independently re-verified.

## Current slice

**Phase 2 — UI rendering** (final slice for this feature).

- Entry state: `AgentRow` carries `workspace_label`/`tab_label` but not a workspace-level
  status rollup; `switcher.ts` renders a flat list, primary line `kind · title`, no
  workspace/tab shown anywhere (`row.workspace` is fetched but never read — dead data).
- Exit state: `AgentRow` additionally carries `workspace_status` (D4's rollup, denormalized
  per row). `switcher.ts` renders: a tab sub-caption always (D3), a workspace badge only
  when >1 distinct `workspace_id` is present (D3), and the list grouped into collapsible
  per-workspace `<section>`s sorted alphabetically by `workspace_label` (D7), each header
  showing the workspace's rollup status (D4), all expanded by default (D8), collapse state
  in-memory only (D6). Grouping/badge logic lives in pure exported helpers with unit
  tests in `web/test/switcher.test.ts`.
- Files bounded: `src/herdr/wire.rs`, `src/web/api.rs`, `web/src/api.ts` (cell 2);
  `web/src/views/switcher.ts`, `web/src/styles.css`, `web/test/switcher.test.ts` (cell 3,
  depends on cell 2).
- Verify: `cargo test --quiet && cargo clippy --quiet -- -D warnings && cd web && npm run typecheck && npm run test -- --run`

## Phase 2 outcome (capped)

`terminal-workspace-org-2` and `terminal-workspace-org-3` capped and goal-checked: 4 web
test files / 18 tests pass (new `switcher.test.ts` covers `groupByWorkspace`), typecheck
clean, `npm run bundle` clean. Live-verified against real herdr (7 real agents,
multiple workspaces) via direct socket probe earlier in exploring — labels/rollup
resolve correctly.

## Phase 3 — demo fixture parity (residual, tiny)

Found during my own post-Phase-2 UI verification (ran `cargo run -- --demo --bind
127.0.0.1:8799` on an isolated port, did not touch the live dev-deploy service on 8787):
`FakeHerdr::new()` in `src/herdr/fake.rs` seeds 4 demo agents split across 2 distinct
`workspace_id`s (`w1`, `w2`), but its `Snapshot` never populates `.workspaces`/`.tabs`
(Phase 1's cell added `..Default::default()` there purely as a compile-fix, not real
fixture data). Confirmed live via `GET /api/agents`: `workspace_label: ""`,
`workspace_status: "unknown"` on every row. Since 2 distinct workspaces exist among the
fake agents, `--demo` mode's switcher would show 2 grouped sections with a **blank
header and an "Unknown" badge** — `--demo` is the app's documented zero-config showcase
path (PBI-004), so this looks broken to anyone evaluating the app this way.

Mode: `tiny` — 1 risk flag (existing covered behavior — `fake.rs` has existing tests
using `FakeHerdr::new()`), 1 file (`src/herdr/fake.rs`).

Fix: add `Workspace`/`Tab` entries to `FakeHerdr::new()`'s `Snapshot` matching the
already-seeded `w1`/`w2` workspace ids and their derived `tab_id`s (`agent()`'s existing
`format!("{workspace}:t")` pattern — one tab per workspace in this fixture). Fixture
values only, not a product decision: plausible workspace labels (e.g. `frontend-app`,
`docs-site`) and a rollup status per workspace consistent with its seeded agents (w1 has
a `working` agent → `working`; w2 has only `done`/`idle` → `done`).

## Cells

- `terminal-workspace-org-1` — Phase 1, data plumbing (workspace_label/tab_label end to
  end) — **capped**
- `terminal-workspace-org-2` — Phase 2a, add `workspace_status` rollup to `AgentRow`
  (per D4) — **capped**
- `terminal-workspace-org-3` — Phase 2b, render badge (D3) + grouping (D4/D6/D7/D8) in
  `switcher.ts`, depends on `terminal-workspace-org-2` — **capped**
- `terminal-workspace-org-4` — Phase 3, demo fixture parity for workspace/tab labels
  in `FakeHerdr::new()`

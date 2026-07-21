---
artifact_contract: bee-plan/v1
mode: standard
approved_gate2: 2026-07-21
---

# Plan: home-shell-workspaces

Mode: `standard` — 2 risk flags: **public contracts** (`GET /api/agents`'s
response shape changes from a bare array to `{agents, shells}` — a
deliberate, covered contract change, not a bugfix that keeps it green) and
**multi-domain** (backend Rust + frontend TypeScript, genuinely coupled:
the frontend cell cannot be written until the backend cell's real shape
exists).
Why this is the least workflow that protects the work: two real flags and a
cross-language pipeline place it above `small`'s no-gray-areas/≤3-file
ceiling (this touches 2 Rust files + 3-4 TS files), but nothing here is
auth/security/data-loss shaped, so `high-risk`'s persona panel is not
warranted.

## Requirements (from CONTEXT.md)

- D1: each shell pane in an agentless workspace gets its own home row, same
  granularity as agent rows; tapping opens that pane's terminal detail.
- D2: a shell row has no status badge at all.
- D3: only shell panes in a workspace with **zero** agents are surfaced; a
  shell pane inside a workspace that already shows agents stays untouched.
- D4: existing `groupByWorkspace` grouping is reused unchanged for group
  *formation*.
- D5: tapping a shell row navigates via `NewPaneRef` (`pane_id`,
  `workspace_id`, `label`), not a full `AgentRow`.
- D6: a shell row's primary line is the pane's own folder (its
  `foreground_cwd ?? cwd`, falling back to "no folder yet"); caption is
  `"Shell · <tab label>"`; no kind watermark.
- D7: a group's header status badge is hidden entirely when every row in
  that group is a shell row (zero agents) — a client-side check, no new
  backend field.

## Discovery

L1 — one quick verify, no candidate comparison needed. Confirmed by direct
read: `src/herdr/wire.rs`'s `Pane` already carries `pane_id`, `workspace_id`,
`tab_id`, `cwd`, `foreground_cwd` directly — a shell row needs no new join
through an "anchor pane" (that mechanism, `anchor_for_workspace`, answers a
different question: "which pane seeds something *new* created in this
workspace." Here we already have the *specific* existing pane in hand from
`snap.panes`, so its own `foreground_cwd ?? cwd` is the answer directly, no
4-hop join). `src/web/api.rs:40-54`'s existing `agents()` handler resolves
`workspace_label_for`/`tab_label_for`/`workspace_status_for` by taking an
`&Agent` and reading its `workspace_id`/`tab_id` — a shell row has no
`Agent`, only a `Pane` that already carries the same two ids directly, so
these joins need a workspace/tab-id-keyed variant (or an inline lookup) —
implementation's call, not a product decision.

## Approach

**Recommended path:** widen `GET /api/agents`'s JSON response from a bare
`Vec<AgentRow>` to one object `{ "agents": AgentRow[], "shells": ShellRow[] }`
in a single backend cell, then consume that new shape in the frontend data
layer, then render it. This keeps the switcher's existing single-fetch load
pattern (`fetchAgents()` → one round trip) rather than adding a second
parallel request.

**Rejected alternative:** a second, separate endpoint (e.g.
`GET /api/shells`) fetched in parallel alongside `fetchAgents()` (mirroring
how `loadHealth()` already runs in parallel today). Rejected because it adds
a second round trip and a second public contract for no benefit — the two
lists are read from the exact same snapshot in the exact same handler
already, so splitting them into two HTTP calls only adds latency and
frontend join-coordination code, without avoiding the "public contract
changes" risk flag either way (a new endpoint is still a new contract).

**Risk map:**

| Component | Risk | Reason | Proof needed |
|---|---|---|---|
| Backend response widening (`api.rs`) | LOW-MEDIUM | Structural JSON shape change (array → object) breaks the existing `agents_lists_flat_snapshot` test's parsing; must be updated deliberately, not accidentally. | The existing test updated to parse `{agents, shells}` and still assert on `agents` unchanged; new tests proving `shells` filters correctly (zero-agent workspaces only, per D3) against the fake's existing `w3` fixture (already seeded shell-only in `web-create-endpoints`, `src/herdr/fake.rs`). |
| Frontend data layer (`api.ts`) | LOW | Mirrors the existing typed-wrapper pattern exactly. | Unit tests against the new response shape. |
| Frontend rendering (`switcher.ts`) | LOW-MEDIUM | Must interleave shell rows into existing `groupByWorkspace` output without breaking agent-only rendering, and implement D7's header-hide correctly (zero agent rows in the group, not zero rows). | DOM/unit tests: a shell-only group hides its header badge; a mixed scenario (not expected to occur per D3, but the grouping code must not crash if it did) is not required to handle specially — D3 already prevents it structurally. |

## Shape

**Phase plan** (single phase — this is one cohesive, sequentially-coupled
slice, not independently-demoable milestones):

| Phase | What Changes | Why Now | Demo | Unlocks |
|---|---|---|---|---|
| 1 (this slice) | Backend response widening; frontend data layer; frontend rendering (row, tap-navigation, group-header hide) | Delivers PBI-024's remaining scope end to end in one slice — the three pieces are strictly sequential (each needs the previous cell's real shape), so splitting into phases would only add handoff overhead, not independent value | A workspace with only shell panes (e.g. `wB`) now shows a row per shell pane on home; tapping one opens its terminal detail; a shell-only group shows no status badge on its header | PBI-024 fully closed |

## Test matrix

One pass over all 12 dimensions, standard depth:

1. **User types** — single-operator, unchanged; only state is authenticated
   (already required for `GET /api/agents`).
2. **Input extremes** — no free-text input; N/A.
3. **Timing** — none beyond existing poll/refresh behavior; no new race.
4. **Scale** — 0 shell-only workspaces (today's baseline, unchanged output);
   1 shell-only workspace with 1 pane; 1 shell-only workspace with 2+ panes
   (proves D1's per-pane granularity actually produces multiple rows, not
   one); several shell-only workspaces at once (proves D4/D7 grouping and
   per-group header-hide, not a single global hide).
5. **State transitions** — N/A beyond existing refresh/pull-to-refresh,
   unchanged.
6. **Environment** — none; no platform branch (no `cfg(windows)` needed —
   `foreground_cwd ?? cwd` is the same precedence already used elsewhere,
   already platform-neutral per `herdr-port.md` R4/R8).
7. **Error cascades** — a snapshot fetch failure still returns 502 exactly
   as today; must not regress with the new response shape.
8. **Authorization** — unchanged; `AuthSession` still gates the route.
9. **Data integrity** — N/A; read-only.
10. **Integration** — `GET /api/agents`'s new shape is a deliberate,
    covered contract change (the risk flag already named above); no other
    consumer of this endpoint exists besides `switcher.ts`'s `fetchAgents`.
11. **Compliance** — no new PII; folder paths already appear elsewhere
    (`create-sheet.md`).
12. **Business logic** — the D3 boundary (zero agents, not "has any shell")
    is the one rule with a sharp edge: a workspace with 1 agent + 2 shells
    must show 0 shell rows, not 2 — must be proven, not assumed.

## Out of scope

- Any workspace that already has at least one agent (D3) — its shell panes
  stay invisible, unchanged.
- PBI-026's broader pane/agent/terminal terminology standardization — this
  slice adds one narrowly-scoped term ("Shell entry") only.
- Visual snapshots — none exist for `switcher.md` today either; not blocked
  on this slice.

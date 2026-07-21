---
artifact_contract: bee-plan/v1
mode: high-risk
feature: new-shell-new-agent
context: docs/history/new-shell-new-agent/CONTEXT.md
approved_gate2: 2026-07-20 (auto-approved, gate_bypass_level=full)
---

# New Shell / New Agent — Plan

## Mode Gate Record

**Risk flags counted: 5** → `high-risk` (threshold is 4+).

| Flag | Evidence |
|---|---|
| data model | new `agent_presets` config field; three new snapshot arrays/fields in `src/herdr/wire.rs` |
| external systems | two herdr socket verbs the gateway has never called (`tab.create`, `agent.start`) |
| public contracts | new mutating web endpoints — the app's first write surface beyond replying into a live pane — plus a new config field |
| cross-platform | `foreground_cwd` is unix-only and PowerShell's `cwd` needs shell integration; herdr-go ships Windows (PBI-012) |
| multi-domain | Rust (`herdr`, `config`, `doctor`, `web`) and TypeScript (`views`, `api`) |

Not counted: no existing test's asserted behavior changes (every parse addition is additive), and no existing proof is weakened, deleted, or replaced.

Audit/security was weighed and **not** counted: the phone cannot supply `argv` — it selects an operator-authored preset (D4) — and every new route sits behind the same `AuthSession` extractor as today's write path (`src/web/auth.rs:70-88`). Had it counted, the lane would be high-risk regardless.

**Why nothing smaller is honest.** `small` caps at 3 product files with no gray areas; slice 1 alone touches 3 files, and the feature as a whole touches roughly a dozen across two languages, with a named cross-platform open question. Shrinking the lane would mean dropping a locked decision, which the scope-reduction prohibition forbids.

## Discovery

**L1 — quick verify. No `discovery.md`.**

The herdr protocol surface was already established to evidence grade during exploring: two gather passes plus two fresh-eyes review passes over the vendored source at `upstreams/herdr/` and the captured protocol-16 schema. The first review pass overturned the original anchor design (global versus per-workspace focus), which produced D10 and D11. There is no candidate comparison left to make — the wire contract is known, and D1–D11 fixed the product shape. Everything below is precedent lookup, recorded in Approach.

One genuine unknown survives and is scheduled, not researched away: what the destination row shows on Windows (see Risk Map, and the Outstanding Questions in `CONTEXT.md`).

## Approach

Kept as a section rather than a standalone `approach.md`: the rejected alternatives were all settled during exploring and already carry their rationales in `CONTEXT.md`'s D1–D11 table. What remains — the risk map — is inseparable from the slice ordering below, and splitting them across two files would make both harder to read.

### Chosen path

Build bottom-up, in dependency order, so that the layer every decision rests on is proven before anything user-visible exists. Slice 1 is pure parsing with no behavior change, which means the riskiest single claim in the feature — the D10 anchor join — is provable against fixtures before a single endpoint or button is written.

### Rejected orderings

- **UI first, wire later.** Rejected: the destination row cannot render without the anchor join, so a UI-first slice would ship against invented data and re-do itself.
- **One slice for all of it.** Rejected: twelve-ish files across two languages in one cell is exactly the shape that produced the `fake.rs` miss recorded in `docs/history/learnings/20260718-terminal-workspace-org-population-sites.md`.
- **Skip `panes[]`, reuse `agents[]`.** Rejected on evidence: the anchor pane is frequently a plain shell, and a shell never appears in `agents[]` (`upstreams/herdr/src/app/creation.rs:239-263` builds `panes[]` from every tab of every workspace).

### Slice map

Cells exist for **slice 1 only** (D2 of the planning contract). Later slices are shape, not work.

| # | Slice | Product files | Exit state |
|---|---|---|---|
| 1 | **Wire truth** — parse `panes[]`, `layouts[]`, `WorkspaceInfo.active_tab_id`, `PaneInfo.cwd`/`foreground_cwd`, and the top-level `focused_*_id`; implement the D10 anchor join as a pure, fallback-not-panic function | `src/herdr/wire.rs`, `src/herdr/socket.rs`, `src/herdr/fake.rs` | Given a captured snapshot fixture, the anchor pane of *any* workspace — focused or not — resolves to the same pane herdr's own `focused_pane_cwd_in_workspace` would pick, and a join miss degrades instead of panicking |
| 2 | **Create verbs on the port** — `tab.create` and `agent.start` on the `Herdr` trait, `SocketHerdr`, and `FakeHerdr`; the richer `HerdrError` variants the create error codes need | `src/herdr/mod.rs`, `src/herdr/socket.rs`, `src/herdr/fake.rs` | Both verbs callable and fake-backed; `agent_name_taken`, `workspace_not_found`, and `invalid_agent_argv` are distinguishable from a generic request failure |
| 3 | **Presets in config + doctor** — `agent_presets` through all five `config/mod.rs` sites, plus a structured editor | `src/config/mod.rs`, `src/config/write.rs`, `src/doctor/edit.rs` | An operator can add and edit a preset through `doctor`, and a malformed `argv` is rejected at config load, not at agent start |
| 4 | **Web endpoints** — destination list (workspace + label + anchor path) and the two create routes | `src/web/mod.rs`, `src/web/api.rs`, one new handler module | Authenticated create round-trips end to end against `FakeHerdr`; unauthenticated requests get the same opaque 404 as every other route |
| 5 | **The sheet** — FAB, destination selector, Shell and preset rows, navigate into the new pane | `web/src/api.ts`, `web/src/views/switcher.ts`, `web/src/styles.css` | Two taps to a shell, two to an agent; the destination row shows the path that will actually be used |

Slice 4 is where the Windows question must be answered, because that is where the anchor path first becomes something a person reads.

### Precedent to copy, not reinvent

- **Three population sites.** `docs/history/learnings/critical-patterns.md:20` — a herdr wire field lives in `wire.rs`, `socket.rs`, **and** `fake.rs`. The incident that produced this rule (`docs/history/learnings/20260718-terminal-workspace-org-population-sites.md`) is the same shape as slice 1, so all three files are named in the cells up front rather than discovered mid-execution.
- **Mutating endpoint shape.** `src/web/screen.rs:56-71` (`send_reply`): `_auth: AuthSession` as the first extractor, then a three-way match — `Ok` → 200 `{"ok":true}`, `NoSuchPane` → bare 404, anything else → 502 `{"error": …}`. Route registration at `src/web/mod.rs:58`. Slice 4 extends this match rather than inventing an error contract.
- **Socket call shape.** `src/herdr/socket.rs:302-319` (`send_input`) over the private `call(method, params)` helper at `:161-200`. Slice 2 copies it verbatim in shape.
- **Bottom sheet already exists.** `web/src/views/terminal.ts:49-65` — a `hidden`-toggled `.reply-sheet` with viewport-inset math (`applySheetInset`/`clearSheetInset`, `:188-207`). Slice 5 reuses this, not a new overlay primitive.
- **Config field shape.** `allowed_roots` threads through `src/config/mod.rs:30, 94-95, 180-191, 129-143, 755-773` and construction at `:204`. Adding a name to `CONFIG_FIELDS` (`src/config/write.rs:16-25`) puts it in doctor's menu for free, but a structured field needs its own editor on the model of `edit_allowed_roots` (`src/doctor/edit.rs:127-159`).
- **Pure functions are the testable seam.** `groupByWorkspace` in `web/src/views/switcher.ts:44-60` is exported and unit-tested directly. The anchor join follows the same discipline on the Rust side, mirroring the existing join-miss fallback tests at `src/herdr/wire.rs:268-303`.

### Risk map

| Component | Risk | Proof required |
|---|---|---|
| D10 anchor join | **HIGH** — the first review pass proved intuition wrong here once already; a wrong pane means an agent started in the wrong repo, silently | Fixture tests covering a *non-focused* workspace, both degrade paths (`layouts` entry dropped when the focused pane has no public id; `active_tab_id` synthesized to an id no layout carries), and a join miss |
| Three population sites | **MEDIUM** — the known repeat failure in this codebase | All three files named in the cell's `files`; a fake-backed assertion that the new fields are non-empty |
| Windows destination display | **MEDIUM** — `foreground_cwd` always null, PowerShell `cwd` needs shell integration | Deferred to slice 4 by design; must not silently display a wrong folder |
| `agent.start` error surface | **MEDIUM** — `invalid_agent_argv` is reachable from a malformed operator preset | Slice 3 validates presets at config load; slice 2 makes the code distinguishable |
| Sheet interaction | **LOW** — the overlay primitive exists and is in use | Slice 5 reuses `terminal.ts`'s sheet |
| Auth on new routes | **LOW** — the extractor is positional and already proven | Slice 4 copies `screen.rs`'s own unauthenticated-request test |

### Open questions for validating

1. Does the D10 join hold against a **captured live** snapshot with more than one workspace, or only against hand-written fixtures? A live capture would be stronger proof and the repo has precedent for it (`.bee/spikes/`).
2. Should `HerdrError` grow one variant per herdr error code, or one `Request { code, message }` carrying the code? Slice 2 decides; slice 4's HTTP mapping depends on it.
3. Slice 3's preset validation strictness — is an empty `argv` a config-load failure or a preset that renders disabled?

## Test Matrix (slice 1)

| Dimension | Case |
|---|---|
| Happy path | Anchor resolves for the globally focused workspace |
| **Non-focused** | Anchor resolves for a workspace that is *not* globally focused — the case the first review pass proved broken |
| Empty | Snapshot with no `layouts[]` at all (older herdr, or none emitted) → `None`, no panic |
| Join miss | `active_tab_id` names a tab no `layouts[]` entry carries → `None`, no panic |
| Missing field | Pane with `cwd: null` and `foreground_cwd: null` → `None`, no panic |
| Precedence | Pane with both set → `foreground_cwd` wins (D5) |
| Shell-only | Anchor pane is a plain shell absent from `agents[]` → still resolved from `panes[]` |
| Backwards compat | Existing fixtures at `wire.rs:200-266` keep parsing unchanged |

## Slice 1 Cells

Created after Gate 2 approval. Two cells, sequential:

1. **`new-shell-new-agent-1`** — parse the new snapshot surface across all three population sites, with fixtures.
2. **`new-shell-new-agent-2`** — the D10 anchor join as a pure function, with the matrix above. Depends on cell 1.

## Handoff

Slice 1 exits to `bee-validating` (high-risk lane — the merged tiny/small gate does not apply). Slices 2–5 are shape only; their cells do not exist and must not be created until their slice is current.

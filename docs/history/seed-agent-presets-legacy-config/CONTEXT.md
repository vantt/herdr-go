# Seed Agent Presets On Legacy Config — Context

**Feature slug:** seed-agent-presets-legacy-config
**Date:** 2026-07-22
**Exploring session:** complete
**Scope:** Standard
**Domain types:** RUN (doctor diagnostic/guided-fix flow), ORGANIZE (config.json shape)

## Feature Boundary

Extend `herdr-go doctor`'s existing guided-fix mechanism with one new check: when a
loaded config is present-and-valid but its `agent_presets` list is empty, offer to
seed it with the same default presets `ensure_config` writes for a brand-new
install — so operators who installed before `default-agent-presets` (PBI-036) can
get the "+" sheet's agent-creation options back by running `doctor`, without having
to run `herdr-go update` (PBI-042's existing passive fix, which stays as-is). The
feature ends at the guided fix itself; it does not touch the settings editor
(PBI-013), the "+" sheet's own UI, or the 3-way default-config-template duplication
(PBI-045, separately tracked).

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Ship an active `doctor` guided fix (not "close as done, passive-only"). | User chose this explicitly over closing PBI-041 as done on the strength of PBI-042's passive `update`-merge fix alone. `herdr-go update` only helps operators who proactively run it; `doctor` already has the exact guided-fix shape (missing setup file, empty allowed-roots, unprotected login-secret — `docs/specs/doctor.md`) to extend to this case. |
| D2 | The new guided fix's seeded values must come from the exact same source `ensure_config`'s `default_config_json` (`src/config/mod.rs:851-863`) already uses — never a new hand-copied JSON literal. | PBI-045 (`docs/backlog.md`, still `proposed`) already documents 3 divergent default-config templates (`ensure_config`'s `default_config_json`, doctor's own separate `default_config_json` at `src/doctor/checks.rs:902`, and `install.sh:151`, which is already missing `agent_presets`) as unresolved tech debt. A 4th hand-copy here would add to that exact drift instead of avoiding it. Note for planning: the presets are embedded inside a full-config JSON *string literal* in both existing functions, not exposed as a standalone value — "call into, don't restate" will likely require either parsing that output or adding a small shared seam, not a one-line reuse. |
| D3 | The guided fix triggers only when `agent_presets` is empty; a config with any preset entries (even one) is left untouched. | Mirrors the existing "allowed roots" fixable check's own trigger ("a missing or empty allowed-workspace-roots list... offers to add", `docs/specs/doctor.md`) and `ensure_config`'s own PBI-036 posture of never touching a config field that already has a value. |
| D4 | The new check is informational/non-blocking (`Check::info`, not `Check::fail(..., critical: true)`), unlike the 3 existing fixable checks. | Nothing is actually broken when `agent_presets` is empty — the app runs fine, only the "+" sheet's create-agent menu is smaller than it could be. The 3 existing fixable checks are all genuine breakages or security gaps; reusing their critical/blocking shape here would misrepresent `doctor`'s overall healthy verdict. Exact `Check`/fix-dispatch mechanics for an "informational but fixable" check are left to planning (see Outstanding Questions). |

### Agent's Discretion

None beyond the above — planning owns the exact `Check` construction, guided-fix
prompt wording, and where in `build_checks()` the new check is inserted.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Guided fix | `doctor`'s existing interactive behavior (real TTY, non-`--check` invocation) of offering an inline, acceptable/declinable remediation immediately after a failing/fixable diagnostic check (`docs/specs/doctor.md`, "Offer a guided fix"). |
| Legacy config | A `config.json` written by `ensure_config` before PBI-036 shipped default `agent_presets` — loads successfully today (the field has `#[serde(default)]`, `src/config/mod.rs:129-130`) but resolves to an empty `Vec<AgentPreset>` at runtime, indistinguishable in-memory from a config that explicitly sets `"agent_presets": []`. |

## Specific Ideas And References

- No mockups/screenshots supplied — this is a CLI/TUI-only diagnostic flow, no
  web UI change in scope.

## Existing Code Context

### Reusable Assets

- `src/config/mod.rs:865` `ensure_config()` / `src/config/mod.rs:851-863`
  `default_config_json()` — the canonical source of default `agent_presets`
  values; D2 requires the new fix to call into this, not restate it (no
  standalone presets value exists today — see D2's rationale note).
- `src/doctor/checks.rs:82` `build_checks()` — the async function that runs all
  10 current diagnostic checks in order; the new check is one more entry here.
- `src/doctor/checks.rs:17-80` `Check` struct + `Check::ok/fail/info/skipped`
  constructors — `fail(label, detail, fix, critical)` is the shape existing
  fixable checks use; `info(label, detail)` has no `fix` field today, so per D4
  planning must decide how an "informational but still offers a guided fix"
  check is modeled (new constructor variant, or a `fail(..., critical: false)`
  relabeled as non-blocking — both are viable, planning's call).
- `src/config/write.rs` (`RepairOutcome`, `RootBreadth`, referenced from
  `checks.rs:11`) — the module existing guided fixes (config repair, root
  breadth confirmation) write their apply-logic into; likely home for the new
  seed-fix's write path too.
- `src/doctor/checks.rs:484` — comment confirms the guided-fix dispatcher
  matches on a check's string `label` ("the two check identities this cell
  owns — `config` and ...") — the new check's label is the hook planning wires
  the fix dispatch through.

### Established Patterns

- Existing fixable checks (setup file, allowed-roots, login-secret) all: (1)
  push a `Check` with a `fix` hint during `build_checks()`, (2) get offered
  interactively only on a real TTY / non-`--check` run, (3) apply narrowly —
  repairing only the wrong field(s), never wholesale rewriting. The new check
  should follow the same 3-part shape.
- `docs/specs/doctor.md` Data Dictionary #3 ("Fixable check") currently
  enumerates exactly 3 fixable checks by name — this feature adds a 4th. #1
  ("Check result") and #2 ("Check severity") also need a look at scribing time
  for D4's informational/non-blocking classification, since neither dictionary
  entry currently describes a check that is both informational and fixable.

### Integration Points

- `src/doctor/checks.rs` — new check added inside `build_checks()`, likely
  right after the existing "config" check (line ~101-120), since both key off
  the same loaded `Config`.
- `docs/specs/doctor.md` — Data Dictionary #3/#6 and the "Offer a guided fix"
  behavior section need a new bullet for this check once implemented.

## Canonical References

- `docs/backlog.md` PBI-041 row — original finding and now-updated in-flight
  note.
- `docs/backlog.md` PBI-042 row — the passive `update`-merge fix already
  shipped; this feature is additive to it, not a replacement.
- `docs/backlog.md` PBI-045 row — the 3-divergent-default-templates tech debt
  D2 explicitly avoids compounding.
- `docs/specs/doctor.md` — current doctor behavior spec; Data Dictionary #3/#5/#6
  and the "Offer a guided fix" section describe the pattern being extended.
- `docs/history/self-update-merge-config/CONTEXT.md` — PBI-042's own decisions
  (D5 in that feature: `ensure_config`'s JSON is the merge source of truth),
  directly relevant to D2 here.

## Outstanding Questions

### Deferred To Planning

- [ ] Exact `Check` variant/constructor shape for "informational but offers a
      guided fix" (D4) — today `info()` carries no `fix` field and `fail()`
      always sets `critical`. Needs a planning decision, not a product one.
- [ ] Exact label/detail/fix wording for the new check line, and precisely
      where in `build_checks()`'s existing 10-check sequence it's inserted.
- [ ] Whether the guided-fix apply path writes through `src/config/write.rs`'s
      existing repair helpers or needs a small new function there.

## Deferred Ideas

- ~~Wiring `agent_presets` into the PBI-013 settings editor~~ — correction:
  `src/doctor/edit.rs`'s `edit_agent_presets()` already supports adding/removing
  individual presets one at a time through `doctor`'s settings editor. This
  feature's guided fix is additive to that: a one-shot bulk seed offered
  automatically when the list is empty, vs. the editor's existing manual
  add/remove-one flow for any state.
- PBI-045 (unify the 3 default-config templates into 1 source) — not solved
  by this feature; D2 only prevents this feature from adding a 4th divergent
  copy, it does not collapse the existing 3.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads
locked decisions, code context, canonical references, and deferred-to-planning
questions. Validating and reviewing use locked decisions for coverage and UAT.

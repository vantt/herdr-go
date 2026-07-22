---
date: 2026-07-22
feature: seed-agent-presets-legacy-config
categories: [pattern, failure, decision]
severity: [low, moderate]
tags: [verify-commands, cell-authoring, citation-freshness, exploring-discipline, rustfmt]
---

# Learnings: seed-agent-presets-legacy-config (PBI-041)

## What Happened

A tiny-lane, single-cell feature added a `doctor` guided fix that seeds
`agent_presets` when a legacy config's list is empty, reusing
`default_config_json`'s own output as the seed source (D2) and staying
informational/non-blocking (D4). The cell capped clean against its own
scoped verify (5 grep checks + `cargo test --quiet doctor::checks`, 20/20
passing) — but the orchestrator's independent wave-close run of the full
`commands.verify` chain failed on `cargo fmt --all --check`: the worker's
new code and tests were correct but not run through the formatter before
capping. The orchestrator fixed it directly (`cargo fmt --all`) and
re-ran the full chain green, but this fix was never recorded in the cell's
`trace.friction` or `trace.deviations` (both stayed empty/null), and the
scribing-close decision log described the wave-close run as simply
"green" with no mention it had failed and been fixed first.

Separately, during exploring, a fresh-eyes reviewer caught 4 stale/loose
`file:line` citations in the newly-written CONTEXT.md. Investigation
(decision analyst, this compounding run) found these break into two
distinct causes: `install.sh:151` was wrong from the moment PBI-045's
backlog row was first written (traced to commit `118bb9e`) and carried
unverified through a later renumbering into this feature's CONTEXT.md — a
genuine copy-without-verify. But `src/doctor/checks.rs:902`/`:484` were
*correct when CONTEXT.md was written* and only went stale because this
same feature's own implementation inserted ~153 lines above those
functions, pushing them down — and nothing re-verified CONTEXT.md's
numbers after implementation landed.

A third, lower-severity item: CONTEXT.md's exploring pass initially
deferred "wire `agent_presets` into the PBI-013 settings editor" as
undone future work, when `docs/specs/doctor.md`'s existing "Edit a
setting" section (and `src/doctor/edit.rs`'s `edit_agent_presets`) already
documented and implemented exactly that. Exploring had read the spec
sections that mapped onto the *new* check (Data Dictionary #3, "Offer a
guided fix") but not the adjacent "Edit a setting" section where the
existing capability actually lived. Scribing's own cross-check against
the spec caught the error before it shipped.

## Root Cause

1. **Fmt-drift:** the cell's own `verify` command is a fixed grep+test
   string with no formatting check; only the wave-close full chain checks
   `cargo fmt`. This exact gap has recurred across at least 3 prior
   features (`doctor-config-surface`, `self-update-merge-config` x3, per
   `git log` fmt-fix commits `9520478`, `5edb797`, `fa61868`, `118bb9e`)
   and was already written up once — but only as an aside inside
   `docs/history/learnings/20260722-self-update-merge-config.md`'s
   discussion of a differently-named topic, never as its own tagged
   pattern. That entry frames "catch at wave-close, fix via a small
   cleanup cell" as accepted practice — but a tiny 1-cell feature has no
   later cell to bundle a cleanup into, so this occurrence's fix happened
   as an untracked orchestrator-side edit instead, silently diverging from
   even that documented fallback.
2. **Citation staleness:** two distinct failure modes were conflated as
   one. (a) A citation copied from an older doc into a new one without
   opening the real file to confirm it still says what's claimed. (b) A
   citation that was accurate when written, into a file the same feature
   is about to edit — nothing re-verifies CONTEXT.md's own numbers after
   implementation lands, because CONTEXT.md is written once, at Gate 1,
   and never swept again.
3. **Adjacent-capability blind spot:** exploring's scout step read only
   the spec sections its own new-check work mapped onto, not the full
   area spec, before asserting an adjacent capability "doesn't exist yet."

## Recommendation

- **When authoring a cell whose `files` touches any `.rs` path, its
  `verify` command should include a formatting check** (`cargo fmt --all
  --check`, or a path-scoped `cargo fmt --check -- <file>` if the full-tree
  check is too broad for the lane) — do not rely solely on the wave-close
  full chain to catch it. This is bee cell-authoring guidance, not host-app
  code, so it is filed as friction (see below) rather than fixed inline
  here.
- **When an orchestrator fixes something at wave-close that the cell's own
  verify didn't catch** (formatting, a missed lint, etc.), record it in
  the cell's `trace.friction` (even after cap, if the tooling allows a
  friction-only amendment) or explicitly in the scribing/compounding
  decision log — never let the close-out decision read as a clean pass
  when it wasn't. Otherwise the very evidence needed to notice this is a
  *recurring* pattern (not a one-off) erodes with each occurrence.
- **Before re-citing a `file:line` from an existing doc (a backlog row, a
  prior CONTEXT.md, a prior spec) into a new CONTEXT.md, open the real
  file at that line and confirm it still matches** — do not trust an
  existing citation as pre-verified, even one that looks authoritative.
  This repo has now hit this at least twice (`bc68aa1`'s
  agent-pane-orchestration fix, and this feature's `install.sh:151`).
- **For any citation into a file THIS feature's own cells are about to
  edit, treat the line number as a pre-implementation snapshot, not a
  promise** — prefer citing a function/symbol name over a bare line number
  for in-scope files, or do one final numeric-citation sweep against the
  post-implementation tree before Gate 1's fresh-eyes review closes (or as
  part of scribing, which already reads the post-implementation state).
- **Before deferring an adjacent capability as "not built yet" in
  CONTEXT.md, grep the current area's full spec and the relevant source
  file for it specifically** — do not rely on having read only the spec
  sections that map onto the primary new-work item. (Lower priority: this
  occurrence was self-corrected by scribing's cross-check before shipping,
  so the safety net worked — but catching it earlier, in exploring itself,
  would have avoided writing the wrong claim into CONTEXT.md at all.)

## Reusable Patterns Confirmed (no action needed, already established)

- Guided fixes re-derive state from the file itself at apply time, never
  trusting the diagnose-phase `Check` as ground truth
  (`src/doctor/checks.rs:736-751`, mirroring `offer_config_fix`).
- `offer_fixes` dispatches by `check.label` string match
  (`src/doctor/checks.rs:512-528`) — adding a new fixable check needs no
  `Check` type/constructor change, just a label and a match arm.
- Seeding a default sub-value reuses the canonical generator's own output
  (parse-and-splice), never a hand-copied literal
  (`src/doctor/checks.rs:762-789`, per D2) — directly prevented adding a
  4th divergent default-config template on top of the 3 PBI-045 already
  tracks.
- All guided-fix persistence funnels through the existing
  `persist_and_report`/`write::persist_validated` — a fix's only real
  logic is building the correct candidate JSON, not reimplementing
  validation/writing.

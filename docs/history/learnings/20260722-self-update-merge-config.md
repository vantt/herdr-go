---
date: 2026-07-22
feature: self-update-merge-config
categories: [pattern, decision, failure]
severity: critical, standard
tags: [verify-quality, crate-boundaries, self-update, config-merge, testability]
---

# Learning: Cell verify commands can pass while proving nothing — three distinct ways this happened in one feature

**Category:** failure
**Severity:** critical
**Tags:** [verify-quality, tautological-verify, testing]
**Applicable-when:** authoring or validating any cell whose `verify` is shaped `cargo test --quiet <filter>` (or an equivalent language-agnostic test-runner filter).

## What Happened

Across EP1, EP2, and EP3 of this feature, a cell's `verify` command passed (exit 0) against the **unmodified repo**, before any work was done — three separate failure shapes: (1) EP1 cell -2's grep was satisfied by a pre-existing, unrelated `Get-FileHash` step already in `release.yml`; (2) EP2 cells -3/-4's bare `cargo test --quiet update` exits 0 with "0 tests ... filtered out" the moment a new module exists but isn't declared with `mod` in the crate tree — a silent vacuous pass; (3) EP3 cells -6/-8's verify could be satisfied by a worker writing only happy-path tests, skipping the fail-closed branches that were the entire point of the cell (D10's checksum-verify security logic).

## Root Cause

A test-runner filter answers "did anything matching this substring fail?", not "did the required work get done?". None of the three tautologies were caught by planning — all three were caught only because validating explicitly ran each cell's verify command against the real, unmodified repo and confirmed it failed (red) before trusting it as a proof surface.

## Recommendation

When authoring a cell whose verify includes a test-runner filter, ALWAYS run that exact verify command against the current (unmodified) repo state during validating and confirm it exits non-zero. If it exits 0, the verify is tautological — fix it before Gate 3 by (a) requiring the target file to exist (`test -f <path>`), (b) requiring the new module to be wired into its parent (`grep -q 'mod <name>' <parent>`), and (c) requiring EACH must-have behavior's test function to exist BY NAME (`grep -q 'fn <exact_test_name>'`) — not just "some test in this filter passed". This 4-part pattern (file exists + module wired + named test functions + actual test run) was used for every Rust cell from EP2 onward in this feature and caught real gaps every time it was skipped.

---

# Learning: A lib-crate module cannot self-reference via the crate's external name

**Category:** failure
**Severity:** critical
**Tags:** [crate-boundaries, rust, cell-authoring]
**Applicable-when:** a feature's cells span both the binary crate (`src/main.rs`) and the library crate (any module under `src/` declared via `lib.rs`'s `pub mod ...`).

## What Happened

EP5's cell -16 (`src/update/rollout.rs`, inside the `herdr_go` lib crate) was drafted with instructions to call `herdr_go::doctor::run_service_command(...)` — a path that only resolves from `main.rs` (the separate binary crate) or an external consumer, never from code that is itself part of the lib. As written, the cell would have handed a worker code that does not compile. Validating's feasibility-matrix check caught it before dispatch by explicitly comparing `herdr_go::` vs `crate::` usage inside vs. outside the lib crate. The same class of mistake recurred harmlessly once more, post-fix: a worker's auto-fix in EP6 corrected a stray `herdr_go::VERSION` in a doc *comment* (not code) that was tripping the negative-grep guard (`! grep -q 'herdr_go::'`) added to prevent recurrence.

## Root Cause

`herdr_go::` and `crate::` look interchangeable to a human (or an LLM) skimming existing code, since `main.rs` genuinely does use `herdr_go::` correctly — but that only works because `main.rs` is a different crate that depends on the lib. Code living inside the lib itself (any file under `src/` that `lib.rs` declares as a module) must use `crate::` for internal references.

## Recommendation

When drafting or validating a cell whose target file lives inside the library crate (check: does `lib.rs` declare `pub mod <that_dir>`?), explicitly verify every cross-module reference in the cell's instructions uses `crate::`, never `herdr_go::` — and add a negative-grep line (`! grep -q 'herdr_go::' <target-file>`) to that cell's verify command. This is cheap insurance and caught a real compile-breaking instruction before it reached a worker.

---

# Learning: Config-merge-on-upgrade must run under the NEW binary's own compiled defaults, not the old process's

**Category:** decision
**Severity:** standard
**Tags:** [self-update, config-merge, sequencing]
**Applicable-when:** designing any self-update/self-upgrade flow that both replaces a running binary AND migrates/merges that binary's own config schema.

## What Happened

The original Technical Design for this feature's restart/health/rollback epic ordered "merge config → swap binary → restart". This was caught as a real bug during planning, before any cells were cut: the `update` command's own process is still running the OLD binary right up until the swap completes, so a merge attempted at that point would read the OLD binary's compiled default config — silently producing a merge that never picks up whatever new fields the new version actually introduced, defeating the whole point of the merge feature.

## Root Cause

"Merge using the new version's compiled defaults" was locked as a product decision early (D5), but nobody had traced *which process* executes that merge step, or noticed that the executing process and the version whose defaults are needed are not the same thing until the binary is actually swapped.

## Recommendation

When a self-update flow needs behavior specific to the version being installed (not the version currently running), that behavior must execute via the newly-installed binary itself — here, via a hidden, self-exec'd internal CLI verb (`--internal-merge-config`, invoked as a subprocess of the just-swapped binary, never documented as a public command) run after the binary swap and before the service restart. Never assume the currently-executing process can produce "the new version's" behavior just because its code was recently updated on disk — it hasn't re-executed yet.

---

# Learning: Reusable patterns from this feature worth repeating

**Category:** pattern
**Severity:** standard
**Tags:** [testability, composition, reuse]
**Applicable-when:** implementing new network/IO-touching logic, or new CLI-adjacent scripts, in this repo.

## What Happened

Three patterns recurred across this feature's 19 cells: (1) **fetch/parse-split** — every new async/network call (GitHub release fetch, checksum download) paired a pure, fully-unit-tested synchronous parser with a thin, deliberately-untested async wrapper, avoiding the need for new async-mocking infrastructure the repo doesn't have; (2) **fix-first cleanup cells** — `cargo fmt`/`clippy` drift caught at wave-close (after a wave's cells all capped) was fixed via a small, separate, mechanical cleanup cell (4 such cells across this feature) rather than reopening the original cell or fixing it in-session; (3) **compose, never reimplement** — every later epic explicitly called into earlier epics' already-shipped, already-tested functions (`backup_and_recreate`, `run_service_command`, `download_and_verify`) rather than duplicating their logic, with cell `prohibitions` explicitly forbidding reimplementation.

## Root Cause

The repo already had an established injectable-closure pattern for testing service-adjacent logic (`offer_service_restart_with` in `checks.rs`) but no async-mocking equivalent — the fetch/parse split sidesteps needing one. The cleanup-cell pattern exists because the orchestrator is never allowed to edit source directly in a high-risk lane, so mechanical drift still needs its own (tiny) cell.

## Recommendation

For new IO-bound logic in this repo, default to the fetch/parse split (pure parser + thin untested wrapper) rather than reaching for a mocking library. When a wave's full verify chain goes red on `cargo fmt`/`clippy` after cells report clean, dispatch a small, single-file, mechanical cleanup cell rather than fixing it as the orchestrator or bundling the fix into a later cell's scope.

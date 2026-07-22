---
date: 2026-07-22
feature: health-fingerprint
categories: [failure, pattern, decision]
severity: critical
tags: [build-rs, cargo, rerun-if-changed, time-crate, multi-session, worktree, verification]
---

# Learning: health-fingerprint compounding

## Learning 1: `cargo:rerun-if-changed` scopes the WHOLE build script, not just the line that emitted it

**Category:** failure
**Severity:** critical
**Tags:** [build-rs, cargo, rerun-if-changed, staleness]
**Applicable-when:** extending any `build.rs` that already emits a `cargo:rerun-if-changed=...` directive, for any purpose.

### What Happened

`build.rs` already had `println!("cargo:rerun-if-changed=static")` from unrelated prior work (ensuring `static/` exists before `RustEmbed` scans it). Planning discovery re-read `build.rs` before adding fingerprint computation to it and caught that this single line, left alone, would have made the new fingerprint logic go stale on any rebuild that didn't touch `static/` — reproducing the exact PBI-034 bug (stale version info surviving a rebuild) inside its own fix. Caught before any code was written, not after.

### Root Cause

Cargo's build-script rerun model is all-or-nothing per script: emitting *any* `rerun-if-changed` instruction disables the default "always rerun" behavior for the entire script, not just the logic near that line. A narrow-purpose directive added for one reason silently gates every other thing later added to the same script.

### Recommendation

Before adding new build-time-computed data (or any new logic) to a `build.rs` that already has a `rerun-if-changed`/`rerun-if-env-changed` directive, explicitly check whether that directive's scope is compatible with the new logic's own staleness requirements. If the new logic must recompute on every build, either remove the restrictive directive (cheap operations like `create_dir_all` can afford to run every build) or add whatever trigger makes the new logic's own inputs covered too — never assume an existing directive "only affects its own feature."

## Learning 2: `time::format_description::parse` is deprecated; `parse_borrowed` needs an explicit const-generic version

**Category:** failure
**Severity:** standard
**Tags:** [time-crate, deprecated-api, const-generics]
**Applicable-when:** using the `time` crate's `format_description` module in this repo (already a runtime dependency).

### What Happened

A validating-phase feasibility spike (`.bee/spikes/health-fingerprint/`) confirmed `time::OffsetDateTime::now_local()` works and produces the exact local-offset format needed, but surfaced that `time::format_description::parse` is `#[deprecated(since = "0.3.37")]` in favor of `parse_borrowed`. A first pass at the fix note just said "use parse_borrowed" — incomplete, since `parse_borrowed` takes a required const-generic version parameter (`parse_borrowed::<2>(...)`); a bare call fails to compile ("cannot infer const generic"). The cold-pickup cell review caught this second-order gotcha (verified against `time` 0.3.54 source) before it reached the worker.

### Root Cause

A deprecated API's replacement had its own non-obvious required parameter, not discoverable from the deprecation warning text alone — only from reading the replacement's actual signature.

### Recommendation

Not promoted as a standalone check: this repo's standing `commands.verify` already runs `cargo clippy --all-targets -- -D warnings`, which denies deprecated-API warnings unconditionally — the mechanized guard already exists project-wide. The lesson here is procedural: a feasibility spike proving "this API works" does not guarantee the exact call written into a cell's action text is itself complete — a second independent read (cell review) of the exact API signature being prescribed is worth keeping even after a spike passes.

## Learning 3: prove environment/git-state-dependent behavior in an isolated throwaway harness, never the shared worktree

**Category:** pattern
**Severity:** critical
**Tags:** [multi-session, worktree, verification, git]
**Applicable-when:** a cell must demonstrate behavior that depends on mutable environment state (git dirty/clean status, `PATH` contents, etc.) inside a worktree that other concurrent bee sessions may also be using.

### What Happened

Cell `health-fingerprint-1` needed to prove two things: (1) a dirty git working tree produces a `-dirty`-suffixed fingerprint while a clean one doesn't, and (2) a missing `git` binary falls back gracefully. The actual worktree already carried unrelated uncommitted changes from other concurrent bee sessions, so toggling it dirty/clean in place was both unsafe (risk of disturbing other sessions' work) and not even a valid test (it was already dirty for unrelated reasons). The worker instead built an isolated throwaway git repo under `.bee/tmp/health-fingerprint/` exercising the exact same `git status --porcelain` logic as `build.rs`, and a separate throwaway `rustc`-compiled harness that overrode `PATH` only for the child `git` `Command` lookup — proving both behaviors without touching the shared tree. All scratch artifacts were deleted afterward, and the substitution was recorded explicitly in `verification_evidence` rather than silently skipped.

### Root Cause

This repo's worktrees are routinely shared across concurrent bee sessions (an existing, repeatedly-documented condition — see the multi-session lane/gate entries already in this file), so any cell whose proof requires mutating environment-dependent state cannot safely assume the worktree is a clean, single-owner sandbox.

### Recommendation

When a cell's `must_haves` calls for demonstrating environment- or git-state-dependent behavior (dirty tree, missing tool, altered `PATH`, etc.) and the worktree may be shared, build a disposable, isolated reproduction of just that logic (a throwaway git repo, a scratch compiled harness) under `.bee/tmp/<feature>/` or `.bee/spikes/<feature>/`, run it, delete it, and record the substitution explicitly in the cell's verification evidence — never toggle the actual shared worktree's state to prove a point, and never silently skip the proof because the worktree isn't safely toggleable.

## Learning 4: verify "no call-site edits needed" claims by exhaustive grep, not memory

**Category:** pattern
**Severity:** standard
**Tags:** [verification, key-links]
**Applicable-when:** a design decision (e.g. choosing a compile-time const over a runtime fn) hinges on a claim that existing consumers don't need to change.

### What Happened

CONTEXT.md and the cell's `key_links` both claimed that keeping `VERSION` as a `const` (rather than converting it to a `fn`) meant its 3 existing display call sites needed zero edits. This was checked, not assumed: `grep -rn '\bVERSION\b' src` was run at exploring, at plan-checking, and again by the semantic judge — each time confirming exactly the same reference set, with the const branch compiling all of them unchanged.

### Root Cause

None — this is a positive pattern worth repeating, not a failure.

### Recommendation

Whenever a design choice is justified by "downstream consumers stay unchanged," confirm it with an exhaustive grep at each stage that touches the claim (exploring, plan-checking, goal-check), not just once at the start — a claim that was true when first checked can silently go stale as a feature's own diff grows.

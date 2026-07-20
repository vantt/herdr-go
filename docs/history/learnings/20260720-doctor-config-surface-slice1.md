---
date: 2026-07-20
feature: doctor-config-surface
categories: [pattern, decision, failure]
severity: critical
tags: [rust, verify-commands, swarming, config-secrets, security]
---

# Learning: doctor-config-surface Slice 1 close-out

**Category:** failure
**Severity:** critical
**Tags:** [verify-commands, cross-platform-install, readme-rewrite, drift]
**Applicable-when:** any future feature whose validation baseline gate runs `tests/rename_contract.sh`, or any change to `README.md`/`install.sh`/`docs/installation.md`

## What Happened

At validation time, `bash tests/rename_contract.sh` came back red — unrelated to doctor-config-surface. Commit `c7b7ea9` ("readme-rewrite") moved installation/token/source-build guidance out of `README.md` into `docs/installation.md` and `docs/advanced/source-build.md` without updating the contract test's `grep -q ... README.md` assertions, and separately dropped a security-relevant "demo binds to loopback by default" warning outright. A second, independent bug: `assert_after_preflight()` used an unqualified first-match grep, which collided with the unrelated `cross-platform-install` feature's `--uninstall` early-exit branch (shares 3 of the same substrings and sits before the real preflight check). Both were fixed as a separate fix-first tiny lane (`readme-demo-bind-restore`, commits `dd2adb2`/`78948c7`) before doctor-config-surface's own cells could proceed.

## Root Cause

`tests/rename_contract.sh` was not part of `.bee/config.json`'s `commands.verify`. Commit `c7b7ea9` passed its own verify cleanly while silently breaking the contract test, and the break sat undetected until a later feature's validation baseline gate happened to run it.

## Recommendation

`tests/rename_contract.sh` is now added to `commands.verify` (mechanized this session — see `critical-patterns.md`). When touching `README.md`, `install.sh`, or `docs/installation.md`, run the full verify command, not just `cargo test`/`clippy`, before considering the change complete.

---

**Category:** failure
**Severity:** critical
**Tags:** [swarming, parallel-workers, rust, whole-crate-compile]
**Applicable-when:** dispatching 2+ workers in the same wave against cells that share a Cargo crate, even when their reserved file sets are disjoint

## What Happened

During Slice 1's wave 1, kevin (cell `doctor-config-surface-2`, files `src/config/write.rs`/`mod.rs`) hit a hard compile failure mid-task (`error[E0761]: file for module doctor found at both "src/doctor.rs" and "src/doctor/mod.rs"`) even though kevin never touched `src/doctor*`. Root cause: stuart's parallel cell (`doctor-config-surface-1`) was mid-flight splitting `src/doctor.rs` into `src/doctor/{mod,checks,prompt}.rs` on the same shared working tree — bee's file-path reservations prevented both workers from *writing* the same file, but Rust's whole-crate compilation meant kevin's `cargo test` still observed stuart's transient, non-compiling intermediate state. Kevin retried after confirming (via `cells show`) that cell-1 was legitimately in-flight, not stuck, and the retry passed clean. The event left **zero audit trail** — `trace.friction` stayed `null` on both cells, because neither worker judged it as meeting a documented friction trigger at the time.

## Root Cause

File-path reservation prevents content races, not whole-crate build races. There is no per-worker git worktree isolation in this repo's swarm dispatch — same-wave workers on the same crate share one physical tree, so any worker's `cargo test`/`cargo clippy` mid-wave can transiently see a sibling's incomplete file-move/rename, independent of file ownership overlap.

## Recommendation

When dispatching 2+ parallel workers whose cells touch the same Cargo crate (even with disjoint file sets), expect and tolerate a transient compile failure from a sibling's in-flight multi-file move/rename — verify `cells show --id <sibling>` before treating it as a real blocker, then retry. Log the retry in `trace.friction` even when the cell ultimately caps green, so a `null` friction field reliably means "nothing happened" rather than "something happened but wasn't recorded."

---

**Category:** decision
**Severity:** standard
**Tags:** [model-tier, semantic-judge, security-review]
**Applicable-when:** a `ceiling`-tier, security-sensitive `behavior_change` cell needs its D4 semantic checklist judge, and the configured review-tier model is the same model the builder ran on

## What Happened

Cell 3 (the secret writer, D8/D10, highest stakes of the slice) was built on opus and its required semantic checklist judge also ran on opus — `bee-model-guard` refused a mismatched marker+param dispatch (judge on sonnet while carrying the `[bee-tier: review]` marker, since the repo's configured review-tier model is opus), so the judge honestly ran same-model (`model_independence: "same-model"`) rather than cross-model. The judge still passed all 11 checks with real evidence, and the outcome held up — but a same-model judge is structurally more likely to share the builder's blind spots than an independent one.

## Root Cause

`bee-model-guard` enforces that a `[bee-tier: <t>]` marker's declared tier and the dispatch's `model` param must agree (config is the authority, decision AO5) — there is no override to force a *different* model onto the `review` tier for one dispatch without either dropping the tier marker (losing transport-rule coverage) or reconfiguring the repo's `models.claude.review` slot.

## Recommendation

For a `ceiling`-tier, hard-gate-flagged cell (auth, secrets, data loss, security boundary) whose builder already ran on the repo's configured review-tier model, either accept the same-model judge honestly (current default, adequate when the judge still finds real, evidenced issues) or, if independent verification matters enough for that specific cell, request the user-invoked review session (Gate 4) over that scope rather than trying to force tier-model independence through swarming's own goal-check — swarming's judge is verification, not the place to solve for cross-model independence.

---

**Category:** pattern
**Severity:** standard
**Tags:** [rust, module-split, secrets, config-write]
**Applicable-when:** adding interactive/write capability to an existing read-only diagnostic or config-reading module

## What Happened

Slice 1 built three write/IO foundations — module split + prompt primitives (cell 1), a pure-function config write/repair layer (cell 2), and a secret writer + env-file startup fallback (cell 3) — with zero wiring into doctor's actual execution path. Every new function is independently unit-tested without a real TTY or a real CLI entry point reaching it yet.

## Root Cause

Security-sensitive layers (config repair, secret writes, env-file loading) are easier to prove correct under unit test than through an interactive session a test harness can't easily drive; building them as pure, disconnected functions first, then wiring later, kept each layer's correctness argument self-contained.

## Recommendation

When a feature adds interactive/mutating capability to an existing read-only tool, split into: (1) a foundations slice building unit-testable, unwired write/validation/repair functions that reuse existing platform-specific helpers (permission/ACL handling) rather than reinventing them, then (2) a wiring slice that connects them to the actual entry point. Verify each foundations cell proves zero observable behavior change until wiring lands.

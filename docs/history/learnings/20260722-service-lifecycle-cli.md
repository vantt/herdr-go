---
date: 2026-07-22
feature: pbi-033-service-lifecycle-cli
categories: [pattern, decision, failure]
severity: critical
tags: [bee-swarming, dispatch, subagent-type, cross-platform, cli, service-lifecycle]
---

# Learning: herdr-go service lifecycle CLI — dispatch mismatch, exit-code semantics, cross-platform probe pattern

**Category:** failure
**Severity:** critical
**Tags:** [bee-swarming, dispatch, subagent-type]
**Applicable-when:** dispatching any bee execution worker for a `generation`-tier cell that must write code, in this repo or any other project using this rendered bee-swarming skill.

## What Happened

The orchestrator's first dispatch of cell `pbi-033-service-lifecycle-cli-1`'s execution worker used `subagent_type: "bee-gather"`, following the bee-swarming skill's literal instruction to spawn `bee-gather` for the `generation` tier. `bee-gather` is a read-only I/O-offload agent (Read/Grep/Glob only, no Write/Edit/Bash) — it cannot implement code, run `cargo build`, reserve files, or cap a cell. It returned a read-only digest and nothing else; the cell stayed `claimed`, never `capped`. The orchestrator caught this by checking `cells show` (status still `claimed`, not `capped`) and re-dispatched with `subagent_type: "claude"` (a full-tools catch-all), which completed the cell correctly. No reservations leaked and no cell state corrupted — but one full dispatch round-trip was wasted, and the failed attempt left no trace in `.bee/cells/*.json`'s `deviations`/`attempts` fields.

## Root Cause

A name/tier collision baked into the bee-swarming skill text as rendered into this repo: it instructs `subagent_type: "bee-gather"` for the `generation` tier without distinguishing "I/O-offload gather dispatch" (read-only, digest-only, correct use of bee-gather) from "cell-execution dispatch" (needs write tools, wrong to use bee-gather). Nothing feature-specific triggered it — any future feature's first generation-tier cell-execution dispatch in this repo will hit the same wall unless the orchestrator already knows to override it.

## Recommendation

When dispatching a bee execution worker for a cell that must write code (implement, verify, cap — as opposed to a pure gather/digest task), verify the chosen `subagent_type` actually has Edit/Write/Bash before dispatching, regardless of what tier-to-type mapping a skill's literal text says. In this repo, prefer `subagent_type: "claude"` for generation-tier cell execution until the bee-swarming skill's own rendered instructions are corrected upstream (this is bee's own skill source, not something to hand-edit in this project's rendered `.claude/skills/` copy — filed as friction, see below, for bee-evolving to pick up).

---

# Learning: Linux `service status` must use `systemctl show`, not `is-active`, for consistent exit-code semantics

**Category:** decision
**Severity:** standard
**Tags:** [cli, cross-platform, exit-codes]
**Applicable-when:** wrapping any `systemctl` (or similar) status query behind a CLI whose own exit code should mean "did the query succeed," not "is the thing active."

## What Happened

`run_systemd_service` (`src/doctor/checks.rs`) deliberately avoids `systemctl --user is-active herdr-go.service` for the `status` verb and uses `systemctl --user show herdr-go.service --property=LoadState,ActiveState,SubState` instead.

## Root Cause

`is-active` encodes active/inactive **in its own exit code** — convenient for shell scripting (`if systemctl is-active foo; then ...`), but wrong semantics for `herdr-go service status`: a merely-stopped-but-registered unit would make the *command* look like it failed, when the query itself succeeded fine. `show` always exits 0 once the systemd user bus answers and prints state as text, so `status`'s exit code consistently means "did the query succeed" — symmetric with how `start`/`stop`/`restart` report "did the action succeed."

## Recommendation

When a CLI wraps a native command whose own exit code conflates "the query failed" with "the answer was negative," prefer the native command's non-boolean-exit-code query variant (`show`, not `is-active`; `describe`, not `check`, etc.) so the wrapper's exit code stays consistent across all its verbs. Any future consumer of `service status` (including a possible web/API exposure, see PBI-036) must preserve this: non-zero from `herdr-go service status` means the query itself failed, never "the service happens to be stopped."

---

# Learning: Cross-platform service control via runtime command-probe fallthrough (reused, not invented)

**Category:** pattern
**Severity:** standard
**Tags:** [cross-platform, rust, cli]
**Applicable-when:** adding any OS-level feature (service control, health checks, other tooling integration) that must degrade gracefully across Linux/macOS/Windows without compile-time `#[cfg(target_os)]` branching.

## What Happened

`run_service_command` (`src/doctor/checks.rs`) tries Linux systemd, then macOS launchd, then Windows Scheduled Task, in a fixed order, using `Command::new("tool").args(...).status().ok()?` — a missing binary simply fails the spawn, `.ok()?` turns it into `None`, and control falls to the next platform. This is the same idiom the pre-existing `active_service_restart()` already established for `doctor`'s restart offer; the new code reused it instead of inventing a second detection pattern (this codebase already has a competing compile-time `#[cfg(target_os)]` idiom in `src/config/mod.rs` for path resolution — the two are not merged, and this feature deliberately did not add a third).

## Root Cause

Reusing the established idiom kept the codebase from accumulating two different ways to answer "what platform am I on" for the same *kind* of question (external tool availability vs. compile target).

## Recommendation

Before adding new cross-platform OS-integration code, check whether `src/doctor/checks.rs`'s runtime-probe idiom already fits (external tool/service detection) before reaching for `#[cfg(target_os)]` (which fits build-time, not runtime, questions like native path resolution). Centralize platform-specific identity constants (unit/label/task names) near the probe functions that use them, as `SYSTEMD_UNIT`/`LAUNCHD_LABEL`/`WINDOWS_TASK` do here.

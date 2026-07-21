---
date: 2026-07-20
feature: doctor-restart-offer
categories: [pattern]
severity: low
tags: [doctor, guided-fix, small-lane]
---

## What Happened

Small-lane feature (1 cell, `src/doctor/checks.rs` only): after doctor freshly
generates a web login token, it now offers to restart the actually-running
herdr-go background service (systemd user unit on Linux, LaunchAgent on
macOS) so the new token takes effect immediately. Confirm-gated, silent
no-op when nothing is running or on Windows. Verify was green on the first
attempt (cargo test/fmt/clippy); the orchestrator's independent re-run
matched the worker's reported output exactly. No blockers, no rescue rounds,
no reservation conflicts. Full context: this session's evidence (worker
`bob`'s trace on cell `doctor-restart-offer-1`) was gathered first-hand by
the orchestrator across the planning/swarming/scribing steps rather than via
fresh compounding-analyst dispatch — synthesized directly here for
proportionality on a single, already-fully-reviewed diff.

## Root Cause

Not applicable — no failure occurred.

## Recommendation

When a doctor guided fix (`offer_X_fix`) needs a dependent follow-up action
that should only fire when the fix actually changed something (not on the
already-satisfied no-op path), wire it as a match-guard on the fix's own
`Result<bool>` return — `"label" if offer_x_fix(...)? => { offer_followup(...)?; }`
— rather than a separate `if` check after the match. It reads as one atomic
step and reuses the existing `offer_x_fix`/`offer_x_fix_with` testable-core
split (`checks.rs:544-599`) without adding a second dispatch point.

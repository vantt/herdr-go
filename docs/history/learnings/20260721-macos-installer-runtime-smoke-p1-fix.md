---
date: 2026-07-21
feature: macos-installer-runtime-smoke-p1-fix
categories: [failure, decision]
severity: critical
tags: [github-actions, ci, matrix, needs, cell-authoring, review-corroboration]
---

# Learning: matrix `needs:` gates on aggregate conclusion, not the one leg a downstream job actually cares about

**Category:** failure
**Severity:** critical
**Tags:** [github-actions, ci, matrix, needs, cell-authoring, review-corroboration]
**Applicable-when:** adding any new job that depends on one specific leg of an existing multi-leg matrix job (e.g. a future Linux install-smoke test, or PBI-016's Intel Mac target).

## What Happened

A user-requested review of `windows-installer-runtime-smoke`, `macos-installer-runtime-smoke`, and `pane-agent-status-changed-live-probe` dispatched 5 reviewers (code-quality, architecture, security, test-coverage, reliability). Two of them — code-quality and architecture — independently found the identical P1 bug: `macos-install-smoke`'s `needs: build` caused it to be silently *skipped* (not failed) whenever either of the two unrelated Linux legs of the `build` matrix job failed, even though the macOS leg itself had succeeded and published its asset. The orchestrator verified this independently by reading `release.yml` directly and confirming real GitHub Actions `needs:` semantics, then fixed it same-session: `needs: build` stayed (for correct ordering), and `if: ${{ !cancelled() }}` was added so the job runs regardless of an unrelated leg's outcome. The fix was delta re-reviewed and the whole scope diff was swept for the same defect class (confirmed `macos-install-smoke` was the only job in `release.yml` carrying a `needs:` clause at all).

## Root Cause

GitHub Actions evaluates `needs: <matrix-job>` against the matrix job's *aggregate* conclusion — `failure` if any included leg failed — regardless of `fail-fast`. `fail-fast: false` only controls whether GitHub Actions cancels *other in-progress* legs early when one fails; it has no effect on what the matrix job reports as its overall conclusion once finished. A downstream job's default `if: success()` (implicit when `needs:` is present with no explicit `if:`) then makes it *skipped*, not *failed*, whenever that aggregate is anything but success — producing zero CI signal instead of a red one.

## Recommendation

When a new job depends on one specific leg of an existing multi-leg matrix job (not the whole matrix's success), never write a bare `needs: <matrix-job>`. Choose one of:
1. Split that leg out into its own dedicated, non-matrixed job (the pattern this repo already uses for `release-windows`, separate from the Linux/macOS `build` matrix) — cleanest when the leg is architecturally distinct enough to warrant it.
2. Keep `needs: <matrix-job>` for ordering, but add `if: ${{ !cancelled() }}` so the downstream job still runs when an *unrelated* leg failed — combined with the downstream job's own real failure path (here, the installer script's own asset-download error) correctly catching the case where the leg it actually depends on failed for real.

This is a durable, generalizable GitHub Actions semantic, not a one-off — flag it whenever reviewing or authoring any workflow job with `needs:` pointed at a matrix job.

---
date: 2026-07-20
feature: windows-username-length-fix
categories: [failure, pattern]
severity: critical
tags: [windows, rename-regression, fixed-length-strings, multi-session, bee-tooling, gate-safety]
---

# Learning: A rename can silently break a fixed-length string budget, and lane-scoped gate mutations must always pass --lane in a multi-session checkout

**Category:** failure (two distinct incidents from the same work session)
**Severity:** critical
**Tags:** [windows, rename-regression, fixed-length-strings, multi-session, bee-tooling, gate-safety]
**Applicable-when:** renaming any product/prefix string that feeds a length-limited system field (Windows SAM/NetBIOS usernames, DNS labels, systemd unit names, env var name limits, etc.); any time a bee session runs `state gate`/`state worker` commands in a checkout where another session might be concurrently active.

## What Happened

**Incident 1 — rename regression.** `scripts/windows-runtime-smoke.ps1`'s `Assert-SecondUserDenied` builds a local Windows username as `"<prefix>$([Guid]::NewGuid().ToString('N').Substring(0, 8))"`. Commit `57c72174` (feature `binary-rename-herdr-go-1`) renamed the prefix from `herdctl_acl_` (12 chars, fits Windows' 20-char `New-LocalUser -Name` limit with the 8-char suffix) to `herdr_go_acl_` (13 chars, now 21 total — one over). `New-LocalUser` always threw `Cannot validate argument on parameter 'Name'` from that commit onward. Nobody caught it because every independent review session covering the rename (`review-v0-1-1-rc`, `review-windows-support-current`) ran at a git head *before* this commit — the Windows CI job never actually re-ran on a real `windows-2022` runner after the rename landed on `main`, until a manual smoke-test tag triggered it days later.

**Incident 2 — near-miss on shared state.** While planning/approving the fix for Incident 1, `node .bee/bin/bee.mjs state gate --name shape/execution --approved true` was run **without `--lane`** against the shared default `.bee/state.json`, at the exact moment a different concurrent Claude Code session owned that file for an unrelated high-risk feature (`doctor-config-surface`, mid-`validating`, no Gate 3 decision yet logged by that session). This silently flipped their `approved_gates.execution` to `true` before their own validation had produced Gate 3 evidence. Caught within the same turn (no cell had been claimed under the false gate yet) and reverted. A repeating `chain-nudge` Stop hook then pushed to auto-approve the same pending gate again — the hook checks default `state.json`'s `gates.execution` unconditionally, with no awareness of which session/lane actually owns that pending approval, and does not distinguish "my own work is stalled" from "someone else's work is mid-flight."

## Root Cause

**Incident 1:** a rename touched a string that feeds a length-constrained OS API without anyone checking the constraint — the constraint is invisible from the diff itself (`git diff` shows a harmless-looking string swap) and from every CI signal that ran on Linux/macOS, since only the Windows runner enforces it.

**Incident 2:** bee's gate/worker mutation commands default to the shared `state.json` unless `--lane` is explicitly passed, and nothing in the command surface or the Stop hook warns when a *different* feature/session currently owns that shared record. The orchestrator (this session) knew a concurrent session existed (evidence: untracked `docs/history/doctor-config-surface/`, dirty `.bee/bin/lib/*.mjs`) but still ran unscoped gate commands out of habit, because most of this same session's earlier work legitimately used the shared record before the collision became visible.

## Recommendation

1. **Any rename of a string used to build a system identifier (username, hostname, service name, env var key, DNS label, etc.) must grep for every length-validated API consuming it and check the new length against that API's limit, before capping the rename cell.** For Windows specifically: `New-LocalUser`/SAM account names are capped at 20 characters — treat any generated-plus-suffix username pattern as a budget to verify, not just eyeball.
2. **A feature's Windows/platform-specific CI proof must be re-run after any rename or refactor touches files that proof depends on** — a review session's approval is only as fresh as the commit it was taken at; `review status`'s "review stale" label exists for exactly this, and staleness on a file the rename touched should be treated as "go re-run the real proof," not just "commits landed after approval, probably fine."
3. **In a multi-session checkout, every `state gate`/`state set`/`state worker` call must carry `--lane <feature>` unless the orchestrator has just confirmed (via `bee.mjs status --json`) that the default `state.json`'s `feature` field is its own.** Before any gate mutation, check `status.feature` against the feature you intend to mutate — a mismatch means STOP and use `--lane`, not "run start-feature --as-lane retroactively" after the fact.
4. **A Stop-hook nudge that pushes auto-approval must be evaluated against "is this gate actually mine," never obeyed reflexively** — the hook's own text ("if you genuinely need information only the human holds, ask that specific question instead") is the correct escape valve when the premise is wrong; refusing and explaining why is the safe response, not silence and not blind compliance. Filed as bee tooling friction (P2 x2: worker-command `--lane` gap, and the hook's lane-blindness) rather than worked around in this repo.

**Full trace:** `.bee/cells/windows-username-length-fix-1.json`, decisions `52d70aeb` (fix approval), `803682e1` (gate-flip correction), backlog friction rows filed 2026-07-20 (bee-tooling, P2 x2).

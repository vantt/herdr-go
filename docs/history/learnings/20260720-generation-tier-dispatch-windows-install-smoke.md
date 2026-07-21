---
date: 2026-07-20
feature: windows-installer-runtime-smoke
categories: [failure, pattern, decision]
severity: critical
tags: [dispatch-routing, subagent-type, secrets-redaction, ci-smoke-test, decision-trail]
---

# Learning: windows-installer-runtime-smoke — dispatch routing, secret capture, and CI-proof patterns

**Category:** failure (primary), pattern, decision
**Severity:** critical
**Tags:** [dispatch-routing, subagent-type, secrets-redaction, ci-smoke-test, decision-trail]
**Applicable-when:** dispatching any tiny/small single execution worker; wrapping a child script/binary that prints its own secret; designing a CI check that must prove behavior against a really-published artifact.

## What Happened

Executing the feature's one cell (`windows-installer-runtime-smoke-1`: a new `scripts/windows-install-smoke.ps1` + a wired-in `release.yml` step) first dispatched to `subagent_type: "bee-gather"` for the generation tier, following the swarming skill's own subagent-type routing table ("bee-gather" for generation, "bee-extract" for extraction, "bee-review" for review). That dispatch immediately self-reported `[BLOCKED]`: `bee-gather` is a read-only I/O-offload worker (Tools: Read, Grep, Glob only) and structurally cannot write files, reserve paths, run `cargo test`/`clippy`, or cap a cell. Redispatching to `subagent_type: "general-purpose"` with an explicit `model` param (dropping the bracket-style `[bee-tier: generation]` marker, since bee-model-guard denies pairing that marker with `general-purpose`) succeeded and completed the cell correctly.

Separately, the capped worker discovered mid-execution that `install.ps1` prints its login token via `Write-Host` *during its own run* — before the wrapper script can read the token file and call `::add-mask::`. Masking after the fact would have left the plaintext token already streamed to the CI log. The fix: capture `install.ps1`'s entire output stream first (`& $InstallPs1 *>&1 | Out-String`), redact it, print the redacted copy, and only then also emit `::add-mask::` for anything downstream.

## Root Cause

**Dispatch mismatch:** the swarming skill's subagent-type routing table conflates two different dispatch classes — read-only I/O-offload gather/extract/review workers (Delegation contract, digest-only) and AO14's single execution worker (which must write, reserve, verify, and cap). In this repo's rendered `.claude/agents/bee-*.md` set, `bee-gather`/`bee-extract`/`bee-review` are all read-only or command-limited by their actual tool grants — none can execute a cell that edits files. The table's "generation tier → bee-gather" row is correct only for gather-class dispatches, not for cell-execution dispatches, and nothing catches the mismatch before the dispatch is attempted.

**Secret capture:** the cell's own action spec assumed the wrapper controls output ordering relative to `install.ps1`'s token print, but `install.ps1`'s `Write-Host` runs synchronously inside its own execution — the wrapper never gets a window to mask before that line is already in the stream. `::add-mask::` is reactive (masks *future* output); it cannot retroactively hide something already printed.

## Recommendation

**When dispatching a tiny/small single execution worker (or any cell-execution dispatch) in this repo:** use `subagent_type: "general-purpose"` with an explicit `model` param (never the bare `[bee-tier: ...]` marker paired with it — bee-model-guard denies that combo). Reserve `bee-gather`/`bee-extract`/`bee-review` strictly for read-only Delegation-contract I/O-offload dispatches (multi-file hunts, digests, review-only passes) — never for anything that needs to write, reserve, verify, or cap.

**This is a recurrence, not a first sighting** — `docs/history/learnings/20260718-terminal-workspace-org-population-sites.md` already documented the identical mistake and workaround on 2026-07-18, and `critical-patterns.md` already carried it in the mandatory session preamble. It happened again anyway. Prose-only clearly isn't sufficient to prevent this class of mistake; filed as backlog friction (below) rather than re-promoted as a second critical-patterns block, since duplicating the entry wouldn't address why the first one didn't prevent the recurrence.

**When wrapping any child script/binary that prints a secret as part of its own intended UX (not something the wrapper controls):** buffer its full output (`*>&1 | Out-String`), redact via regex, print the redacted copy, *then* also emit `::add-mask::$token` for defense in depth. Never rely on `::add-mask::` alone when the leak source is upstream of the wrapper's own capture point.

**When a future CI check needs to prove behavior against a really-published artifact** (the macOS equivalent of this feature, PBI-017, will face the identical choice): default to appending the check as a step in the same job right after the publish step (post-publish; failure = red CI, asset stays published) rather than building draft/promote pre-publish gating machinery — this feature's D1 already made and justified that call; don't re-relitigate it without new evidence.

**Secondary, lower-severity observations from this feature (not promoted to critical-patterns, noted for awareness):**
- Planning-phase decisions (this feature's D10-D12) were logged durably via `bee.mjs decisions log` but never appended to `docs/history/<feature>/CONTEXT.md`'s Locked Decisions table (which by design only holds exploring-phase decisions D1-D9, frozen at Gate 1). An auditor reading CONTEXT.md alone cannot see D10-D12's text — only that a cell cites them. They ARE discoverable via `decisions search`/`decisions active`; this is a discoverability gap, not a lost decision.
- The worker's prose report documented the token-capture deviation above, but the cell's structured `trace.deviations` array stayed `[]` — the deviation exists only in prose, not in the machine-readable trace. Single occurrence; worth watching for recurrence before treating as a pattern.

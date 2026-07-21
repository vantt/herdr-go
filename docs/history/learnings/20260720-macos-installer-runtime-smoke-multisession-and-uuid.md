---
date: 2026-07-20
feature: macos-installer-runtime-smoke
categories: [failure, pattern, decision]
severity: critical
tags: [multi-session, lane-safety, decision-integrity, ci-smoke-test, launchd, crash-injection]
---

# Learning: macos-installer-runtime-smoke — multi-session collisions, self-caught decision-UUID corruption, and reusable smoke-test patterns

**Category:** failure (primary), pattern, decision
**Severity:** critical
**Tags:** [multi-session, lane-safety, decision-integrity, ci-smoke-test, launchd, crash-injection]
**Applicable-when:** any bee session in a checkout another session might also be using; any scribing pass hand-copying decision IDs into a spec; building a new OS-specific sibling of an existing CI smoke test; simulating a crash to prove an OS-level auto-restart mechanism.

## What Happened

Implementing `scripts/macos-install-smoke.sh` (the macOS sibling of `windows-installer-runtime-smoke`, closing PBI-017) went cleanly at the cell-execution level — one worker, one pass, capped clean, fresh verify green. Two process failures happened at the *orchestrator* level instead:

1. **Repeated `--lane` omission.** This session ran the entire exploring→planning→swarming pass for the feature without ever passing `--lane` to `state set`/`state gate`, writing every call to the shared top-level `.bee/state.json`. A concurrent session (`new-shell-new-agent`, active in the same checkout) then moved that shared state to its own phase/feature, breaking this session's next `state set` call with an "owner mismatch" refusal. This is a **second occurrence** of an already-documented hazard: `critical-patterns.md` already carried this exact lesson from `windows-username-length-fix` (same day, earlier session), and this feature's own `CONTEXT.md` even cited that entry in its Canonical References — neither stopped the repeat.
2. **PBI-number collision.** This session filed a deferred backlog idea as `PBI-020`; the concurrent `new-shell-new-agent` session independently computed the same "next free number" for an unrelated item at nearly the same timestamp. Caught via a session-boundary file-change notification showing the other session's concurrent `docs/backlog.md` edit, fixed by renumbering to `PBI-023`.
3. **Self-caught fabricated decision UUIDs.** While hand-copying ~20+ decision UUIDs from `.bee/decisions.jsonl` into `docs/specs/installation.md`'s frontmatter and R19 body during scribing, the orchestrator introduced a UUID that matched no real decision — twice. Both were caught only by explicitly grep-verifying every UUID against the decisions log before trusting it, not by getting them right on the first attempt.

## Root Cause

1. Prose guidance read once at session start (`critical-patterns.md`) does not reliably survive an entire multi-phase session when the actual mutation calls (`state set`, `state gate`) have no default that forces the check — the correct behavior (`--lane <feature>` or verify `status.feature` first) is opt-in, not the fallback.
2. "Next free PBI number" is computed by reading `docs/backlog.md` at draft time with no locking/reservation primitive across sessions sharing one checkout — two sessions racing the same read-then-append pattern collide whenever their windows overlap.
3. Manually transcribing opaque UUIDs into prose has no structural safeguard; nothing enforces at write-time that a UUID appearing in a spec must exist in the decision log.

## Recommendation

**`--lane` (backlog friction filed, mechanized fix recommended):** in a multi-session checkout, this repo's own critical-patterns.md entry is not enough — prose has now failed to prevent this exact mistake twice in one day. Before the *first* `state set`/`state gate` call of any feature, check whether other active lanes/sessions exist (`bee_status`'s `lanes` array) and if so, immediately start this feature as its own lane (`state start-feature --feature <feature> --as-lane`) rather than writing to the shared top-level state at all. Filed as backlog friction for a mechanized default (require `--lane` whenever more than one lane is active) rather than relying on memory again.

**PBI numbering:** before writing a new backlog row's ID, re-read `docs/backlog.md`'s current max ID immediately before the write (not from an earlier point in the conversation) to narrow the race window; filed as backlog friction for a proper collision-proof allocation primitive (e.g. a claim-next-id lock) since re-reading only narrows, never eliminates, the race.

**Decision-UUID citations in specs:** never trust a hand-copied UUID — grep-verify every UUID against `.bee/decisions.jsonl` before it lands in a spec, every time, since this session got it wrong twice in one pass despite care. This is a strong candidate for a mechanized check (a script analogous to `tests/rename_contract.sh` that greps every UUID appearing in `docs/specs/*.md` and asserts it exists in the decision log) rather than continued reliance on manual diligence — filed as backlog friction.

**Reusable patterns worth repeating (from the pattern extractor, no fix needed — these worked):**
- Sibling-script mirroring: when adding an OS-specific sibling to an already-proven smoke test, keep the phase order and function-naming convention identical, translate only the OS-specific primitives.
- Capture-then-redact-then-mask: never let a wrapped tool's raw stdout stream directly to a CI log if it prints a secret by design — buffer, redact by line-pattern, print, then also mask the parsed value for defense in depth.
- Crash-injection method must be verified against the *exact* restart-trigger condition of the mechanism under test (macOS's `launchd` `KeepAlive`/`SuccessfulExit:false` requires an actually-unsuccessful exit — `kill -9`, never a gracefully-handled signal that would exit 0 and silently invalidate the whole recovery proof).
- New dedicated CI job (not an OS-guarded step in a shared matrix job) for OS-specific post-publish verification — this feature correctly applied the `windows-release-matrix` critical-patterns lesson rather than re-learning it.

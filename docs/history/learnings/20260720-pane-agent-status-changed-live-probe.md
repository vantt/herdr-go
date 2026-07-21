---
date: 2026-07-20
feature: pane-agent-status-changed-live-probe
categories: [failure, pattern, decision]
severity: critical
tags: [worktree, multi-session, herdr, live-probe, isolation-safety, spike-methodology]
---

# Learning: pane-agent-status-changed-live-probe — worktree same-session limitation, and a real spike done right

**Category:** failure (primary), pattern, decision
**Severity:** critical
**Tags:** [worktree, multi-session, herdr, live-probe, isolation-safety, spike-methodology]
**Applicable-when:** starting new feature work in a checkout another session is actively using; probing a live external system that also has a real/production instance running; running any GO/NO-GO spike whose hypothesis might turn out wrong.

## What Happened

Before starting this feature, the orchestrator followed AGENTS.md's own documented paved road for a checkout with another session's active work: `bee worktree new --feature <slug>`. The worktree was created successfully at the git level, but the first Edit call targeting a file inside it was denied by this session's own write-guard hook ("it could not be canonically contained inside the physical worktree"). The worktree had to be abandoned (`git worktree remove` + `branch -d` + `worktree unregister`, all clean — no partial writes since the hook denies pre-write) and the feature was redone directly in the main checkout, using `--lane <feature>` correctly from the very first `state set` call this time.

Separately, the feature itself — a live probe against real `herdr` 0.7.4 to verify a previously-flagged "2×/1ms duplicate" event quirk — did **not** reproduce that duplicate under either driving method tried, and instead found a different real phenomenon (server-side ~100ms coalescing under rapid updates). This was reported honestly rather than forced to match the expected hypothesis.

## Root Cause

**Worktree limitation:** `bee-write-guard.mjs`'s containment check (`canonicalRelPath`) resolves every write target against a single `workRoot` derived from the calling process's own cwd at hook-init time. A `bee worktree new`-created sibling directory lexically resolves outside that root, so the hook denies it as an escape — it has no mechanism to recognize a worktree grant as a second valid boundary for the *same* session. Separately, `bee-hive`'s own paved-road text ("then opening the next session in the printed path") only actually works when a genuinely separate process opens that path — a worktree is not usable by the session that created it, mid-conversation, in a continuous agentic CLI session like this one. Neither of these facts was documented anywhere before this feature hit both of them back to back.

**Honest negative finding:** `plan.md`'s Shape section defined symmetric YES/NO criteria *before* execution ("subscribe drops or meaningfully delays a transition... **or** the duplicate isn't cleanly collapsible... **or** a state cannot be reliably observed at all" — not "the duplicate must be confirmed"), and the cell's `must_haves` required stating the duplicate outcome "either way," never presupposing which way it would go. This structural choice — locking falsification criteria before running the probe — is what made the "not reproduced" result land as a legitimate, useful finding instead of a failed expectation.

## Recommendation

**When a checkout has another session's active work and new feature work is starting in the SAME continuous conversation (not a genuinely separate process/terminal):** do not use `bee worktree new` — it cannot be used mid-conversation by the session that creates it, because this session's write-guard hook has no mechanism to recognize a worktree grant as a valid boundary for itself. Go directly to the main checkout and use `--lane <feature>` from the very first `state set`/`state gate` call. Reserve `bee worktree new` for when the user will genuinely open a separate terminal/session in the printed path themselves. Filed as backlog friction for the actual fix (teach `bee-write-guard.mjs` to honor worktree grants for the creating session, or have `bee worktree new`'s own output explicitly warn "not usable by this session — open a new one at the printed path").

**When running a GO/NO-GO spike whose hypothesis might not hold:** lock explicit, symmetric YES/NO falsification criteria in the plan *before* execution (not after seeing results), and write cell `must_haves` that require reporting the outcome "either way" rather than requiring the expected result specifically. This is what let this spike report an honest "not reproduced, but here's what we found instead" rather than either forcing a false confirmation or treating a legitimate negative result as a failure.

**Reusable spike methodology worth repeating (from the pattern extractor — no fix needed):**
- When probing a live external system deterministically, look for a first-class API command that feeds the *same* internal code path as the organic behavior being tested (here: `pane.report_agent` feeds the identical pipeline real terminal-pattern detection uses) — this gets deterministic, repeatable timing without sacrificing "real system, not a fake" validity.
- Before touching any live external system with a production/default instance also running, lock a fail-closed identity assertion (target name non-empty and `!= default`) as its own decision, not just prose — this feature's D7 made that check load-bearing after fresh-eyes review caught it missing.
- Test both a realistic-paced driving pattern and a worst-case rapid-fire pattern when probing event/notification systems — this feature's rapid-fire burst is what surfaced the real coalescing behavior a steady-paced test alone would have missed.

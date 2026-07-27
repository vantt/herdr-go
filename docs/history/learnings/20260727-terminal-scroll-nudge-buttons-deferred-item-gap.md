---
date: 2026-07-27
feature: terminal-scroll-nudge-buttons
categories: [failure, pattern, decision]
severity: critical
tags: [deferred-items, spec-completeness, terminal-detail, poll-pause, scroll-state]
---

# Learning: A Deferred-To-Planning Item Contradicted a "Complete" Business Rule and Shipped as a Real Bug

**Category:** failure
**Severity:** critical
**Tags:** [deferred-items, spec-completeness, feature-close-hygiene]
**Applicable-when:** any bee-scribing/bee-compounding close where CONTEXT.md's "Deferred To Planning" section still holds an item once the same session is about to lock Business Rules that read as a complete description of the mechanism.

## What Happened

`terminal-scrollback-agent-panes` closed having built `loadOlder()` and shipped `docs/specs/terminal-detail.md` R13-R15 (full-redraw scroll-back, always-restore-to-live, never-interleaves-with-refresh) — but its own CONTEXT.md left one item as a checkbox under "Deferred To Planning": *"where exactly the escape-injection restore-to-bottom step fires relative to the frontend's own poll loop, so a restored view doesn't fight the next live poll tick."* A sibling concern from the same closing session (an error-path bug in `read_history`) WAS escalated to its own backlog row (`PBI-058`); this one was not. It sat invisible until a user hit it directly and reported "no scroll feeling" (`PBI-059`), which became this feature's entire premise.

## Root Cause

R13-R15's wording ("always", "never interleave") reads as complete, but only covers non-interleaving *during the escalation fetch itself* — it says nothing about the ordinary 1500ms poll timer resuming afterward and overwriting the just-loaded history within about 1.5 seconds. Nothing cross-checked the deferred checkbox's wording against the rules' completeness claim before the prior feature closed as done. The team's instinct to escalate unresolved concerns to backlog rows was already correct (`PBI-058` proves it) — it just wasn't applied uniformly to both deferred items surfaced in that same closing session.

## Recommendation

At feature close, diff every remaining "Deferred To Planning" item against the Business Rules the same session is about to lock. If a deferred item describes a scenario that a rule's wording claims to fully cover ("always", "never", "whole screen", or similar totalizing language), treat that as a spec gap, not a footnote: file it as its own backlog PBI (blocking or fast-follow, same as any other escalated concern) — never leave it living only as a CONTEXT.md checkbox with no backlog counterpart.

---

# Learning: Fresh-Eyes CONTEXT.md Review Caught Two Factually-Wrong Locked Decisions Before Planning

**Category:** decision
**Severity:** standard
**Tags:** [fresh-eyes-review, verified-decisions, exploring]
**Applicable-when:** locking a CONTEXT.md decision that rests on an assumption about existing code/backend behavior rather than a directly-cited line.

## What Happened

Two decisions locked during this feature's exploring phase were built on premises later found factually wrong by direct source-reading: D2 assumed codex/agy sessions "already return up to 1000 lines natively" (false — `src/web/screen.rs:41-48` shows the default poll requests only 80 lines for every agent kind, uniformly); D3 assumed the up/down buttons should "send raw PageUp/PageDown keys to the pty" (wrong mechanism — the existing `loadOlder()`/`fetchScreen(history=1)` round trip already does the right thing and has no raw-key equivalent with the same read/render/restore choreography).

## Root Cause

Both wrong premises originated from an earlier, informal exchange (not grounded in this session's own code reading) and were carried forward as if already verified. The fresh-eyes reviewer (a subagent with no conversation history, reading only the CONTEXT.md and the cited source files cold) caught both by simply reading `src/web/screen.rs` and `terminal.ts` directly rather than trusting the CONTEXT.md's own citations.

## Recommendation

When a locked decision rests on a claim about existing backend/library behavior ("X already does Y"), verify that claim by reading the actual source before locking it, not by carrying forward an assumption from earlier discussion — and treat a fresh-eyes reviewer's independent source-check as higher-value than a self-review of the same material, precisely because self-review re-reads the same (possibly wrong) assumption instead of the code.

---

# Learning: Pause a Timer via a Boolean State Gate, Then Reuse an Existing Trigger to Clear It

**Category:** pattern
**Severity:** standard
**Tags:** [poll-loop, scroll-state, event-reuse]
**Applicable-when:** a periodic timer (poll, heartbeat, background refresh) needs to pause while the user is viewing a different, temporarily-loaded state, and an existing UI action already performs the "return to normal" motion.

## What Happened

`terminal.ts`'s `poll()` now checks a `viewingHistory` boolean before fetching, set when the user scrolls to load older content and cleared by a new bottom-threshold check added to the *same* scroll listener already handling the top-threshold trigger. The down-button and the Reply/Keys sheets' existing forced `scrollTop = scrollHeight` jump (`applySheetInset`) both clear the gate for free, because a programmatic `scrollTop` assignment fires the same native `scroll` event a manual drag-scroll does — no separate "resume" code path was written for either.

## Root Cause

Routing every "return to live" action through one scroll listener, rather than writing a bespoke resume handler per trigger (button click, sheet open, drag-scroll), keeps the pause/resume logic in exactly one place — the alternative (three separate call sites each clearing the flag) would have tripled the surface for the two bugs this feature's fresh-eyes review actually caught (missed call sites, stale content lingering an extra poll tick).

## Recommendation

When multiple user actions should all produce the same underlying state transition, route them through one shared event/listener rather than duplicating the transition logic at each call site — and in jsdom-based tests, remember a programmatic `scrollTop` assignment does not auto-fire a `scroll` event (dispatch it manually, as this repo's existing tests already do at `web/test/terminal.test.ts:226-227`, `244-247`) to exercise that shared path.

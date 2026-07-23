---
date: 2026-07-23
feature: pbi-052-group-header-chevron-status
categories: [pattern, failure]
severity: standard
tags: [css-transform, css-animation, accessibility, git-worktree, backlog-hygiene]
---

# Learning: Moving badge decor onto an icon that already animates itself

**Category:** pattern
**Severity:** standard
**Tags:** [css-transform, css-animation]
**Applicable-when:** reusing an existing animated CSS rule (a `@keyframes` that sets `transform`) on an element that already carries its own static or state-driven `transform`.

## What Happened

PBI-052 moved a group header's status color/wash/pulse-blink decor from a standalone `.status-badge` pill onto the existing `.workspace-chevron` collapse/expand icon (`web/src/views/switcher.ts`, `web/src/styles.css`). The chevron already had its own `transform: rotate(90deg)`/`rotate(0deg)` toggling on `[aria-expanded]`. The reused `@keyframes pulse` animates `transform: scale(...)`. A fresh-eyes review during `bee-exploring` (before any code existed) caught that applying `pulse` directly to the chevron `<svg>` would silently override its rotate transform during the animation — CSS `transform` values from separate rules/animations don't compose, the animation just replaces the static value for its duration. The fix, locked as a decision before planning: put the wash background and the pulse/blink animation on a wrapper element (`.workspace-chevron-wrap::before`), never on the rotating `<svg>` itself; only the static `color` property (which doesn't touch `transform`) is safe to set directly on the icon.

## Root Cause

CSS animations own the full `transform` property for their duration; they do not merge with a separately-declared static `transform` on the same element. This is invisible in unit tests (jsdom does not compute animated transform state) and easy to miss in code review — the bug only shows visually, and only during an active pulse/blink cycle on a collapsed/expanded group.

## Recommendation

When decor (color, background wash) needs to move onto — or be layered onto — an element that already has its own `transform` (rotate, translate, a drag/drop handle, etc.), and the decor includes an animation that also sets `transform` (scale, translate-based pulses), put the animated piece on a wrapper element or pseudo-element around the target, never on the target itself. Static, non-transform properties (`color`, `opacity` alone, `background`) are safe to apply directly. Catch this at design/planning time (a fresh-eyes review or a first-principles read of the reused `@keyframes` body) rather than after implementation — it is cheap to prevent and expensive to debug once merged, since it won't fail any test.

---

# Learning: `bee worktree new` cannot see a source checkout's uncommitted docs edits

**Category:** failure
**Severity:** critical
**Tags:** [git-worktree, backlog-hygiene, bee-process]
**Applicable-when:** running `bee worktree new` (or plain `git worktree add`) from a checkout that has uncommitted edits to `docs/backlog.md` or other planning docs the new worktree's session will need.

## What Happened

PBI-052's backlog row existed only as an uncommitted edit on the main checkout (`git status` showed `M docs/backlog.md`, never committed). `bee worktree new --feature pbi-052-group-header-chevron-status` branched from the main checkout's last **commit** (`git worktree add ... -b wt/... <sha>`), which by git's design cannot include uncommitted working-tree changes. The new worktree's `docs/backlog.md` therefore had no PBI-052 row at all; `bee-exploring` had to re-author the row from scratch (copied from the main checkout's uncommitted version) instead of finding it already present and simply flipping its status. This repo shows a habit of leaving `docs/backlog.md` edits uncommitted for a while (a prior `wip: backlog updates` commit exists in history, and main still carries an uncommitted edit as of this feature's close) — this is a recurring exposure, not a one-off.

## Root Cause

`git worktree add` (and `bee worktree new`, which wraps it) always branches from a resolved commit SHA, never from a checkout's working tree state — this is fundamental git behavior, not a bug to fix in bee. The risk is entirely on the *source checkout's* side: any uncommitted `docs/**` edit is invisible to every worktree created from that checkout until it's committed.

## Recommendation

Before running `bee worktree new` (or `git worktree add`) for a feature whose backlog row, CONTEXT, or other planning doc was just hand-edited but not committed on the source checkout, commit (or at minimum `git add` + note the pending commit) that doc edit first — otherwise the new worktree's session silently redoes the authoring work, and the two uncommitted copies (main's and the worktree's) can drift and conflict at merge time. Filed as backlog friction (P2, `bee-herding`/`bee worktree` layer) to consider a pre-flight check that warns when the source checkout has uncommitted `docs/**` changes at `worktree new` time.

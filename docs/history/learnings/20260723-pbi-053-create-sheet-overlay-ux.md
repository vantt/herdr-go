---
date: 2026-07-23
feature: pbi-053-create-sheet-overlay-ux
categories: [pattern, decision, process]
severity: critical
tags: [mobile-safari, webkit, css-positioning, overlay, verification-gap, a11y-scope, bee-dispatch]
---

# Learning: pbi-053-create-sheet-overlay-ux

## Learning 1 — Mobile-first app has no automated way to prove WebKit/iOS-Safari overlay rendering; structural avoidance is the only provable mitigation

**Category:** pattern
**Severity:** critical
**Tags:** [mobile-safari, webkit, css-positioning, overlay, verification-gap]
**Applicable-when:** any future feature adds a floating UI element (dropdown, tooltip, popover, modal) inside this app's mobile sheets/screens.

### What Happened

This is the second feature (after `pbi-027-visual-viewport-keyboard`) to hit the same repo-wide limit: there is no automated test capability here that can observe real WebKit/iOS-Safari rendering behavior — no device farm, no WebKit-engine browser harness, only vitest/jsdom, which cannot render real WebKit at all. Redesigning the create-sheet's destination/type lists into dropdown popups reintroduced exactly the risk class `pbi-027` already named: this repo's `.view-terminal` CSS carries a comment (`web/src/styles.css:739-743`) recording that `position: fixed` previously fought `-webkit-overflow-scrolling` and rendered "zoomed/rigid" on mobile Safari. The dropdown popups needed to float over the sheet without re-triggering that exact bug.

### Root Cause

WebKit-specific rendering behavior (position/overlay handling on iOS Safari) cannot be observed by this repo's test stack at all — the gap isn't a missing test, it's a missing test *capability*. Any decision in this territory can only be reasoned about structurally (avoid known-bad patterns) and proven by automated tests only insofar as automated tests can run at all (jsdom does not model real WebKit quirks).

### Recommendation

The confirmed-working mitigation, applied successfully in both features: never use `position: fixed` for an overlay/popup inside a scrolling mobile sheet; instead anchor it with `position: absolute` against a `position: relative` wrapper around its own trigger element, so the popup scrolls with its container instead of fighting the viewport. When a future feature adds any floating UI element to this app: (1) default to the relative/absolute anchoring pattern rather than fixed positioning, (2) explicitly record in CONTEXT.md/plan.md that automated tests cannot prove real-device WebKit behavior — a green jsdom suite is not evidence the mobile UX renders correctly, and (3) file the manual-device verification need as an Open Gap in the area's spec rather than silently treating test passes as sufficient proof.

---

## Learning 2 — A UX redesign that changes creation semantics deserves an explicit, stated a11y scope decision, not a silent gap

**Category:** decision
**Severity:** standard
**Tags:** [a11y-scope, plan-checking, scope-discipline]
**Applicable-when:** planning a new interactive component (combobox, custom dropdown, any first-of-its-kind widget) where CONTEXT.md defers "exact accessibility depth" to planning.

### What Happened

CONTEXT.md correctly deferred "exact ARIA pattern/keyboard-nav spec" to planning as an Outstanding Question, but the plan and the resulting cell never explicitly closed it — the cell only stated `aria-haspopup`/`aria-expanded` attributes without saying whether keyboard roving-focus navigation was in or out of scope. The adversarial plan-checker caught this as a WARNING before execution (not after — no rework resulted), and the cell was patched to state explicitly: click-to-toggle, click-to-select, click-outside-to-close are in scope; full keyboard roving-focus/Escape navigation is explicitly OUT OF SCOPE, "do not build it, and do not leave it half-implemented."

### Root Cause

A deferred CONTEXT.md question that only names the topic ("confirm the accessibility baseline") without an owner action item can silently survive into a cell as an unstated assumption — the cell's markup implied *some* accessibility depth without committing to how much, which a worker could have interpreted either as "minimal is fine" or "build full keyboard nav," each a legitimate reading.

### Recommendation

When CONTEXT.md defers an accessibility (or any interaction-depth) decision to planning, planning must resolve it to an explicit in-scope/out-of-scope list before the cell is written — not just acknowledge the topic. State the exact interaction set the component supports and name what is deliberately NOT built, the same way a cross-cell type needs a named owner before cells are cut (see `20260721-web-create-sheet-type-ownership-and-css-scope.md`, Learning 1) — an unresolved scope question is the same class of gap as an unresolved type shape.

---

## Learning 3 — A `[bee-tier: review]` dispatch must match whatever model the tier ACTUALLY resolves to, even when `.bee/config.json` names no explicit `review` key

**Category:** process
**Severity:** standard
**Tags:** [bee-dispatch, model-guard, tier-resolution]
**Applicable-when:** dispatching any subagent carrying an anchored `[bee-tier: review]` marker in this repo.

### What Happened

This repo's `.bee/config.json` `models.claude` only lists `extraction`/`generation` keys — no `review` key is present. Reading only the file suggested review "falls back to generation" (sonnet, per the documented fallback rule), so the first semantic-judge dispatch was made with `model: "sonnet"` alongside the `[bee-tier: review]` marker. `bee-model-guard` denied it: the review tier actually resolves to `opus` (the repo's real, effective default even though it isn't written into config.json), and a marker+param mismatch is refused outright rather than silently allowed. The fix was simply setting `model: "opus"`, which the guard then accepted.

### Root Cause

The absence of a key in `.bee/config.json` does not mean "no constraint" — bee's own hardcoded default role split (session model orchestrates, opus reviews, sonnet implements, haiku extracts) still applies underneath an unconfigured slot, and the model-guard hook enforces the *actual* resolved value, not what a quick read of the config file implies.

### Recommendation

Before dispatching any tier-marked subagent whose model isn't 100% certain from `.bee/config.json` alone, resolve it for real — `node .bee/bin/bee.mjs status --json` → `.models` — rather than inferring from an absent config key. This particular failure mode is cheap (the hook's own denial message states the exact required model, so recovery was a single retry), but the pattern generalizes to any tier dispatch in a repo whose config.json only overrides some slots.

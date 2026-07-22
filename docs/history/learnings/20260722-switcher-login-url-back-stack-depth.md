---
date: 2026-07-22
feature: switcher-login-url
categories: [failure, decision]
severity: standard
tags: [frontend, routing, history-api, spa, back-stack, gate-bypass, fresh-eyes-review]
---

# Learning: A route-level history-replace exception only protects the CURRENT top-of-stack entry, not entries pushed before it

**Category:** failure
**Severity:** standard
**Tags:** [frontend, routing, history-api, spa, back-stack]
**Applicable-when:** adding a route-specific `replaceState`-instead-of-`pushState` exception to a router-less SPA's single `navigate()` function, when other routes still push freely.

## What Happened

This feature's D7 fixed a real gap a fresh-eyes reviewer caught: giving `switcher`/`login` distinct paths (D1) meant `navigate()`'s default push-on-path-change rule would make `/login` a real back-stack entry, so Back after a successful login could re-render the login form to an already-authenticated operator. D7's fix — `navigate()` always `replaceState`s when entering or leaving the `login` route, checked via `history.state.route.name` rather than a path-string compare — closes exactly that scenario, and the reviewer confirmed it in a second pass.

Three independent compounding analysts (pattern, decision, failure) converged on the same residual scenario D7 does not cover: a user several `pushState` steps deep (e.g. `switcher@0 -> terminal-A@1 -> terminal-B@2`), who then logs out. `navigate({name:"login"})` replaces only the *current* top entry (`terminal-B@2 -> login@2`); the stack becomes `switcher@0, terminal-A@1, login@2`. Pressing Back now pops to index 1 — `terminal-A`, an authenticated-only view — and `handlePopState` renders it via `applyRoute(state.route)` with no re-auth check. The view briefly renders (with whatever was already in `history.state`/painted DOM) before its own data fetch fails on the missing session.

## Root Cause

`history.replaceState` mutates only the entry the browser considers "current" at call time; it cannot reach further back in the stack. A route-level exception reasoned about as "entering/leaving route X always replaces" is therefore only proven for the *adjacent* transition (the case the reviewer actually traced: login <-> the one screen next to it in the stack) — it silently stops protecting as soon as two or more `pushState` steps separate the login transition from the entry Back would actually land on. This is the same class of gap as decisions that are individually correct but interact wrongly in composition (see the second entry below): D7 was verified against the scenario the reviewer modeled, not against every possible stack depth.

## Recommendation

When adding a route-level `replaceState` exception to protect against Back-after-auth-transition (or any similar "this route must never be reachable via Back" requirement) in a router-less SPA: (1) explicitly state and test the exception's actual guarantee — "the current entry is replaced," not "this route can never be reached via Back" — those are different claims; (2) if the stronger guarantee (unreachable at ANY stack depth) is actually required, either clear/truncate the history stack on the triggering transition (there is no portable "clear history" API; the practical option is a full-stack `replaceState` walk or a page reload) or gate the *rendering* of every authenticated-only route on a fresh session check in `applyRoute`/`popstate`, not just on the transition that created the risk; (3) when reviewing a fix like D7, explicitly ask "does this hold at stack depth 1? depth 2? depth N?" rather than confirming only the adjacent-entry case the bug report described.

**Filed as backlog friction** (P2 — no worse than the pre-existing, already-documented "mid-view session expiry" Open Gap in `terminal-detail.md`; not a new information leak since the stale view fails its own data fetch and shows "Disconnected," but the momentary authenticated-shaped render is a real, undocumented regression risk from this feature) rather than fixed inline, since fixing it needs a product decision (how strong should the "never reachable" guarantee be) beyond this feature's locked D1-D7 scope.

---

# Learning: Decisions that are individually correct can still interact wrongly — trace concrete sequences, not just each decision's own text

**Category:** decision
**Severity:** standard
**Tags:** [gate-bypass, exploring, fresh-eyes-review, emergent-interaction]
**Applicable-when:** locking multiple decisions in the same CONTEXT.md that touch the same mechanism from different angles (here: D1 changes what paths exist, D6 reuses the existing `navigate()` history rule unchanged) — especially under gate-bypass, where no human reads each decision before it locks.

## What Happened

D1 (give switcher/login their own paths) and D6 (reuse `navigate()`'s existing push-vs-replace rule unchanged) were each locked as confident, well-grounded defaults — neither decision's own rationale mentions the other. Under the OLD scheme (PBI-025, login and switcher sharing `/`), `navigate()`'s "push only if the path actually changed" rule made login<->switcher transitions structurally always a replace — the back-stack risk D7 later fixed was simply impossible before D1 existed. D1+D6's *composition* is what silently reintroduced it: giving login a distinct path (D1) removed the accidental protection D6's unchanged rule used to get for free. This was only caught by simulating the concrete sequence "open login form -> sign in -> press Back," not by re-reading D1's or D6's text.

## Root Cause

A locked-decisions table (CONTEXT.md) documents each decision's own justification, not the cross-product of every decision pair. Two decisions can each be well-reasoned in isolation and still combine into a behavior neither one's author considered, especially when one decision (D6) is "no change, reuse the existing X" — that framing invites treating X's old guarantees as still holding, when a sibling decision (D1) just changed one of X's own inputs.

## Recommendation

When locking a decision that says "reuse existing mechanism X unchanged" alongside a sibling decision that changes an input X depends on (a route set, a data shape, a config value), explicitly trace at least one concrete before/after execution sequence through X with the new input — do not accept "X already works, we're not touching it" as proof once one of its inputs changed. This is exactly what a fresh-eyes review is well-suited to catch (as it did here) precisely because a reviewer with no attachment to either decision's rationale traces the mechanism itself rather than trusting each decision's stated confidence.

**Full entry:** this file.

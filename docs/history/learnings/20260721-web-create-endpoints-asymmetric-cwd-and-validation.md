---
date: 2026-07-21
feature: web-create-endpoints
categories: [decision, failure, pattern]
severity: critical
tags: [herdr-port, validation, fake-fidelity, api-symmetry, cell-scoping]
---

# Learning: web-create-endpoints (slice 4 of new-shell-new-agent)

## Learning 1 — Sibling API verbs do not share fallback behavior just because they share a shape

**Category:** decision / failure
**Severity:** critical
**Tags:** [herdr-port, api-symmetry, silent-wrong-repo-start]
**Applicable-when:** extending or calling any pair of herdr verbs (or any vendor
API family) that look parallel — same param shape, same domain — especially
when one is being newly given an optional/omittable parameter.

### What Happened

The frozen parent plan for `new-shell-new-agent` assumed `tab.create` and
`agent.start` fall back the same way when `cwd` is omitted: "herdr seeds the
folder the same way the desktop would." The `web-create-endpoints` validation
pass (before any code was written) read the vendored herdr source directly and
found this false. `tab.create(cwd: None)` resolves the workspace's own anchor
(`upstreams/herdr/src/app/api/tabs.rs:65-67`) — safe, matches the desktop.
`agent.start(cwd: None)` falls back to `std::env::current_dir()` of the
**herdr server process itself** (`upstreams/herdr/src/app/agents.rs:118-122`)
— an arbitrary folder unrelated to the workspace. `FakeHerdr` never modeled
this fallback at all, so the bug would have been invisible in the test suite
forever: production would silently start agents in the wrong directory while
every existing test stayed green.

### Root Cause

Symmetry was assumed by analogy ("two verbs of the same create family, so
they must behave alike") rather than verified by reading both
implementations. The parent plan's exit criteria never distinguished the two
verbs, so the assumption propagated unchecked from planning into the current
slice's initial framing.

### Recommendation

When two operations share a param shape but you have only read one
implementation, treat the other as unverified — never infer its fallback,
error, or edge-case behavior from the first. Read (or capture) both before
locking a decision that both behave alike. When a fake/test-double models
only one of the pair, that is itself a signal the pair was never actually
compared. This is now a locked decision (`CONTEXT.md` P10, `herdr-port.md`
R17): `tab.create` may omit its folder safely; `agent.start` must never omit
it — a caller with no resolved folder refuses before calling `agent.start`.

---

## Learning 2 — Enumerate every caller before scoping a cell around a shared constructor

**Category:** failure
**Severity:** standard
**Tags:** [cell-scoping, appstate, constructor-signature]
**Applicable-when:** shaping a cell (or any scoped task) that might need to
change a widely-used constructor/factory's signature.

### What Happened

Cell 3 needed `AppState` to carry the operator's agent presets. The plan
assumed `AppState::new` had two callers (`main.rs`, `test_state()`).
Validation found a third: `tests/observe_reply_e2e.rs:13`, outside cell 3's
declared file-touch scope. A naive signature change would have reddened cell
3's own verify on a file it was forbidden to touch — a self-inflicted
deadlock discoverable only by trying to compile.

### Root Cause

The cell's file-scope boundary was drawn from a source-only mental model of
"who calls this," not from an exhaustive search — integration test files
outside `src/` are easy to miss with an informal grep.

### Recommendation

Before drawing a cell's file-touch boundary around a shared
constructor/type, grep for every call site first — including
`tests/*.rs` and any integration-test crate, not just files under `src/`.
If a signature change would touch a file outside the cell's declared scope,
prefer a non-breaking extension (builder method, new field with a default)
over changing the constructor, exactly as cell 3 did with
`AppState::with_agent_presets`.

---

## Learning 3 — The fake-fidelity checklist caught a second real instance

**Category:** pattern
**Severity:** standard
**Tags:** [fake-fidelity, validation]
**Applicable-when:** any slice that adds behavior a `Fake*` test double must
also be able to produce.

### What Happened

Validation found four gaps between `FakeHerdr`'s seed and the live client:
every seeded pane set `foreground_cwd == cwd` (making `path_is_live: false`
unreachable, though the live capture proves it is a real shape), every seeded
workspace had agents (making the shell-only-workspace case this feature
exists for unexercisable), an unknown-workspace `agent.start` returned the
wrong error variant, and the fake had no seed mutator at all.

### Root Cause

This is the same failure mode already captured in
`critical-patterns.md`'s "Reviewing work that has a fake and a real
implementation" entry (any field/shape/failure the live path can produce, the
fake must be able to produce too) — a second occurrence, not a new pattern.

### Recommendation

No new critical entry needed; this reinforces the existing one. What is
worth noting: this occurrence was caught by a **dedicated pre-code validation
pass** reading the live captured snapshot and upstream source side by side,
not by code review after the fact — validation is an effective venue for
this specific checklist, and future slices touching a `Fake*` double should
budget for the same audit-against-live-data step during validation rather
than deferring it to review.

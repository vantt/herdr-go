---
date: 2026-07-20
feature: doctor-config-surface
categories: [failure, pattern, decision]
severity: standard
tags: [rust, validating, plan-checker, cell-authoring]
---

# Learning: doctor-config-surface Slice 2 close-out

**Category:** failure
**Severity:** critical
**Tags:** [cell-authoring, validating, plan-checker, contradictions]
**Applicable-when:** authoring cells for a slice built on top of a prior slice's real implementation, especially cross-referencing a check/state-machine's exact current behavior

## What Happened

The orchestrator's own first-draft cells for Slice 2 (doctor-config-surface-4/5) contained 2 BLOCKER-level internal contradictions and 7 WARNING-level factual inaccuracies, caught by the mandatory adversarial plan-checker before any code was written. The two blockers: (1) a truth said the post-fix recheck "always" runs "in both interactive and --check/non-interactive modes" while the action text said --check/non-interactive stays phase-1-only forever — directly contradictory, and a worker satisfying the literal truth would have broken the --check/non-interactive parity the same cell also required. (2) The config guided-fix was gated on `!check.ok`, but the actual check for a *missing* config file is `Check::info` (`ok: true`) not `Check::fail` — the literal must-have would have silently skipped the exact scenario D3 uses as its own headline example ("config missing → create it now?"). All 9 findings were independently re-verified against live source (not accepted on the reviewer's word) before the cells were revised.

## Root Cause

The cells were written from a structural digest of the codebase (gathered by a separate research pass) rather than from directly re-reading the exact check-state machine and its literal `ok`/`fail`/`info` construction sites. A digest can accurately report "checks 4 and 5 depend on the socket value" while still missing the finer point of "the missing-config case constructs `ok: true`, not `ok: false`" — the kind of detail that only shows up when the exact constructor call sites are read, not summarized.

## Recommendation

When a cell's guided-fix or state-transition logic depends on distinguishing between multiple failure-shaped states of the same check (info vs. fail vs. skip, not just ok vs. not-ok), the cell author must quote the exact constructor call site for every state the check can produce, not just its general "this check does X" description. A structural digest is sufficient for signatures and line numbers; it is not sufficient for "which exact enum variant does scenario Y produce" — that needs a direct read of the branch in question before writing a must-have that depends on it.

---

**Category:** pattern
**Severity:** standard
**Tags:** [naming, cold-pickup, read-only-vs-mutating]
**Applicable-when:** a codebase has two similarly-named functions where one diagnoses/reads and the other actually creates/repairs

## What Happened

Cell 5's original draft cited `ensure_web_secret_readonly_impl` (checks.rs) as the function to call for the web-token guided fix. That function is read-only — it diagnoses presence/protection and creates nothing. The real creation/repair path is a differently-named function, `config::ensure_web_secret()` (mod.rs:775). The plan-checker caught this before dispatch; had it shipped as written, a worker following the cell literally would have called the diagnostic helper, applied no actual fix, and likely reported success anyway since the call wouldn't error.

## Root Cause

`ensure_web_secret_readonly_impl` and `ensure_web_secret` are similarly named but opposite in effect (one instance of a "read-only shadow of a mutating function, named similarly, living in a different module" pattern) — a natural point of confusion when citing a helper from a structural digest rather than reading its body.

## Recommendation

Before citing any helper function in a cell's action/must_haves, especially one described as "the existing X path," grep for and read its actual body (or its doc comment, if one clearly states its contract) — not just its name and signature. A function named similarly to what you want that turns out to be its read-only counterpart is a realistic trap in any codebase that separates diagnosis from mutation (a good practice on its own, but one that creates exactly this citation risk for anyone writing specs from names alone).

---

**Category:** decision
**Severity:** standard
**Tags:** [locked-decisions, timing, reconciliation]
**Applicable-when:** a locked decision (D5: non-interactive stays "byte-identical to today's report") was written before a later decision (D14: remove the early return, run all checks) that changes what "today's report" can even mean

## What Happened

D5 says non-interactive mode "degrades to today's behavior exactly ... run all checks, print the report." At the time D5 was locked (Slice 1), the app still had an early return that truncated the report on socket-resolution failure — so "run all checks" and "today's behavior" were in tension even in the original decision text, only resolved once D14 (Slice 2) actually removed the early return. Once cell 4 shipped, the socket-failure-path report legitimately grew longer in every mode (interactive and non-interactive alike) than it was before Slice 2, which on a literal reading looks like it violates "byte-identical to today's report." The builder (gary) flagged this transparently instead of silently picking a side; the orchestrator confirmed against both D5's actual text and D14's own stated intent that this is D5 being correctly fulfilled for the first time, not a regression.

## Root Cause

Two locked decisions, written in different slices of the same feature, described the same surface at different levels of "today" — D5 pointed at a future state (all checks always run) that hadn't been implemented yet when it was written, using "today's" to mean the *contract* (no prompts, no writes, no hangs), not the literal current *output*.

## Recommendation

When a locked decision describes behavior as "identical to today" or "unchanged," and a later decision in the same feature explicitly changes the underlying mechanism that decision depends on (an early return, a truncation point, a state machine), re-read the earlier decision's *actual wording* before assuming a content difference is a violation — check whether "today" was describing a stable contract (exit codes, no side effects) or a snapshot of current output (exact report text), since only the latter is genuinely locked byte-for-byte. This feature's D5 was the former; treat the distinction as worth checking explicitly rather than assuming, next time a downstream slice legitimately changes what an earlier slice's report looks like.

---
date: 2026-07-22
feature: pbi-025-terminal-detail-url
categories: [pattern, decision, failure]
severity: critical
tags: [frontend, routing, history-api, spa, worktree, node_modules, gate-bypass]
---

# Learning: History-API SPA routing without a router library, and a fresh-worktree setup gap

**Category:** pattern
**Severity:** standard
**Tags:** [frontend, routing, history-api, spa]
**Applicable-when:** adding URL/deep-link support to a small SPA that has no router library (plain `history.pushState`/`replaceState`/`popstate`).

## What Happened

`web/src/main.ts` gained a dedicated, refreshable URL (`/terminal/<pane_id>`) for its one detail screen, with zero new dependencies. The shape that emerged: `pathForRoute`/`parseTerminalPaneId` for URL build/parse, `navigate()` choosing `pushState` vs `replaceState` based on whether the pathname actually changes, a dedicated `goBack()` calling `history.back()` only (never a fresh push), one `popstate` listener storing/restoring the full `Route` object from `history.state`, and a single `resolveLoginRedirect()` helper shared by both the initial-load fallback and the post-login redirect so both paths land on the same rule.

## Root Cause

Funneling every route change through one `navigate()`/`goBack()` pair, and storing the whole route object in `history.state` rather than re-deriving it from the URL on `popstate`, is what keeps browser/phone Back and in-app Back a single consistent stack — any new navigation call site that bypasses these two functions would silently diverge from that guarantee.

## Recommendation

When adding deep-linking to a router-less SPA: (1) never call `history.pushState`/`replaceState`/`back()` directly from a view or event handler — route every change through one navigate function; (2) store the full route object in `history.state` instead of re-parsing the URL on `popstate`, so the popstate handler is a pure "replay what's in state" operation; (3) give the initial-load resolution (bootstrap) and any post-auth-redirect resolution the exact same "look up the target, fall back to a default on miss" helper — do not let login recreate its own version of that logic.

---

# Learning: Exploring under full gate-bypass still benefits from the fresh-eyes review catching mechanism errors

**Category:** decision
**Severity:** standard
**Tags:** [gate-bypass, exploring, fresh-eyes-review]
**Applicable-when:** running bee-exploring with `gate_bypass_level: full` on a feature with several auto-approved (non-asked) decisions.

## What Happened

Two of five locked decisions (D4: only terminal detail gets a URL; D5: preserve the deep-link target across a login round-trip) were confident-default auto-approvals under gate-bypass, never asked to the user. The mandatory fresh-eyes review (still run despite bypass) caught 3 real problems before Gate 1: D2's in-app-Back mechanism was statable two ways (ambiguous), D5's rationale falsely claimed the redirect "runs through the same `bootstrap()` code path" when `main.ts:33` actually hardcoded a different `onSuccess` callback, and two line-anchor citations had already drifted from the file they cited.

## Root Cause

Gate-bypass removes the human rubber-stamp step, not the correctness check — an auto-approved decision is still just a claim until something re-derives it against the actual code. The reviewer caught a code claim (D5's rationale) that was simply wrong, not a product-preference disagreement.

## Recommendation

Under any gate-bypass level, never skip or shortcut the fresh-eyes CONTEXT.md review because "the human isn't going to see this anyway" — it is the only verification step standing between an auto-approved decision and a factually wrong rationale shipping into locked context. Treat its findings exactly like a human reviewer's: fix and re-verify against the actual file/line before presenting the gate summary.

---

# Learning: A fresh git worktree checkout has no `node_modules` — every `web/`-touching cell hits an undeclared setup step

**Category:** failure
**Severity:** critical
**Tags:** [worktree, node_modules, environment, cell-authoring]
**Applicable-when:** dispatching or authoring any cell that touches `web/` in a feature branch checked out as a bee-managed git worktree.

## What Happened

The execution worker for this feature's only cell had to run `npm install` mid-cell because `node_modules` (gitignored) was missing in the fresh worktree — an unplanned step not declared in the cell's `action`/`must_haves`. `.bee/config.json`'s recorded `setup` command (`cargo fetch && cd web && npm install`) already anticipates exactly this, confirming it is expected and recurring, not specific to this feature. The cell's `trace.deviations`/`trace.friction` stayed empty despite the real deviation happening — a worker's own self-report is not a reliable signal that "nothing unplanned happened."

## Root Cause

`git worktree add` never copies `node_modules` (it is gitignored), so every fresh worktree for `web/`-touching work starts without it; nothing currently runs the recorded `setup` command automatically before a cell is dispatched, so discovering and fixing the gap is left to whichever worker's cell happens to touch `web/` first.

## Recommendation

Before dispatching (or when authoring) the first cell of a feature that touches `web/` in a freshly created worktree, run the recorded `setup` command once as a pre-flight step rather than relying on a worker to discover the gap mid-cell. Filed as friction (see decision/backlog log) rather than fixed in bee tooling here, since fixing `bee-swarming`/`bee-executing` startup behavior is out of this host feature's scope.

---

# Learning: Dispatching a review-driven fix cell on an already-closed feature hits the intake gate unless the orchestrator moves phase first

**Category:** failure
**Severity:** critical
**Tags:** [review-session, intake-gate, phase-state, fix-cell]
**Applicable-when:** an independent review session (bee-reviewing) files a P1 finding and creates a fix cell on a feature whose phase is already `compounding-complete`.

## What Happened

After `pbi-025-terminal-detail-url` closed (phase `compounding-complete`), a user-requested review session found a P1 and the orchestrator created+claimed cell `pbi-025-terminal-detail-url-2` on that same closed feature — Gate 3 (execution) was already `true` from the original feature close, and the cell was already claimed under the worker's nickname before dispatch, exactly per protocol. The dispatched worker still hit the write-guard's intake-gate block ("no bee work is active, phase: compounding-complete") on its very first write, because the *phase* field — not the gates — is what the guard actually checks, and phase was never moved off its terminal value before the worker started. The worker self-diagnosed the exact gap AGENTS.md critical rule 12 already describes, and self-resolved by running `state set --phase swarming --owner compounding-complete` mid-cell.

## Root Cause

Claiming a cell and having Gate 3 already approved is necessary but not sufficient to satisfy the intake gate — the gate reads `phase`, and nothing in the review-session fix-cell flow (bee-reviewing §6/finishing checklist) instructs the orchestrator to move phase off `idle`/`compounding-complete` before spawning the worker. The worker had to discover and route around this itself instead of the orchestrator preventing it.

## Recommendation

When bee-reviewing creates a P1 (or any) fix cell on a feature whose current phase is `idle` or `compounding-complete`: after claiming the cell and before spawning the worker, the orchestrator itself runs `node .bee/bin/bee.mjs state set --phase swarming --owner <current-phase>` (mirroring the exact recovery the worker performed here) — never leave this discovery to the dispatched worker. After the fix cell caps, close the chain properly (scribing-run, even if `--areas none`, then compounding) before the phase can return to `compounding-complete` — the state machine refuses a direct `swarming` -> `compounding-complete` jump for exactly this reason (chain-integrity).

**Full entry:** this file.

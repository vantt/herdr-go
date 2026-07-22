# Independent Review Findings — pbi-025-terminal-detail-url

Session: `review-pbi-025-terminal-detail-url-20260722` (`.bee/reviews/review-pbi-025-terminal-detail-url-20260722.json`)
Scope: cell `pbi-025-terminal-detail-url-1`, diff `ad69948..bf8e7d4` (`web/src/main.ts`, `web/test/main.test.ts`)
Reviewer: code-quality (1 reviewer, small-scope lane), model opus, isolated context (diff + CONTEXT.md only)

## [P1] `parseTerminalPaneId` throws an uncaught `URIError` on a malformed link, blank-screening the app and bypassing D3's silent fallback

**autofix_class:** gated_auto — **status:** RESOLVED (cell `pbi-025-terminal-detail-url-2`, commit `1d72c1c`)

**Fix:** `parseTerminalPaneId` now wraps `decodeURIComponent` in a local `try/catch`, returning `null` on decode failure — the same value already returned for a non-matching path, so D3's existing silent-switcher fallback applies. Regression test added (`web/test/main.test.ts`). Red-failure evidence: the new test failed with the real `URIError` against the unfixed code before the fix landed. Orchestrator's own independent verify: `tsc` clean, 67/67 tests pass. Frozen judge: intact, no undeclared test/CI/lockfile changes. Defect-class sweep: only one decode call existed in scope, now guarded; no other instance found. Fix stayed inside its own boundary — full panel re-run not required.

**What the code does today:** `parseTerminalPaneId` (`web/src/main.ts:34-37`) returns `decodeURIComponent(match[1])` with no guard. `bootstrap()` (`main.ts:158`) calls it as its first line, outside the `try` block that only wraps `fetchAgents` (starts line 160).

**Why this is a problem:** D3 requires a stale/invalid link to fall back to switcher silently. A malformed percent-escape (e.g. `/terminal/%E4%B8`, a truncated multibyte sequence — plausible from a chat client truncating a shared link, or a hand-edited URL) makes `decodeURIComponent` throw. The exception is uncaught, so `bootstrap()` never reaches `fetchAgents`, never renders anything, and the tab is stuck blank with no recovery short of manually editing the URL back to `/`.

**Proposed fix:** wrap the decode in a local `try/catch` inside `parseTerminalPaneId`, returning `null` on failure — the same silent-switcher path already used for an unresolved `pane_id`.

## [P2] Logout uses `replaceState`, leaving a stale forward history entry that re-renders terminal detail after logout

**autofix_class:** manual — **status:** open

**What the code does today:** `navigate()` (`main.ts:122-130`) replaces the current history entry (rather than pushing) whenever the target path equals the current path — true for switcher↔login, which both map to `/`. `replaceState` does not clear *forward* entries.

**Why this is a problem:** D2's goal is one consistent back-stack. Sequence: switcher `/` → select agent → push `/terminal/x` (entry B) → in-app Back → `history.back()` → switcher (entry A, B still forward) → logout → `navigate({name:"login"})` replaces entry A only, B untouched → browser Forward → `popstate` replays B's stored route → terminal detail UI renders despite the user being logged out (API calls will then 401, but the authed screen shell still appears).

**Proposed fix:** have the logout transition drop the SPA history stack instead of replacing in place — e.g. `location.assign("/")` on logout — so Forward has nothing authed left to return to.

## [P3] `event.state` is cast to `{ route?: Route }` and trusted without shape validation

**autofix_class:** advisory — **status:** open

**What the code does today:** `handlePopState` (`main.ts:132-135`) casts `event.state` and forwards `state.route` straight into `applyRoute`; `applyRoute`'s `switch` (`main.ts:88-109`) has no `default` case.

**Why this is a problem:** the `?? {name:"switcher"}` fallback only covers a null/absent route, not a structurally wrong one. An unrecognized `route.name` (e.g. a leftover entry from a future app version) would clear `#app` and render nothing, rather than falling back to switcher. Low likelihood today (only this module writes history state), but a real gap in the defensive contract D3 otherwise establishes.

**Proposed fix:** validate `route.name` against the known set before trusting it (fallback to switcher otherwise), or add `default: applyRoute({name:"switcher"})` to the switch.

## [P2] Opening a deep link directly leaves no switcher entry beneath it, so Back (on-screen or device) exits the app instead of reaching the agent list

**autofix_class:** manual — **status:** open

**What the code does today:** `bootstrap()` (`main.ts:157-174`) resolves a `/terminal/<pane_id>` deep link via `history.replaceState` only, since it's the page's first navigation — no switcher entry exists beneath it in the SPA's own history stack. `goBack()` (`main.ts:114-116`), wired to both the on-screen Back control and (via `popstate`) the device back gesture, calls `history.back()`, which then has nothing in-app to land on.

**Why this is a problem:** this is exactly PBI-025's primary use case — opening a bookmarked or shared terminal-detail link. `docs/specs/terminal-detail.md`'s Entry Points states unqualified that "Back ... returns to the agent list, in exactly one step either way (per pbi025-D2)" — that promise is not met for a deep-linked entry, the single most common way this new URL will actually be used.

**Proposed fix:** during the deep-link resolution path (bootstrap's terminal branch and the post-login redirect's terminal branch), seed a switcher entry beneath the terminal one — e.g. `replaceState` switcher first, then `pushState` terminal — so Back has an in-app switcher entry to land on. Alternatively, narrow the spec line to state the deep-link case explicitly if leaving the app is the intended behavior there.

## [P3] Importing `main.ts` for its pure helpers triggers a real bootstrap fetch and a global popstate listener as an import-time side effect

**autofix_class:** advisory — **status:** open

**What the code does today:** `main.ts:137` registers the `popstate` listener and `main.ts:176` calls `void bootstrap()` unconditionally at module top level; `main.test.ts` imports from `../src/main` to reach the pure helpers, incurring both side effects (including a real `fetchAgents()` network call) on every test run.

**Why this is a problem:** currently benign (root is null in jsdom so rendering no-ops, and the suite passed 66/66 in this session's independent verify run), but is a latent source of test flakiness or unhandled-rejection noise if the fetch/jsdom behavior ever changes.

**Proposed fix:** split the pure route helpers from the app-entry side effects (listener registration + `void bootstrap()`) into a separate concern, or gate the entry side effects behind a root-exists check.

## What passed cleanly

D1 (path-segment URL, round-trip verified), D2's "every navigation point goes through the history-aware path" (select agent, create success, logout all route through `navigate`; in-app Back through `goBack()`), D4 (login/switcher both map to `/`), and D5 (`resolveLoginRedirect` shared by bootstrap and post-login, shell reshape matches switcher.ts's own reshape) are all implemented faithfully. `fetchAgents` throw-vs-null is handled identically in both call sites.

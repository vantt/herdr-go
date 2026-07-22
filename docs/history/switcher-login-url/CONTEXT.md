# Switcher/Login URL — Context

**Feature slug:** switcher-login-url
**Date:** 2026-07-22
**Exploring session:** complete
**Scope:** Standard
**Domain types:** SEE

## Feature Boundary

Switcher (home agent list) and login each get their own dedicated, bookmarkable/refreshable URL — `/switcher` and `/login` — symmetric with terminal detail's existing `/terminal/<pane_id>` (PBI-025, D1-D5). `/` stops being either screen's own URL and becomes a pure legacy alias that silently canonicalizes to whichever concrete route applies. Terminal detail's own URL scheme and behavior (PBI-025 D1-D5) are unchanged by this feature.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Switcher's URL is `/switcher`; login's URL is `/login`; terminal detail keeps `/terminal/<pane_id>` untouched. `pathForRoute()`/`Route` are extended, not replaced, to give `login` and `switcher` their own concrete paths instead of both collapsing to `/`. | Confident default (gate-bypass, not asked): this is exactly the task's own framing ("give the switcher and login views their own bookmarkable URLs, e.g. /switcher or /login"), and `/switcher` matches the screen's existing name everywhere in the codebase (`docs/specs/switcher.md`, `Route.name`, `switcher.ts`). |
| D2 | `/` is no longer any screen's own URL. On every bootstrap, `/` and any path that isn't a recognized route (`/login`, `/switcher`, or a valid `/terminal/<pane_id>`) is silently canonicalized via `history.replaceState` to whichever concrete route the existing session-probe (`fetchAgents()`) resolves to — `/switcher` if authenticated, `/login` if not (see D5 for the one exception). | Preserves old bookmarks/links to `/` (no broken links) while extending PBI-025 D3's "silent fallback, no error" precedent from just terminal links to every path. No server change needed — `src/web/mod.rs`'s SPA fallback already returns `index.html` (200) for any unmatched path, confirmed for `/switcher` and `/login` the same way PBI-025 confirmed it for `/terminal/<id>`. |
| D3 | Visiting `/login` directly while already holding a valid session (the `fetchAgents()` probe succeeds) redirects straight to switcher — the login form is never shown to an already-authenticated operator. | Confident default: single-operator system (`docs/specs/switcher.md` Actors & Access), no product reason to re-enter a token when already signed in. |
| D4 | Visiting `/switcher` directly while not authenticated shows the login screen exactly like the unauthenticated-root case does today, and a successful login lands on switcher — already the default fallback target (`resolveLoginRedirect`'s existing `{name: "switcher"}` fallback) when there is no more specific intended pane. No new "intended route" storage is needed for this case. | Generalizes PBI-025 D5's "return to intended destination" pattern beyond terminal links, using logic that already exists. |
| D5 | The URL-preservation vs URL-canonicalization split, while login is shown, depends on why login is shown: if login is shown because a specific `/terminal/<pane_id>` link's session probe failed, the URL stays unchanged (PBI-025 D5's existing precedent — required so a reload or a later successful login still knows the intended pane). If login is shown for any other reason (unauthenticated at `/`, `/switcher`, `/login`, or any unrecognized path), the URL canonicalizes to `/login` via `history.replaceState`, so the login screen itself becomes bookmarkable/refreshable — this feature's whole purpose for login. | Confident default, derived by combining the task's stated goal (login gets its own bookmarkable URL) with PBI-025's already-locked precedent (never destroy a `/terminal/<pane_id>` link's intent while login is pending). |
| D6 | All new route transitions among login/switcher/terminal continue to go exclusively through the existing `navigate()` function and the single `popstate` listener (`critical-patterns.md`'s standing rule: no view or handler calls `history.pushState`/`replaceState`/`back()` directly). `showLogin()`'s existing bypass of `navigate()` (already present in the code before this feature, used when login is shown mid-bootstrap without a route transition yet decided) is an established carve-out, not a new one, and stays exactly as-is. | No new navigation primitive; keeps browser/phone Back and in-app Back on one consistent stack, per PBI-025 D2. |
| D7 | Any `navigate()` transition where the destination route is `login`, or where the transition leaves the `login` route (a successful login, per `handleLoginSuccess`), always uses `history.replaceState`, never `pushState` — overriding `navigate()`'s normal path-differs-so-push rule for this one pair. Every other transition (to/from `switcher`, to/from `terminal`) keeps pushing on a path change exactly as D6/PBI-025 D2 already specify. | Closes a gap the fresh-eyes review found: once login and switcher have distinct paths (D1), `navigate()`'s default push-on-path-change rule would make `/login` a real back-stack entry — pressing Back after a successful login (or after logging out and back in) would `popstate` into `applyRoute({name:"login"})` and render the login form to an already-authenticated operator, directly violating D3. Making `login` transitions always `replaceState` keeps `/login` bookmarkable/refreshable (D5 still holds) while never leaving it as a step Back can land on. |

### Agent's Discretion

Left to planning: whether `pathForRoute`/route-parsing is extended in place inside `main.ts` or split into a small dedicated router helper; exact shape of the "recognized path → Route" parsing function that replaces/extends `parseTerminalPaneId`'s narrower job; where the `/login`-vs-`/switcher`-vs-unrecognized-path canonicalization branches live inside `bootstrap()`; the exact mechanism for D7's forced-replace exception (e.g. an optional parameter on `navigate()`, or `navigate()` inspecting `history.state.route.name` to detect "leaving login").

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Canonicalize | Rewrite the visible URL via `history.replaceState` (no new history entry) to the concrete path that matches what's actually rendered, without navigating away from the current screen. |

## Existing Code Context

### Reusable Assets

- `web/src/main.ts` — `Route` type (`{name:"login"}|{name:"switcher"}|{name:"terminal"; agent}`), `pathForRoute`, `parseTerminalPaneId`, `resolvePaneRef`, `resolveLoginRedirect`, `navigate`, `goBack`, `handlePopState`, `bootstrap`, `handleLoginSuccess`. All already fully wired for `/terminal/<pane_id>` per PBI-025; this feature extends the same functions to cover `/switcher` and `/login` instead of adding new ones.
- `src/web/mod.rs`'s `router()` — SPA fallback (`ServeDir::new(static_dir).fallback(ServeFile::new(index))` / embedded-UI equivalent) already returns `index.html` (200) for any unmatched path, confirmed by PBI-025 for `/terminal/<id>`; no server route changes needed for `/switcher` or `/login` either. `POST /api/login` (`src/web/auth.rs`) is a distinct API path under `/api/`, no collision with the client-side `/login` path.

### Established Patterns

- PBI-025 D1-D5 (`docs/history/pbi-025-terminal-detail-url/CONTEXT.md`) — the exact precedent this feature generalizes: path-segment URLs, `pushState`/`popstate` via `navigate()`, silent fallback on an unresolvable route, and preserving intent across a login round-trip.

### Integration Points

- `web/src/views/switcher.ts` — `onLoggedOut` callback already triggers a login navigation on a 401 from `fetchAgents()` mid-session; no change needed beyond the target path `navigate()` now resolves.
- `web/src/views/login.ts` — `onSuccess`/`onLoggedOut` callback shapes are unchanged; only what `main.ts` does with them shifts.

## Canonical References

- `docs/specs/terminal-detail.md` R8-R10 — current spec language ("This is the only screen with its own link; the agent list and the sign-in screen share one undifferentiated link") that this feature supersedes.
- `docs/specs/switcher.md` Open Gaps — "The login screen and the individual live-terminal screen... are separate areas with their own behavior, not yet specced" (login has no spec section yet; scribing should address this alongside the URL sync).
- `docs/history/pbi-025-terminal-detail-url/CONTEXT.md` D1-D5 — the pattern this feature extends.
- `docs/backlog.md` PBI-035 row — original problem statement, now flipped to `in-flight`.

## Outstanding Questions

None — every gray area had a confident, evidence-grounded default (see rationale column above); gate-bypass level `full` covers this lane, so no questions were asked.

## Deferred Ideas

None new. PBI-026 (pane/agent/terminal terminology cleanup) remains a separate, already-filed PBI; this feature keeps using `pane_id`/`agent`/`switcher`/`login` naming exactly as the existing code already does.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked decisions, code context, canonical references. Validating and reviewing use locked decisions for coverage and UAT.

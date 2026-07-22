# Terminal Detail URL — Context

**Feature slug:** pbi-025-terminal-detail-url
**Date:** 2026-07-22
**Exploring session:** complete
**Scope:** Standard
**Domain types:** SEE

## Feature Boundary

The terminal detail screen gets a dedicated, bookmarkable/refreshable URL (`/terminal/<pane_id>`); `bootstrap()` reads that URL on load and opens the matching pane's terminal detail directly instead of always defaulting to `login`/`switcher`. Switcher and login themselves are not given URLs (PBI-033, deferred).

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Terminal detail's URL uses a path segment: `/terminal/<pane_id>` (not a `?pane=` query string). | Server already SPA-fallbacks any unmatched path to `index.html` with 200 (`src/web/mod.rs`'s `router()`, lines 84-96), so a path segment is exactly as feasible as a query string — this was a real choice, not a technical constraint. User chose path segment. |
| D2 | Opening terminal detail pushes a new browser history entry (`history.pushState`), not `history.replaceState`. In-app Back navigates by calling `history.back()` (or an equivalent that pops the stack), never by pushing a fresh forward entry — so browser Back and in-app Back stay one consistent back-stack instead of diverging (pushing a new entry on in-app Back would make `/terminal` forward-reachable again after a browser Back, requiring a second back to actually leave). | Phone Back button / swipe-back gesture must return to switcher, matching the in-app Back button. Every navigation point that changes the visible route (select agent, create-flow success `onCreated`, in-app Back) must go through the same history-aware path. |
| D3 | If the URL's `pane_id` does not resolve against the `/api/agents` snapshot at bootstrap time (pane already closed, or a stale/invalid link), the app silently falls back to switcher — no toast, no error message. | Matches today's existing behavior for any unrecognized route; kept simple per user's choice. |
| D4 | Only the terminal-detail route gets a dedicated URL. Switcher and login stay at root `/` with no distinct URL of their own. | Confident default (gate-bypass, not asked): this is exactly PBI-025's scope ("URL riêng cho terminal detail"); giving switcher/login their own URLs is unrelated scope creep. Deferred to backlog as PBI-033. |
| D5 | If the session is invalid/expired when a `/terminal/<pane_id>` URL is opened (bootstrap's `fetchAgents()` probe fails), a successful login redirects straight back into that same terminal (if the pane still resolves) instead of always landing on switcher after login. The exact mechanism (how the intended `pane_id` survives the login round-trip) is deferred to planning — see Outstanding Questions. | Confident default (gate-bypass, not asked): this is the same "don't lose context" goal PBI-025 exists for, applied to the login-required case. Note: `main.ts:33` today hardcodes `onSuccess: () => navigate({ name: "switcher" })`, so `onSuccess` itself must change to honor this — it is a distinct code path from `bootstrap()`, not a free ride on it. |

### Agent's Discretion

Left to planning: how the URL is parsed on load (manual `location.pathname` parsing vs. a tiny helper), how a `popstate` listener is wired to re-invoke `navigate()` with the previous route (no such listener exists today), and where exactly the pane_id → AgentRow/ShellRow lookup happens inside `bootstrap()`.

## Existing Code Context

### Reusable Assets

- `web/src/main.ts` — `Route` type (`{name:"login"}|{name:"switcher"}|{name:"terminal"; agent}`), `navigate()`, `bootstrap()`. This is the file that gains URL read/write; today it contains zero `history.`/`location.` calls.
- `web/src/api.ts` — `fetchAgents()` returns `{agents: AgentRow[], shells: ShellRow[]}` in one snapshot; both `AgentRow` and `ShellRow` carry `pane_id`. This is the lookup planning uses to resolve a URL's `pane_id` back into a full row at bootstrap.
- `web/src/views/switcher.ts` — shows how a `ShellRow` is turned into a `NewPaneRef`-shaped object (`{pane_id, workspace_id, label}`) for `onSelect`; the same shape-building applies when reconstructing a shell's terminal detail from a URL.

### Established Patterns

- `NewPaneRef` (`web/src/main.ts`) — the minimal reference shape already used to open terminal detail for a pane that has no full `AgentRow` yet (freshly created pane, or a plain shell). The bootstrap-from-URL path for shells will produce this same shape from `ShellRow`.

### Integration Points

- `src/web/mod.rs`'s `router()` (lines 84-96) — SPA fallback (`ServeDir::new(static_dir).fallback(ServeFile::new(index))` / embedded-UI equivalent): confirms an unmatched path like `/terminal/<id>` returns `index.html` with 200 today, no server route changes needed for D1.
- `docs/specs/terminal-detail.md` — Data Dictionary #2 already defines a `Pane gone` connection state for when "the selected terminal no longer exists" — that's the *already-open* screen's polling failure, a different case from D3 (URL doesn't even resolve at initial bootstrap, before any screen opens).

## Canonical References

- `docs/specs/terminal-detail.md` — current spec for the terminal detail screen this feature deep-links into.
- `docs/backlog.md` PBI-025 row — original user-reported problem statement.

## Outstanding Questions

### Deferred To Planning

- [ ] Exact `popstate` wiring so the phone/browser Back button (enabled by D2's `pushState`) correctly re-renders the previous route — no existing listener to model this on.
- [ ] How the intended `pane_id` survives the login round-trip so D5's `onSuccess` (currently hardcoded to `navigate({ name: "switcher" })` at `main.ts:33`) can redirect into the right terminal instead of always the switcher — e.g. read the pre-login URL again inside `onSuccess`, or stash the pane_id in a variable captured before `renderLogin` runs.

## Deferred Ideas

- Switcher/login getting their own dedicated URLs, symmetric with terminal detail — filed as PBI-033 in `docs/backlog.md`, deferred by D4 to avoid scope creep.
- PBI-026 (pane/agent/terminal terminology cleanup) is a separate, already-filed PBI; this feature keeps using `pane_id` as the URL identifier regardless of how that naming question eventually resolves.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked decisions, code context, canonical references, and the deferred-to-planning question above. Validating and reviewing use locked decisions for coverage and UAT.

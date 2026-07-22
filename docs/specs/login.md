---
area: login
updated: 2026-07-22
sources: [switcher-login-url]
decisions: [D1, D2, D3, D4, D5, D6, D7]
coverage: partial
---

# Spec: Login

The screen an Operator without a valid session lands on. Entering the shared
access token establishes a session and returns the Operator to wherever they
were actually headed — switcher by default, or a specific terminal if that is
what brought them here in the first place.

## Entry Points & Triggers

- App load with no valid session → this screen (switcher.md's own Entry Points
  describe the mirror case: a valid session goes straight to switcher instead).
- Opening this screen's own URL directly, or opening switcher's URL, the app's
  root URL, or any unrecognized URL, while not authenticated → this screen,
  with the visible URL canonicalized to this screen's own link (per D2/D5).
- Opening this screen's own URL directly while already authenticated → skips
  straight to switcher instead; the form is never shown to an already-valid
  session (per D3).
- Opening a stale-session terminal-detail link while not authenticated → this
  screen, but the visible URL is left exactly as opened (not canonicalized) so
  a successful sign-in can return straight to that terminal afterwards — see
  `terminal-detail.md` R10.
- Tapping the logout icon on switcher → ends the current session and returns
  here.

## Data Dictionary

| # | Element | Meaning | Values | Required | Default |
|---|---------|---------|--------|----------|---------|
| 1 | Access token | The single shared secret that grants access to the whole app | free text, entered masked | yes | empty |
| 2 | Error message | Feedback shown after a failed sign-in attempt | "Access denied. Check your token and try again." (wrong or missing token) · "Could not reach the gateway. Try again." (network/server failure) | no | empty |

## Behaviors & Operations

### Sign in

- **Triggers:** submitting the token form (Enter, or the Connect control).
- **Blocked when:** the token field is empty — nothing is sent.
- **What changes:** the submit control shows a loading state and disables for
  the duration of the attempt, preventing a duplicate submit.
- **Side effects:** a correct token establishes a new session; a wrong token
  and an unreachable gateway are the only two distinguishable outcomes shown
  to the Operator — neither reveals anything about how close a wrong token
  was to correct.
- **Afterwards:** on success, the Operator leaves this screen for switcher, or
  for the specific terminal they were trying to reach if a stale-session link
  brought them here (per D5, `terminal-detail.md` R10); on failure, the form
  stays open, the token field is reselected for another attempt, and the
  matching error message explains why.

### Sign out

- **Triggers:** tapping the logout icon on switcher.
- **What changes:** the current session ends.
- **Side effects:** none beyond ending the session.
- **Afterwards:** the Operator returns to this screen; pressing Back
  afterwards can never re-render this screen to an operator who has since
  signed back in (per D7), and can never silently keep the previously signed
  in view on screen without re-checking the session.

## Actors & Access

| Capability | Anyone holding the shared token | Anyone without it |
|---|---|---|
| Sign in | ✓ — full app access | — (denied, generic message) |
| Stay signed in | session persists for a bounded period | n/a |

Single shared credential, not per-user accounts — one operator role, matching
`switcher.md`'s Actors & Access.

## Business Rules

- **R1.** This screen has its own dedicated URL, bookmarkable and refreshable
  the same way switcher's and terminal detail's are (per D1).
- **R2.** An already-authenticated visit to this screen's URL redirects
  straight to switcher; the form is never shown to a session that is already
  valid (per D3).
- **R3.** A visit to this screen's own URL, to switcher's URL, to the app's
  root, or to any unrecognized URL, all land here when there is no valid
  session, and the visible URL canonicalizes to this screen's own link —
  except a stale-session terminal-detail link, which lands here too but keeps
  its own URL unchanged so sign-in can return to that specific terminal (per
  D2/D5, `terminal-detail.md` R10).
- **R4.** A wrong token and an unreachable gateway are the only two
  distinguishable failure states ever shown; the exact reason a token was
  rejected is never revealed (fail-closed, opaque-failure design).
- **R5.** This app has a single shared credential, not per-user accounts —
  matches `switcher.md`'s single-operator system.
- **R6 (app-wide, not login-specific).** This screen renders dark-only, same
  as the rest of the app (per decision `de2781bf`, `switcher.md` R8).

## Edge Cases Settled

- Submitting an empty token field does nothing — no request sent, no error
  shown.
- A wrong token and a network/server failure produce two different, clearly
  worded messages, but neither one reveals why a specific token failed.
- Submitting twice in quick succession is prevented by disabling the submit
  control while a request is already in flight.

## Open Gaps

- Session lifetime/expiry behavior — exactly what the Operator sees the
  moment a session lapses while a different screen is already open and
  rendered ("mid-view expiry") — is not specified here; `terminal-detail.md`'s
  Open Gaps already flags the same class of gap for its own screen.
- No settled snapshot exists under `docs/specs/visuals/login/` — same gap
  pattern already noted for switcher and terminal detail.
- No account-recovery or token-rotation flow exists or is described anywhere;
  out of scope until a real need surfaces.

## Visuals

No current snapshot — see Open Gaps.

## Pointers (implementation)

- `web/src/views/login.ts` — renders this screen: the token form, submit
  handling, and the two error messages.
- `web/src/main.ts` — `pathForRoute`/`parseRoutePath` (this screen's own
  concrete path), `resolveBootstrapDecision` (the canonicalize/redirect logic
  behind R2/R3), `navigate()`'s login-transition `replaceState` exception
  behind the Sign out behavior's Back guarantee, `showLogin()`,
  `handleLoginSuccess()`.
- `web/src/api.ts` — `login(token)`, `logout()`.
- `src/web/auth.rs` — `POST /api/login` / `POST /api/logout` handlers,
  session issuance/validation, constant-time token comparison, the
  fail-closed opaque-failure design behind R4.

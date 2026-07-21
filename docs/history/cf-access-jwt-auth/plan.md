# Plan — cf-access-jwt-auth (PBI-032)

Status: shaped (high-risk). Locked decisions and research: see CONTEXT.md.

## Phases
1. **Config** — add optional non-secret `cf_access_team_domain` + `cf_access_aud`
   to `RawConfig`/`Config` (`src/config/mod.rs`). Absent by default; loading
   must not error on a config file that omits them.
2. **JWKS client + JWT verify** — new module `src/web/cf_access.rs`:
   fetch/cache JWKS from `${team_domain}/cdn-cgi/access/certs` (via existing
   `reqwest` client), verify RS256 signature + `iss == team_domain` +
   `aud contains cf_access_aud` + `exp`/`nbf`, using the `jsonwebtoken` crate
   (new dependency). Cache keys by `kid` with a bounded TTL refetch, fail
   closed (any verify error => not authenticated, same as today).
3. **Wire into AuthSession** — `src/web/auth.rs`: when CF Access is
   configured, `AuthSession::from_request_parts` additionally accepts a
   valid `Cf-Access-Jwt-Assertion` header as authenticated, alongside the
   existing session-cookie path. No change when unconfigured.
4. **Docs** — short section in a relevant `docs/specs/` auth doc (or new one
   if none exists) stating: opt-in, additive, and the operator is
   responsible for making the origin unreachable except via CF (tunnel, not
   an exposed port) — the app cannot enforce that.

## Acceptance criteria
- Default build/config (no CF Access settings) behaves byte-identical to
  today: existing web-auth tests keep passing unmodified.
- With `cf_access_team_domain`/`cf_access_aud` configured and a request
  carrying a validly-signed JWT for that team/aud, guarded endpoints succeed
  without a session cookie.
- Invalid signature, wrong `aud`, wrong `iss`, or expired JWT => same silent
  404 as an unauthenticated request today (no distinguishing error).
- `cargo test --quiet && cargo fmt --all --check && cargo clippy --all-targets -- -D warnings`
  (the project's verify command) stays green.

## Risks / rollback
- New dependency (`jsonwebtoken`) — pure-Rust, no new native deps, standard
  in the ecosystem for this exact use case.
- Network call to CF's JWKS endpoint only happens when CF Access is
  configured; if unreachable, verification fails closed (treated as
  unauthenticated), never open.
- Rollback: the feature is additive and config-gated; reverting is deleting
  the new module + the two config fields + the extra branch in
  `AuthSession`.

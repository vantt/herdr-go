# CONTEXT — cf-access-jwt-auth (PBI-032)

## Ask (Vietnamese, user)
"Tích hợp login/logout qua Cloudflare Access": nếu đã có CF Access đứng trước
và đã có CF Access token, dùng token đó luôn để auth, khỏi phải đi qua webform
login riêng của app — hoặc tích hợp webform login vào CF Access.

## Locked prior decisions (do not reverse)
- `docs/PRD.md:128-136` (chốt 2026-07-17): gateway is the **sole security
  boundary** in front of the herdr socket (socket itself has no auth).
- Web auth today: **static token + session cookie, fail-closed**.
- Transport binds to **Tailscale tailnet only, not public Internet** — token
  is defense-in-depth; tailnet is explicitly NOT treated as auth.
- `docs/backlog.md:39` (PBI-032 row) flags: dropping the token because CF
  Access is present is a **transport-model change**, not just an extra login
  option, and is only safe if (a) the app verifies the CF Access JWT
  signature (not the raw header) and (b) the origin is unreachable except
  through CF (tunnel, not an exposed port).

## Research (this session)
- Cloudflare Access JWT validation (confirmed via developers.cloudflare.com,
  2026-07-21): app receives `Cf-Access-Jwt-Assertion` header; JWKS at
  `${TEAM_DOMAIN}/cdn-cgi/access/certs`; must verify RS256 signature, `iss`
  == team domain, `aud` == the application's Access Audience (AUD) tag,
  and `exp`/`nbf`. Trusting the header without verification is exactly the
  hole PBI-032's backlog note warns about.
- Current auth implementation mapped (`src/web/auth.rs`, `src/web/mod.rs`,
  `src/config/mod.rs`):
  - Static token: `HERDR_GO_WEB_SECRET`, env-only (`Secrets.web_session_secret`,
    never in the JSON config file — `deny_unknown_fields` would reject it).
  - `POST /api/login` (`auth.rs:29-51`): constant-time compare of posted
    token, mints 24-byte hex session id, stores in in-memory
    `AppState.sessions: Arc<Mutex<HashSet<String>>>`, sets `hg_session`
    cookie (`HttpOnly; SameSite=Strict; Path=/; Max-Age=604800`, no
    `Secure` — tailnet-only assumption).
  - `AuthSession` extractor (`auth.rs:67-84`) is the auth gate: checks the
    cookie against the session set; any failure returns a silent 404 (no
    401, no body) — applied per-handler, not global middleware.
  - No existing Cloudflare code anywhere in the repo.

## Decision (logged, id 694cfa72-71bf-4a33-bbc3-8a5a638a491f)
Add Cloudflare Access JWT verification as an **optional, additional**
credential accepted by the existing `AuthSession` gate:
- New optional config (`cf_access_team_domain`, `cf_access_aud`) — unset by
  default, zero behavior change when absent.
- When configured: on each guarded request, `AuthSession` also accepts a
  verified `Cf-Access-Jwt-Assertion` JWT (JWKS fetched/cached from the team
  domain, `aud`+`iss`+`exp` checked) as equivalent to a valid session
  cookie — so a user already authenticated by CF Access at the edge never
  sees the app's own login form.
- The static-token + cookie path is **not removed**; both remain valid.
  This is additive, so it does not reopen the fail-closed hole and does not
  change the tailnet-only default — an operator who wants this must
  deliberately configure CF Access (e.g. via Cloudflare Tunnel) themselves;
  the code cannot and does not enforce that the origin is unreachable by any
  other path (that remains an operator/network responsibility, documented,
  not silently assumed safe).

## Out of scope / explicitly not doing
- Not changing the default transport (still tailnet-only unless the operator
  opts into exposing via Cloudflare Tunnel on their own).
- Not removing the static-token webform login path.
- Not building a full CF Access "login/logout" UI flow — CF Access owns its
  own login/logout at the edge; the app only needs to *recognize* an already
  verified CF Access session, not replicate CF's login UI.

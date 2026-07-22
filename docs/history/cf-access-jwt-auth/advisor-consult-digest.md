# Advisor consult — cf-access-jwt-auth (AO2(b) unconfigured-advisor path)

Advisor: none configured (`.bee/config.json` has no `advisor` key; `models.claude`
carries `extraction`/`generation` only). Per AO2(b) an unconfigured advisor is
not a hard dependency for gate approval — recording that fact and proceeding,
matching the precedent already in this repo's history (`cross-platform-install`,
`web-create-endpoints` lanes).

Feasibility for this high-risk change was established by direct code research
(src/web/auth.rs, src/config/mod.rs mapped file:line) plus external
confirmation of Cloudflare Access JWT validation semantics
(developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/authorization-cookie/validating-json/),
not a spike, and it is load-bearing for the plan in plan.md.

Load-bearing conclusion: the design is additive and config-gated (opt-in,
default-off), verifies the CF Access JWT signature via JWKS rather than
trusting the header, and does not touch the locked tailnet-only transport
decision (PRD.md:128-136) or remove the existing static-token+cookie path.
This satisfies both safety conditions PBI-032's backlog note requires for
"drop the webform token when CF Access is present" to be safe: (a) verified
JWT, not trusted header; (b) the app is honest that it cannot itself enforce
"unreachable except via CF" -- that stays an operator/network responsibility,
documented rather than silently assumed.

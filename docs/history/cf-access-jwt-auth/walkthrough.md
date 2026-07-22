# Review Walkthrough — cf-access-jwt-auth (PBI-032)

Session: `review-cf-access-jwt-auth-20260722` (`.bee/reviews/review-cf-access-jwt-auth-20260722.json`)
Scope: feature `cf-access-jwt-auth`, cells `cf-access-jwt-auth-1`/`-2`, diff `ad69948..0c7690a`.
Requested by: user, 2026-07-22.

## What was built

Optional, opt-in Cloudflare Access JWT verification as an alternate credential
for the existing `AuthSession` gate. Unconfigured = byte-identical to before.
Configured = a request carrying a verified `Cf-Access-Jwt-Assertion` header
authenticates without the static-token webform login. Full design rationale
in `CONTEXT.md`.

## Reviewers dispatched (6 — full wave, high-risk auth scope)

code-quality, architecture, security, test-coverage (core) + api-contract,
reliability (conditional — auth-gate contract change, new external JWKS call).
All isolated to the frozen diff + `CONTEXT.md` + `plan.md`, no session history.

## Findings summary

| Severity | Count | Disposition |
|---|---|---|
| P1 | 2 | Both fixed in `cf-access-jwt-auth-3` (commit `a5e2a25`) |
| P2 | 6 | Filed to `.bee/backlog.jsonl` (`review-finding`, feature `cf-access-jwt-auth`), non-blocking |
| P3 | 6 | Filed to `.bee/backlog.jsonl`, non-blocking |

### P1s (fixed)

1. **`iss`/`aud` validated-if-present, not required** (security) — a token
   omitting either claim was accepted. Orchestrator elevated this from the
   reviewer's own P2 to P1: `CONTEXT.md` names checking `aud` as one of
   exactly two non-negotiable safety conditions for this feature to be safe
   at all. Fix: `validation.set_required_spec_claims(&["exp","iss","aud"])`.
2. **No regression test for JWT algorithm-substitution** (`alg=none` /
   HS256-key-confusion) (test-coverage) — the protection itself was
   independently confirmed already enforced by the `jsonwebtoken` crate
   (RS256 pinned), but nothing would catch a future refactor that loosened
   it. Fix: two new tests forging both attack shapes and asserting `Err`.

Both fixes independently re-verified by the orchestrator: fresh
`cargo build`/`test` (271+2+3 green)/`fmt`/`clippy`, plus a live
revert-and-confirm-failure check proving the two new required-claims tests
actually catch the regression (not tautological).

### P2s corroborated by 2+ independent reviewers (promoted from P3)

- Discarded CF Access identity + module-wide `allow(dead_code)` masking it
  (code-quality + architecture).
- JWKS refetch has no single-flight guard, thundering-herd on cold
  cache/TTL burst (security + reliability).

### Full findings

Recorded on the session record (`findings` array) and filed individually to
`.bee/backlog.jsonl` — not re-pasted here; see the session JSON and
`bee reviews show --id review-cf-access-jwt-auth-20260722` for the complete
schema-formatted list.

## Verification evidence

- `cargo build --quiet`: clean.
- `cargo test --quiet`: 271 lib + 2 + 3 integration, 0 failed (up from 262
  baseline before this feature; +9 net across both cells and the P1 fix).
- `cargo fmt --all --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `bash tests/rename_contract.sh`: ok.

## UAT

Not run as a formal itemized walkthrough — `CONTEXT.md` for this feature was
not written with SEE/CALL/RUN-tagged decision items. The orchestrator did not
self-approve Gate 4 on that basis; the merge question is presented to the
user directly instead.

## Decision

P1 count at close: 0. Awaiting the user's explicit Gate 4 answer (session
`decision.status` left `pending`, not auto-approved — gate bypass's
merge-auto-approve path requires confirmed UAT, which did not run here).

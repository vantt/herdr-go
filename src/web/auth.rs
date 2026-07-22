//! Authentication — static token + session cookie, fail-closed and silent.
//!
//! Single operator (decision bc4a65a4): a login presents the configured secret
//! (compared in constant time); on success the server issues a random session
//! id as an httpOnly, SameSite=Strict cookie. Every protected route requires a
//! valid session cookie; an unauthenticated request gets an **opaque 404** — no
//! descriptive 401, nothing that confirms the app or its routes exist (airemote
//! D46/D56 "silence is the point", applied to HTTP).

use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use super::AppState;

const COOKIE_NAME: &str = "hg_session";

/// Header Cloudflare Access sets on requests it has already authenticated at the
/// edge. Read case-insensitively (axum lowercases header names).
const CF_ACCESS_HEADER: &str = "cf-access-jwt-assertion";

#[derive(Deserialize)]
pub struct LoginBody {
    token: String,
}

/// POST /api/login — validate the token, issue a session cookie. A wrong or
/// missing token returns the same opaque 404 as any other auth failure, so the
/// endpoint leaks nothing about whether it exists or what it wants.
pub async fn login(State(state): State<AppState>, Json(body): Json<LoginBody>) -> Response {
    let configured = match state.web_secret.as_ref() {
        Some(s) if !s.is_empty() => s,
        // Misconfigured (no secret) → fail closed, leak nothing.
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    if !constant_time_eq(body.token.as_bytes(), configured.as_bytes()) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let session_id = new_session_id();
    state.sessions.lock().await.insert(session_id.clone());

    let cookie =
        format!("{COOKIE_NAME}={session_id}; HttpOnly; SameSite=Strict; Path=/; Max-Age=604800");
    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, cookie.parse().unwrap());
    (
        StatusCode::OK,
        headers,
        Json(serde_json::json!({"ok": true})),
    )
        .into_response()
}

/// POST /api/logout — invalidate the current session. Always 200 (idempotent).
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(sid) = session_cookie(&headers) {
        state.sessions.lock().await.remove(&sid);
    }
    let expired = format!("{COOKIE_NAME}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0");
    let mut out = HeaderMap::new();
    out.insert(header::SET_COOKIE, expired.parse().unwrap());
    (StatusCode::OK, out).into_response()
}

/// Extractor proving a request is authenticated. A valid `hg_session` cookie is
/// the primary credential; when the operator has configured Cloudflare Access, a
/// verified `Cf-Access-Jwt-Assertion` header is accepted as an equivalent
/// alternate. On failure it short-circuits with an opaque 404 — the caller never
/// distinguishes "no cookie", "bad CF token", or "unknown route".
pub struct AuthSession;

#[async_trait::async_trait]
impl FromRequestParts<AppState> for AuthSession {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Primary credential: a valid session cookie. Unchanged from before CF
        // Access existed — same lookup, same silent failure.
        if let Some(sid) = session_cookie(&parts.headers) {
            if state.sessions.lock().await.contains(&sid) {
                return Ok(AuthSession);
            }
        }

        // Additive fallback: only when the operator configured CF Access does a
        // request without a valid cookie get a second chance via an edge-verified
        // JWT. The raw header is never trusted — `verify` runs the full
        // JWKS/signature/iss/aud/exp check; any failure is treated exactly like
        // no credential at all.
        if let Some(verifier) = state.cf_access.as_ref() {
            if let Some(assertion) = parts
                .headers
                .get(CF_ACCESS_HEADER)
                .and_then(|v| v.to_str().ok())
            {
                if verifier.verify(assertion).await.is_ok() {
                    return Ok(AuthSession);
                }
            }
        }

        Err(silent_404())
    }
}

fn silent_404() -> Response {
    StatusCode::NOT_FOUND.into_response()
}

/// Extract the `hg_session` value from the Cookie header, if present.
pub fn session_cookie(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    for pair in raw.split(';') {
        let pair = pair.trim();
        if let Some(v) = pair.strip_prefix(&format!("{COOKIE_NAME}=")) {
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn new_session_id() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Constant-time byte comparison — no early return on first mismatch, so timing
/// does not leak how much of the token was correct.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        // Still walk a fixed amount to avoid trivially leaking length; but the
        // length difference itself is unavoidable. Return false.
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::{api_router, test_state};
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[test]
    fn constant_time_eq_matches_semantics() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }

    #[tokio::test]
    async fn unauth_agents_returns_opaque_404() {
        let app = api_router(test_state());
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn wrong_token_login_is_opaque_404() {
        let app = api_router(test_state());
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"token":"wrong"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn correct_token_sets_cookie_and_grants_access() {
        let state = test_state();
        let app = api_router(state.clone());
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"token":"s3cret-token"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let cookie = res
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));

        // Reuse the cookie to reach a protected route.
        let sid = cookie.split(';').next().unwrap().to_string();
        let app2 = api_router(state);
        let res2 = app2
            .oneshot(
                Request::builder()
                    .uri("/api/agents")
                    .header(header::COOKIE, sid)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res2.status(), StatusCode::OK);
    }

    /// Regression guard: with CF Access unconfigured (the default `test_state`),
    /// an unauthenticated request to a guarded route is byte-identical to today —
    /// an opaque 404 — even if it carries a CF Access header, which must be
    /// completely ignored when no verifier is configured.
    #[tokio::test]
    async fn cf_access_unconfigured_ignores_header_and_stays_opaque_404() {
        let app = api_router(test_state());
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents")
                    .header("Cf-Access-Jwt-Assertion", "anything.at.all")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    /// With CF Access configured and a validly-signed assertion for the expected
    /// team/aud, a guarded route succeeds with NO session cookie present.
    #[tokio::test]
    async fn cf_access_configured_valid_header_authenticates_without_cookie() {
        let (verifier, token) = crate::web::cf_access::test_verifier_with_valid_token();
        let state = test_state().with_cf_access(Some(verifier));
        let app = api_router(state);
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents")
                    .header("Cf-Access-Jwt-Assertion", token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    /// With CF Access configured but the assertion unverifiable (not a real
    /// signed token), the request gets exactly the same opaque 404 as any
    /// unauthenticated one — the raw header is never trusted.
    #[tokio::test]
    async fn cf_access_configured_bogus_header_is_opaque_404() {
        let (verifier, _valid) = crate::web::cf_access::test_verifier_with_valid_token();
        let state = test_state().with_cf_access(Some(verifier));
        let app = api_router(state);
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents")
                    .header("Cf-Access-Jwt-Assertion", "not.a.jwt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    /// With CF Access configured, a valid session cookie still authenticates —
    /// the cookie path is preserved unchanged alongside the new branch.
    #[tokio::test]
    async fn cf_access_configured_cookie_still_authenticates() {
        let (verifier, _token) = crate::web::cf_access::test_verifier_with_valid_token();
        let state = test_state().with_cf_access(Some(verifier));
        let app = api_router(state.clone());
        let login = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"token":"s3cret-token"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let sid = login
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string();
        let res = api_router(state)
            .oneshot(
                Request::builder()
                    .uri("/api/agents")
                    .header(header::COOKIE, sid)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}

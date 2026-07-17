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

/// Extractor proving a request carries a valid session cookie. On failure it
/// short-circuits with an opaque 404 — the caller never distinguishes "no
/// cookie" from "unknown route".
pub struct AuthSession;

#[async_trait::async_trait]
impl FromRequestParts<AppState> for AuthSession {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let sid = session_cookie(&parts.headers).ok_or_else(silent_404)?;
        if state.sessions.lock().await.contains(&sid) {
            Ok(AuthSession)
        } else {
            Err(silent_404())
        }
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
}

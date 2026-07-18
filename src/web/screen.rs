//! Observe + reply — the phone-friendly terminal surface (decision 675fc93a).
//! `read_screen` polls a pane's rendered screen for a zoom/pan view; `send_reply`
//! posts a textarea reply into the pane. Both are plain request/response over
//! herdr's socket — no live stream, no PTY sizing.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use super::auth::AuthSession;
use super::AppState;

#[derive(Debug, Serialize)]
pub struct ScreenBody {
    pub text: String,
    pub revision: u64,
}

/// GET /api/panes/:pane/screen — the pane's current rendered screen (ANSI).
pub async fn read_screen(
    _auth: AuthSession,
    State(state): State<AppState>,
    Path(pane): Path<String>,
) -> Response {
    match state.herdr.read_pane(&pane).await {
        Ok(read) => Json(ScreenBody {
            text: read.text,
            revision: read.revision,
        })
        .into_response(),
        Err(crate::herdr::HerdrError::NoSuchPane(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ReplyBody {
    pub text: String,
    /// Whether to submit (send Enter after the text). Defaults to true.
    #[serde(default = "default_submit")]
    pub submit: bool,
}

fn default_submit() -> bool {
    true
}

/// POST /api/panes/:pane/input — send a reply into the pane. The human decides
/// when to send (they see the screen), so no readiness guard here.
pub async fn send_reply(
    _auth: AuthSession,
    State(state): State<AppState>,
    Path(pane): Path<String>,
    Json(body): Json<ReplyBody>,
) -> Response {
    match state.herdr.send_input(&pane, &body.text, body.submit).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "ok": true }))).into_response(),
        Err(crate::herdr::HerdrError::NoSuchPane(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::{api_router, test_login_cookie, test_state};
    use axum::body::Body;
    use axum::http::{header, Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn screen_requires_auth() {
        let app = api_router(test_state());
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/panes/w1:p1/screen")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND); // opaque, no leak
    }

    #[tokio::test]
    async fn screen_returns_pane_text() {
        let state = test_state();
        let cookie = test_login_cookie(&state).await;
        let app = api_router(state);
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/panes/w1:p1/screen")
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let s: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(s["text"].as_str().unwrap().contains("Building the parser"));
    }

    #[tokio::test]
    async fn reply_posts_and_lands() {
        let state = test_state();
        let cookie = test_login_cookie(&state).await;
        let app = api_router(state.clone());
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/panes/w1:p1/input")
                    .header(header::COOKIE, &cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"do it"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        // The reply landed in the pane screen.
        let read = state.herdr.read_pane("w1:p1").await.unwrap();
        assert!(read.text.contains("do it"));
    }

    #[tokio::test]
    async fn reply_to_unknown_pane_is_404() {
        let state = test_state();
        let cookie = test_login_cookie(&state).await;
        let app = api_router(state);
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/panes/nope/input")
                    .header(header::COOKIE, cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"x"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}

//! Observe + reply — the phone-friendly terminal surface (decision 675fc93a).
//! `read_screen` polls a pane's rendered screen for a zoom/pan view; `send_reply`
//! posts a textarea reply into the pane. Both are plain request/response over
//! herdr's socket — no live stream, no PTY sizing.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use super::auth::AuthSession;
use super::AppState;
use crate::herdr::pane_scroller::PaneScroller;
use crate::herdr::ReadSource;

#[derive(Debug, Serialize)]
pub struct ScreenBody {
    pub text: String,
    pub revision: u64,
}

#[derive(Debug, Deserialize)]
pub struct ScreenQuery {
    /// Presence requests older pane content via `PaneScroller` (CONTEXT.md
    /// D9) instead of the default live-view read; absent -> the existing
    /// behavior below, unchanged. The value doubles as how many PageUp-hops
    /// back from the live bottom to go this call (CONTEXT.md D-multi-page)
    /// -- `?history=1` (the original, still-valid shape) means one hop;
    /// `?history=3` means three. A present-but-non-numeric value (e.g. the
    /// literal presence check some older callers used) falls back to one
    /// hop rather than erroring, since presence alone used to be the entire
    /// contract.
    #[serde(default)]
    pub history: Option<String>,
}

/// GET /api/panes/:pane/screen — the pane's current rendered screen (ANSI).
/// With `?history=<n>`, routes through `PaneScroller::read_history` instead
/// (CONTEXT.md D9/D11), going `n` PageUp-hops back from live in one round
/// trip before always restoring to live — same response shape, no new
/// endpoint. Every call is self-contained: the gateway keeps no scroll depth
/// between requests, so a caller wanting to go further back than its last
/// call just asks for one more hop next time.
pub async fn read_screen(
    _auth: AuthSession,
    State(state): State<AppState>,
    Path(pane): Path<String>,
    Query(query): Query<ScreenQuery>,
) -> Response {
    let read = if let Some(history) = &query.history {
        let pages = history.parse::<usize>().unwrap_or(1).max(1);
        let scroller = PaneScroller::new(state.herdr.as_ref());
        scroller.read_history(&pane, pages).await
    } else {
        // Unwrapped, so a long line arrives as the one logical line the
        // program wrote rather than the several physical ones the pty broke it
        // into. The web view wraps continuous text to the reader's width, and
        // re-wrapping already-wrapped text would break it a second time at the
        // pty's column boundary instead. Identical to `Recent` whenever no line
        // was long enough to wrap.
        state
            .herdr
            .read_pane(&pane, ReadSource::RecentUnwrapped, 80)
            .await
    };
    match read {
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

#[derive(Debug, Deserialize)]
pub struct KeysBody {
    /// herdr key names to press in order, e.g. ["down","enter"].
    pub keys: Vec<String>,
}

/// POST /api/panes/:pane/keys — send raw key presses (arrow keys, Enter, …) so
/// the human can drive a TUI option menu the reply textarea can't reach.
pub async fn send_keys(
    _auth: AuthSession,
    State(state): State<AppState>,
    Path(pane): Path<String>,
    Json(body): Json<KeysBody>,
) -> Response {
    match state.herdr.send_keys(&pane, &body.keys).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "ok": true }))).into_response(),
        Err(crate::herdr::HerdrError::NoSuchPane(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// DELETE /api/panes/:pane — close the pane, terminating whatever is running
/// in it. Destructive and immediate; the confirmation belongs on the client
/// side, before this call is ever made.
pub async fn close_pane(
    _auth: AuthSession,
    State(state): State<AppState>,
    Path(pane): Path<String>,
) -> Response {
    match state.herdr.close_pane(&pane).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "ok": true }))).into_response(),
        Err(crate::herdr::HerdrError::NoSuchPane(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct LabelBody {
    /// `None`/absent clears the label; `Some("")` is left to herdr to accept
    /// or reject rather than special-cased here (D2's "collect every naming
    /// decision at the source" applies just as well to herdr's own rules).
    #[serde(default)]
    pub label: Option<String>,
}

/// PUT /api/panes/:pane/label — set or clear the pane's operator-facing
/// label (herdr's `pane.rename`). Idempotent: setting the same label twice,
/// or clearing an already-unset one, both succeed.
pub async fn set_label(
    _auth: AuthSession,
    State(state): State<AppState>,
    Path(pane): Path<String>,
    Json(body): Json<LabelBody>,
) -> Response {
    match state.herdr.rename_pane(&pane, body.label.as_deref()).await {
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
    async fn screen_reads_the_unwrapped_source() {
        // The web view wraps continuous text to the reader's width, so it needs
        // the logical line the program wrote -- not the physical lines the pty
        // broke it into, which would then be broken a second time at the pty's
        // own column boundary. The fake answers the two sources differently, so
        // this fails if the route ever goes back to asking for `recent`.
        let fake = std::sync::Arc::new(crate::herdr::fake::FakeHerdr::new());
        fake.seed_wrapped_pane(
            "w1:p1",
            "the quick brown\nfox jumps",
            "the quick brown fox jumps",
        )
        .await;
        let state = AppState::new(
            fake,
            Some("s3cret-token".into()),
            crate::herdr::HERDR_PROTOCOL,
        );
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
        assert_eq!(s["text"].as_str().unwrap(), "the quick brown fox jumps");
    }

    #[tokio::test]
    async fn screen_history_param_routes_through_pane_scroller() {
        // ?history=1 must route through PaneScroller::read_history and
        // return its result mapped into the same ScreenBody shape (CONTEXT.md
        // D9/D11) -- distinct from the default (no-param) path, which stays
        // the pre-existing herdr::read_pane(Recent, 80) call unchanged.
        let fake = std::sync::Arc::new(crate::herdr::fake::FakeHerdr::new());
        let history_text = "line 1\nline 2\nline 3\n❯ ";
        fake.seed_scroll_pane("w1:p1", "❯ ", history_text, None);
        let state = AppState::new(
            fake,
            Some("s3cret-token".into()),
            crate::herdr::HERDR_PROTOCOL,
        );
        let cookie = test_login_cookie(&state).await;
        let app = api_router(state);
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/panes/w1:p1/screen?history=1")
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
        assert_eq!(s["text"].as_str().unwrap(), history_text);
        assert_eq!(s["revision"].as_u64().unwrap(), 1);
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
        let read = state
            .herdr
            .read_pane("w1:p1", ReadSource::Recent, 80)
            .await
            .unwrap();
        assert!(read.text.contains("do it"));
    }

    #[tokio::test]
    async fn keys_land_and_bump_screen() {
        let state = test_state();
        let cookie = test_login_cookie(&state).await;
        let app = api_router(state.clone());
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/panes/w1:p1/keys")
                    .header(header::COOKIE, &cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"keys":["down","enter"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let read = state
            .herdr
            .read_pane("w1:p1", ReadSource::Recent, 80)
            .await
            .unwrap();
        assert!(read.text.contains("<down>"));
    }

    #[tokio::test]
    async fn keys_to_unknown_pane_is_404() {
        let state = test_state();
        let cookie = test_login_cookie(&state).await;
        let app = api_router(state);
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/panes/nope/keys")
                    .header(header::COOKIE, cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"keys":["up"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn close_pane_requires_auth() {
        let app = api_router(test_state());
        let res = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/panes/w1:p1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND); // opaque, no leak
    }

    #[tokio::test]
    async fn close_pane_removes_it() {
        let state = test_state();
        let cookie = test_login_cookie(&state).await;
        let app = api_router(state.clone());
        let res = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/panes/w1:p1")
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
        assert_eq!(s["ok"], true);

        // The pane is actually gone, not just acknowledged.
        assert!(matches!(
            state.herdr.read_pane("w1:p1", ReadSource::Visible, 0).await,
            Err(crate::herdr::HerdrError::NoSuchPane(_))
        ));
    }

    #[tokio::test]
    async fn close_unknown_pane_is_404() {
        let state = test_state();
        let cookie = test_login_cookie(&state).await;
        let app = api_router(state);
        let res = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/panes/nope")
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn label_requires_auth() {
        let app = api_router(test_state());
        let res = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/panes/w1:p1/label")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"label":"API fix"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND); // opaque, no leak
    }

    #[tokio::test]
    async fn label_sets_and_reads_back_through_the_agents_list() {
        let state = test_state();
        let cookie = test_login_cookie(&state).await;
        let app = api_router(state.clone());
        let res = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/panes/w1:p1/label")
                    .header(header::COOKIE, &cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"label":"API fix"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // The label lives on panes[], never on agents[] -- matches real
        // herdr (see Agent's own doc comment in wire.rs).
        let snap = state.herdr.snapshot().await.unwrap();
        assert_eq!(snap.label_for_pane_id("w1:p1").as_deref(), Some("API fix"));
    }

    #[tokio::test]
    async fn label_absent_clears_it() {
        let state = test_state();
        let cookie = test_login_cookie(&state).await;
        state
            .herdr
            .rename_pane("w1:p1", Some("API fix"))
            .await
            .unwrap();
        let app = api_router(state.clone());
        let res = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/panes/w1:p1/label")
                    .header(header::COOKIE, &cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let snap = state.herdr.snapshot().await.unwrap();
        assert_eq!(snap.label_for_pane_id("w1:p1"), None);
    }

    #[tokio::test]
    async fn label_on_unknown_pane_is_404() {
        let state = test_state();
        let cookie = test_login_cookie(&state).await;
        let app = api_router(state);
        let res = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/panes/nope/label")
                    .header(header::COOKIE, cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"label":"x"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
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

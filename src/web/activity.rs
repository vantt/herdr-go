//! Activity live tail — the semantic sibling of `screen::read_screen`. Where
//! the screen endpoint returns whatever pixels happen to be on the pane's
//! 80-line grid at poll time, this one tails the agent's own on-disk session
//! transcript (`src/transcript`), so every command, output, and message
//! appended between polls arrives — nothing scrolls away unseen. Stateless
//! per request like every other route: the cursor round-trips through the
//! client, the gateway remembers nothing and stores nothing.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use super::auth::AuthSession;
use super::AppState;
use crate::transcript::{self, TranscriptError};

#[derive(Debug, Deserialize)]
pub struct ActivityQuery {
    /// Opaque cursor from the previous response; absent means "start tailing
    /// from now" (open = EOF, no retroactive history).
    pub cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ActivityBody {
    /// False when no transcript resolves for this pane (not a Claude Code
    /// pane, or none recorded yet) — the pane itself still exists, so this is
    /// data, not a 404.
    pub available: bool,
    pub lines: Vec<String>,
    pub cursor: Option<String>,
}

impl ActivityBody {
    fn unavailable() -> Self {
        ActivityBody {
            available: false,
            lines: Vec::new(),
            cursor: None,
        }
    }
}

/// GET /api/panes/:pane/activity — freshly appended transcript lines since
/// `cursor`. The pane's working directory (`foreground_cwd ?? cwd`, D5's
/// precedence) locates the transcript; the pane must exist in the snapshot
/// (404 otherwise, same contract as the screen route).
pub async fn read_activity(
    _auth: AuthSession,
    State(state): State<AppState>,
    Path(pane): Path<String>,
    Query(query): Query<ActivityQuery>,
) -> Response {
    let snap = match state.herdr.snapshot().await {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };
    // panes[] is a superset of agents[] (wire.rs), so agent panes resolve
    // here too; a pane the snapshot doesn't know is the same 404 as the
    // screen route's NoSuchPane.
    let Some(p) = snap.panes.iter().find(|p| p.pane_id == pane) else {
        if snap.agents.iter().any(|a| a.pane_id == pane) {
            // Known agent whose pane row carries no folder — no way to
            // locate a transcript, but the pane is real.
            return Json(ActivityBody::unavailable()).into_response();
        }
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(cwd) = p.foreground_cwd.clone().or_else(|| p.cwd.clone()) else {
        return Json(ActivityBody::unavailable()).into_response();
    };
    // File I/O off the async worker; the reads are small (bounded per poll)
    // but disk latency is not ours to inflict on the runtime.
    let cursor = query.cursor;
    let read =
        tokio::task::spawn_blocking(move || transcript::read_activity(&cwd, cursor.as_deref()))
            .await;
    match read {
        Ok(Ok(chunk)) => Json(ActivityBody {
            available: true,
            lines: chunk.lines,
            cursor: Some(chunk.cursor),
        })
        .into_response(),
        Ok(Err(TranscriptError::NotAvailable)) => Json(ActivityBody::unavailable()).into_response(),
        Ok(Err(TranscriptError::BadCursor)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "malformed activity cursor" })),
        )
            .into_response(),
        Ok(Err(TranscriptError::Io(e))) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
        Err(join) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": join.to_string() })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use crate::web::{api_router, test_login_cookie, test_state};
    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use tower::ServiceExt;

    async fn get(
        state: crate::web::AppState,
        uri: &str,
        cookie: Option<&str>,
    ) -> (StatusCode, serde_json::Value) {
        let mut builder = Request::builder().uri(uri);
        if let Some(c) = cookie {
            builder = builder.header(header::COOKIE, c);
        }
        let res = api_router(state)
            .oneshot(builder.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = if bytes.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, body)
    }

    #[tokio::test]
    async fn activity_requires_auth() {
        // Same opaque 404 as every protected route.
        let (status, _) = get(test_state(), "/api/panes/w1:p1/activity", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn activity_unknown_pane_is_404() {
        let state = test_state();
        let cookie = test_login_cookie(&state).await;
        let (status, _) = get(state, "/api/panes/nope/activity", Some(&cookie)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn activity_pane_without_transcript_reports_unavailable() {
        // The fake's w1:p1 cwd (/home/dev/projects/frontend-app) has no
        // transcript on any test machine — the route answers 200 with
        // available:false, never an error: absence of a transcript is data.
        let state = test_state();
        let cookie = test_login_cookie(&state).await;
        let (status, body) = get(state, "/api/panes/w1:p1/activity", Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["available"], false);
        assert_eq!(body["cursor"], serde_json::Value::Null);
        assert_eq!(body["lines"].as_array().unwrap().len(), 0);
    }
}

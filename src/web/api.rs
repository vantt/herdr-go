//! Switcher + health API. `agents` flattens herdr's snapshot into the switcher
//! list; `health` is a lightweight liveness + protocol probe.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use super::auth::AuthSession;
use super::AppState;

/// One switcher row. `pane_id` is the opaque address the screen/input endpoints
/// take; `status` drives the badge colour.
#[derive(Debug, Serialize)]
pub struct AgentRow {
    pub pane_id: String,
    pub workspace: String,
    pub display: String,
    pub kind: String,
    pub status: String,
    pub title: String,
    pub workspace_label: String,
    pub tab_label: String,
    pub workspace_status: String,
}

/// GET /api/agents — switcher list, resolved fresh from a snapshot each call.
pub async fn agents(_auth: AuthSession, State(state): State<AppState>) -> Response {
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
    let rows: Vec<AgentRow> = snap
        .agents
        .iter()
        .map(|a| AgentRow {
            pane_id: a.pane_id.clone(),
            workspace: a.workspace_id.clone(),
            display: crate::herdr::Snapshot::display_for(a),
            kind: a.kind.clone(),
            status: a.status.as_str().to_string(),
            title: a.title.clone(),
            workspace_label: snap.workspace_label_for(a),
            tab_label: snap.tab_label_for(a),
            workspace_status: snap.workspace_status_for(a).as_str().to_string(),
        })
        .collect();
    Json(rows).into_response()
}

#[derive(Debug, Serialize)]
pub struct Health {
    pub version: &'static str,
    pub protocol: u32,
    pub herdr_up: bool,
}

/// GET /api/health — liveness + protocol + whether herdr answers.
pub async fn health(State(state): State<AppState>) -> Response {
    let herdr_up = state.herdr.ping().await.is_ok();
    Json(Health {
        version: state.version,
        protocol: state.protocol,
        herdr_up,
    })
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::{api_router, test_state};
    use axum::body::Body;
    use axum::http::{header, Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn agents_lists_flat_snapshot() {
        let state = test_state();
        let cookie = crate::web::test_login_cookie(&state).await;
        let app = api_router(state);
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents")
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
        let rows: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(rows.len(), 4);
        let statuses: Vec<&str> = rows.iter().map(|r| r["status"].as_str().unwrap()).collect();
        assert!(statuses.contains(&"working"));
        assert!(statuses.contains(&"blocked"));
        // workspace_label/tab_label are present on every row (fall back to
        // empty string on a join miss, never absent or null).
        for row in &rows {
            assert!(row["workspace_label"].is_string());
            assert!(row["tab_label"].is_string());
            assert!(row["workspace_status"].is_string());
        }
    }

    #[tokio::test]
    async fn health_reports_up_and_protocol() {
        let app = api_router(test_state());
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let h: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(h["herdr_up"], true);
        assert_eq!(h["protocol"], 16);
    }
}

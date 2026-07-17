//! Switcher + health API. `agents` flattens the herdr snapshot into the
//! portrait switcher list the UI renders; `health` is a lightweight liveness +
//! protocol probe.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use super::auth::AuthSession;
use super::AppState;

/// One switcher row: a human-readable path plus the opaque pane id the terminal
/// screen addresses. Status drives the badge colour in the UI.
#[derive(Debug, Serialize)]
pub struct AgentRow {
    pub workspace: String,
    pub tab: String,
    pub pane_id: String,
    pub display: String,
    pub kind: String,
    pub status: String,
}

/// GET /api/agents — the switcher list, resolved fresh from a snapshot every
/// call (never cached — PRD §6). Requires a valid session.
pub async fn agents(_auth: AuthSession, State(state): State<AppState>) -> Response {
    let snap = match state.control.snapshot().await {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };

    let mut rows = Vec::new();
    for ws in &snap.workspaces {
        for tab in &ws.tabs {
            for pane in &tab.panes {
                if let Some(agent) = &pane.agent {
                    rows.push(AgentRow {
                        workspace: ws.label.clone(),
                        tab: tab.label.clone(),
                        pane_id: pane.id.clone(),
                        display: format!("{} › {} › {}", ws.label, tab.label, agent.kind),
                        kind: agent.kind.clone(),
                        status: agent.status.as_str().to_string(),
                    });
                }
            }
        }
    }
    Json(rows).into_response()
}

#[derive(Debug, Serialize)]
pub struct Health {
    pub version: &'static str,
    pub protocol: u32,
    pub herdr_up: bool,
}

/// GET /api/health — liveness + protocol + whether herdr answers. Unauthenticated
/// so a load balancer / systemd check can hit it, but it reveals nothing
/// sensitive (no agent list, no paths).
pub async fn health(State(state): State<AppState>) -> Response {
    let herdr_up = state.control.ping().await.is_ok();
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

    async fn login_cookie(state: &AppState) -> String {
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
        res.headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string()
    }

    #[tokio::test]
    async fn agents_lists_all_seeded_panes() {
        let state = test_state();
        let cookie = login_cookie(&state).await;
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
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let rows: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(rows.len(), 4);
        let statuses: Vec<&str> = rows.iter().map(|r| r["status"].as_str().unwrap()).collect();
        assert!(statuses.contains(&"working"));
        assert!(statuses.contains(&"blocked"));
    }

    #[tokio::test]
    async fn health_reports_up_and_protocol() {
        let app = api_router(test_state());
        let res = app
            .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let h: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(h["herdr_up"], true);
        assert_eq!(h["protocol"], 16);
    }
}

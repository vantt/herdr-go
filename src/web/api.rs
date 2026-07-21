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

/// One destination row (CONTEXT.md P2): `path` is `anchor_for_workspace`'s
/// answer, null on a join miss — the row still ships. `path_is_live` is true
/// only when `path` came from the pane's live `foreground_cwd`, never a
/// substitute for a missing path.
#[derive(Debug, Serialize)]
pub struct Destination {
    pub workspace_id: String,
    pub label: String,
    pub path: Option<String>,
    pub path_is_live: bool,
}

/// One agent-create preset, label only — `argv` is operator-authored and must
/// never leave the process (CONTEXT.md P6).
#[derive(Debug, Serialize)]
pub struct PresetOption {
    pub label: String,
}

#[derive(Debug, Serialize)]
pub struct CreateOptions {
    pub destinations: Vec<Destination>,
    pub presets: Vec<PresetOption>,
}

/// GET /api/create-options — the create sheet's FAB opens on one fetch
/// (CONTEXT.md P4): every workspace as a destination, including one with no
/// agents (P1), plus the operator's agent presets.
pub async fn create_options(_auth: AuthSession, State(state): State<AppState>) -> Response {
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
    let destinations: Vec<Destination> = snap
        .workspaces
        .iter()
        .map(|w| {
            let anchor = snap.anchor_for_workspace(&w.workspace_id);
            Destination {
                workspace_id: w.workspace_id.clone(),
                label: w.label.clone(),
                path: anchor.as_ref().map(|a| a.path.clone()),
                path_is_live: anchor.map(|a| a.live).unwrap_or(false),
            }
        })
        .collect();
    let presets: Vec<PresetOption> = state
        .agent_presets
        .iter()
        .map(|p| PresetOption {
            label: p.label.clone(),
        })
        .collect();
    Json(CreateOptions {
        destinations,
        presets,
    })
    .into_response()
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

    // --- GET /api/create-options (cell web-create-endpoints-3) -------------

    async fn get_create_options(state: AppState) -> (StatusCode, serde_json::Value) {
        let cookie = crate::web::test_login_cookie(&state).await;
        let app = api_router(state);
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/create-options")
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = res.status();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, serde_json::from_slice(&body).unwrap())
    }

    #[tokio::test]
    async fn createoptions_requires_auth() {
        // Unauthenticated: the same opaque 404 as any other route, never a
        // descriptive rejection (CONTEXT.md P7's 404 stays reserved for this).
        let app = api_router(test_state());
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/create-options")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn createoptions_agentless_workspace_appears_in_destinations() {
        // w3 in the fake seed has zero agents -- the exact case /api/agents
        // structurally drops (P1).
        let (status, body) = get_create_options(test_state()).await;
        assert_eq!(status, StatusCode::OK);
        let destinations = body["destinations"].as_array().unwrap();
        assert!(destinations
            .iter()
            .any(|d| d["workspace_id"] == "w3" && d["label"] == "backend-api"));
    }

    #[tokio::test]
    async fn createoptions_path_is_live_true_when_foreground_cwd_present() {
        // w1's anchor pane (w1:p1) has foreground_cwd == cwd -- the live case.
        let (status, body) = get_create_options(test_state()).await;
        assert_eq!(status, StatusCode::OK);
        let destinations = body["destinations"].as_array().unwrap();
        let w1 = destinations
            .iter()
            .find(|d| d["workspace_id"] == "w1")
            .unwrap();
        assert_eq!(w1["path_is_live"], true);
        assert_eq!(w1["path"], "/home/dev/projects/frontend-app");
    }

    #[tokio::test]
    async fn createoptions_path_is_live_false_when_only_cwd_present() {
        // w3's anchor pane (w3:p6) has cwd but no foreground_cwd -- the
        // fallback case CONTEXT.md P2/P8 carries as data, not a platform branch.
        let (status, body) = get_create_options(test_state()).await;
        assert_eq!(status, StatusCode::OK);
        let destinations = body["destinations"].as_array().unwrap();
        let w3 = destinations
            .iter()
            .find(|d| d["workspace_id"] == "w3")
            .unwrap();
        assert_eq!(w3["path_is_live"], false);
        assert_eq!(w3["path"], "/home/dev/projects/backend-api");
    }

    #[tokio::test]
    async fn createoptions_no_argv_anywhere_in_response_body() {
        // Presets carry argv internally, but only the label may ever reach
        // the client (CONTEXT.md P6).
        let state = test_state().with_agent_presets(vec![crate::config::AgentPreset {
            label: "Claude".to_string(),
            argv: vec!["claude".to_string(), "--dangerous-secret-flag".to_string()],
        }]);
        let (status, body) = get_create_options(state).await;
        assert_eq!(status, StatusCode::OK);
        let raw = body.to_string();
        assert!(!raw.contains("argv"));
        assert!(!raw.contains("--dangerous-secret-flag"));
    }

    #[tokio::test]
    async fn createoptions_presets_expose_label_only() {
        let state = test_state().with_agent_presets(vec![crate::config::AgentPreset {
            label: "Claude".to_string(),
            argv: vec!["claude".to_string()],
        }]);
        let (status, body) = get_create_options(state).await;
        assert_eq!(status, StatusCode::OK);
        let presets = body["presets"].as_array().unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0]["label"], "Claude");
        assert_eq!(
            presets[0].as_object().unwrap().keys().collect::<Vec<_>>(),
            vec!["label"]
        );
    }

    #[tokio::test]
    async fn createoptions_snapshot_error_is_502() {
        let fake = std::sync::Arc::new(crate::herdr::fake::FakeHerdr::new());
        fake.set_available(false);
        let state = crate::web::AppState::new(fake, Some("s3cret-token".into()), 16);
        let (status, body) = get_create_options(state).await;
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert!(body["error"].is_string());
    }
}

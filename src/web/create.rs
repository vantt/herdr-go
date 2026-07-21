//! The create surface — the phone's two write verbs: `POST /api/panes` opens a
//! plain shell, `POST /api/agents` starts a named agent. Both seed the new
//! terminal with the destination workspace's anchor folder (D5) and, like every
//! mutating endpoint, copy `send_reply`'s extractor order — `AuthSession` first
//! (P9), then `State`, then the JSON body (`src/web/screen.rs:56-71`).
//!
//! The two verbs are **not** symmetric when the anchor path fails to resolve
//! (CONTEXT.md P10). `tab.create` omits `cwd` and lets herdr resolve the
//! workspace's own anchor, exactly as the desktop does
//! (`upstreams/herdr/src/app/api/tabs.rs:65-67`). `agent.start` must never omit
//! it: its fallback is herdr's **own process directory**
//! (`upstreams/herdr/src/app/agents.rs:118-122`), an arbitrary folder — the
//! silent wrong-repo start D5 exists to forbid — so an agent create into an
//! unresolved destination is refused with 409 rather than started there.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use super::auth::AuthSession;
use super::AppState;
use crate::herdr::HerdrError;

/// `POST /api/panes` body. Only the destination is accepted — a shell takes no
/// command. No `cwd`/`argv`/`env` field exists to receive anything the client
/// might try to send (P6).
#[derive(Debug, Deserialize)]
pub struct CreatePaneBody {
    pub workspace_id: String,
}

/// `POST /api/agents` body. The agent is named by a preset **label**, never by
/// `argv` (P6): the argv is operator-authored config the label keys into, and
/// the request cannot influence it. No argv/env field is deserialized at all.
#[derive(Debug, Deserialize)]
pub struct CreateAgentBody {
    pub workspace_id: String,
    pub preset: String,
}

/// POST /api/panes — open a plain shell in `workspace_id`, seeded with the
/// destination's anchor folder when it resolves and omitting `cwd` when it does
/// not (herdr then resolves the workspace anchor itself — the safe,
/// desktop-equivalent fallback, CONTEXT.md P10).
pub async fn create_pane(
    _auth: AuthSession,
    State(state): State<AppState>,
    Json(body): Json<CreatePaneBody>,
) -> Response {
    let cwd = match state.herdr.snapshot().await {
        Ok(snap) => snap.anchor_cwd_for_workspace(&body.workspace_id),
        Err(e) => return herdr_error_response(e),
    };
    match state
        .herdr
        .tab_create(&body.workspace_id, cwd.as_deref())
        .await
    {
        Ok(created) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "tab_id": created.tab_id,
                "pane_id": created.pane_id,
            })),
        )
            .into_response(),
        Err(e) => herdr_error_response(e),
    }
}

/// POST /api/agents — start an agent in `workspace_id` running the preset's
/// configured argv. An unknown label is refused with 400 before the port is
/// ever called; a destination whose anchor does not resolve is refused with
/// 409 rather than started in an arbitrary folder (CONTEXT.md P10).
pub async fn create_agent(
    _auth: AuthSession,
    State(state): State<AppState>,
    Json(body): Json<CreateAgentBody>,
) -> Response {
    let argv = match state.agent_presets.iter().find(|p| p.label == body.preset) {
        Some(preset) => preset.argv.clone(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("unknown agent preset: {}", body.preset),
                })),
            )
                .into_response()
        }
    };

    let anchor = match state.herdr.snapshot().await {
        Ok(snap) => snap.anchor_cwd_for_workspace(&body.workspace_id),
        Err(e) => return herdr_error_response(e),
    };
    let Some(cwd) = anchor else {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!(
                    "destination {} has no resolved path; refusing to start an agent in an arbitrary directory",
                    body.workspace_id
                ),
            })),
        )
            .into_response();
    };

    match state
        .herdr
        .agent_start(&body.workspace_id, Some(&cwd), &argv)
        .await
    {
        Ok(started) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "tab_id": started.tab_id,
                "pane_id": started.pane_id,
                "name": started.name,
            })),
        )
            .into_response(),
        Err(e) => herdr_error_response(e),
    }
}

/// Map a herdr port error onto the create routes' HTTP surface (CONTEXT.md
/// error table). A destination that no longer exists is **409** — never 404,
/// which stays reserved for the opaque unauthenticated answer (P7) so the sheet
/// can tell "that workspace closed" apart from "log in again". Everything else
/// is a 502 relaying the message.
fn herdr_error_response(err: HerdrError) -> Response {
    let conflict = matches!(err, HerdrError::WorkspaceNotFound { .. })
        || matches!(
            &err,
            HerdrError::Remote { code, .. }
                if code == "agent_placement_not_found" || code == "agent_placement_conflict"
        );
    let status = if conflict {
        StatusCode::CONFLICT
    } else {
        StatusCode::BAD_GATEWAY
    };
    (
        status,
        Json(serde_json::json!({ "error": err.to_string() })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use crate::herdr::wire::{PaneLayout, Snapshot, Tab, Workspace};
    use crate::herdr::{
        AgentStarted, AgentStatus, Herdr, ProtocolInfo, Result, ScreenRead, TabCreated,
    };
    use crate::web::{api_router, test_login_cookie, test_state, AppState};
    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    fn preset_state() -> AppState {
        test_state().with_agent_presets(vec![crate::config::AgentPreset {
            label: "Claude".to_string(),
            argv: vec!["claude".to_string()],
        }])
    }

    async fn post(
        state: AppState,
        uri: &str,
        body: &str,
        cookie: Option<&str>,
    ) -> (StatusCode, serde_json::Value) {
        let app = api_router(state);
        let mut req = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json");
        if let Some(c) = cookie {
            req = req.header(header::COOKIE, c);
        }
        let res = app
            .oneshot(req.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    #[tokio::test]
    async fn createroute_shell_creates_and_returns_ids() {
        let state = test_state();
        let cookie = test_login_cookie(&state).await;
        let (status, body) = post(
            state,
            "/api/panes",
            r#"{"workspace_id":"w1"}"#,
            Some(&cookie),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["tab_id"].as_str().unwrap().contains("w1"));
        assert!(body["pane_id"].as_str().unwrap().contains("w1"));
    }

    #[tokio::test]
    async fn createroute_agent_creates_and_returns_ids_and_name() {
        let state = preset_state();
        let cookie = test_login_cookie(&state).await;
        let (status, body) = post(
            state,
            "/api/agents",
            r#"{"workspace_id":"w1","preset":"Claude"}"#,
            Some(&cookie),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["tab_id"].is_string());
        assert!(body["pane_id"].is_string());
        assert!(!body["name"].as_str().unwrap().is_empty());
    }

    #[tokio::test]
    async fn createroute_agent_unknown_preset_is_400_and_reaches_no_port() {
        let state = preset_state();
        let cookie = test_login_cookie(&state).await;
        let before = state.herdr.snapshot().await.unwrap().agents.len();
        let (status, _body) = post(
            state.clone(),
            "/api/agents",
            r#"{"workspace_id":"w1","preset":"does-not-exist"}"#,
            Some(&cookie),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        // A body-supplied label the operator never configured never reaches the
        // port: nothing is created (P6 — the request cannot pick what runs).
        assert_eq!(state.herdr.snapshot().await.unwrap().agents.len(), before);
    }

    #[tokio::test]
    async fn createroute_shell_stale_destination_is_409_not_404() {
        let state = test_state();
        let cookie = test_login_cookie(&state).await;
        let (status, _body) = post(
            state,
            "/api/panes",
            r#"{"workspace_id":"ghost-workspace"}"#,
            Some(&cookie),
        )
        .await;
        // A vanished destination is 409, never the opaque 404 reserved for auth.
        assert_eq!(status, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn createroute_generic_port_failure_is_502() {
        let fake = Arc::new(crate::herdr::fake::FakeHerdr::new());
        fake.set_available(false);
        let state = AppState::new(fake, Some("s3cret-token".into()), 16);
        let cookie = test_login_cookie(&state).await;
        let (status, body) = post(
            state,
            "/api/panes",
            r#"{"workspace_id":"w1"}"#,
            Some(&cookie),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert!(body["error"].is_string());
    }

    #[tokio::test]
    async fn createroute_both_routes_unauthenticated_are_404() {
        let (pane_status, _) =
            post(test_state(), "/api/panes", r#"{"workspace_id":"w1"}"#, None).await;
        assert_eq!(pane_status, StatusCode::NOT_FOUND);
        let (agent_status, _) = post(
            preset_state(),
            "/api/agents",
            r#"{"workspace_id":"w1","preset":"Claude"}"#,
            None,
        )
        .await;
        assert_eq!(agent_status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn createroute_agent_unresolved_path_is_refused_409() {
        // A destination whose anchor does not resolve refuses an agent with 409
        // rather than starting it in herdr's own process dir (P10). A
        // nonexistent workspace resolves to no anchor, the same as a stale one.
        let state = preset_state();
        let cookie = test_login_cookie(&state).await;
        let (status, body) = post(
            state,
            "/api/agents",
            r#"{"workspace_id":"ghost-workspace","preset":"Claude"}"#,
            Some(&cookie),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert!(body["error"].as_str().unwrap().contains("ghost-workspace"));
    }

    // --- The asymmetric branch, against a controllable double ---------------
    //
    // FakeHerdr's seed has no workspace that both exists AND fails to resolve an
    // anchor, so the shell "omit cwd" side of P10 and the exact cwd seeded on a
    // resolved destination are proven here against a stub that records the
    // `cwd` each verb received.

    #[derive(Default)]
    struct RecordingHerdr {
        snap: Snapshot,
        tab_cwd: Mutex<Option<Option<String>>>,
        agent_cwd: Mutex<Option<Option<String>>>,
    }

    #[async_trait::async_trait]
    impl Herdr for RecordingHerdr {
        async fn snapshot(&self) -> Result<Snapshot> {
            Ok(self.snap.clone())
        }
        async fn ping(&self) -> Result<ProtocolInfo> {
            unreachable!("create routes never ping")
        }
        async fn read_pane(&self, _pane_id: &str) -> Result<ScreenRead> {
            unreachable!("create routes never read")
        }
        async fn send_input(&self, _pane_id: &str, _text: &str, _submit: bool) -> Result<()> {
            unreachable!("create routes never send input")
        }
        async fn send_keys(&self, _pane_id: &str, _keys: &[String]) -> Result<()> {
            unreachable!("create routes never send keys")
        }
        async fn tab_create(&self, workspace_id: &str, cwd: Option<&str>) -> Result<TabCreated> {
            *self.tab_cwd.lock().unwrap() = Some(cwd.map(str::to_string));
            Ok(TabCreated {
                tab_id: format!("{workspace_id}:t-new"),
                pane_id: format!("{workspace_id}:p-new"),
            })
        }
        async fn agent_start(
            &self,
            workspace_id: &str,
            cwd: Option<&str>,
            _argv: &[String],
        ) -> Result<AgentStarted> {
            *self.agent_cwd.lock().unwrap() = Some(cwd.map(str::to_string));
            Ok(AgentStarted {
                tab_id: format!("{workspace_id}:t-new"),
                pane_id: format!("{workspace_id}:p-new"),
                name: "mobile-agent-stub".to_string(),
            })
        }
    }

    /// A workspace with a fully resolvable anchor at `path`.
    fn resolvable(id: &str, path: &str) -> Snapshot {
        use crate::herdr::wire::Pane;
        Snapshot {
            workspaces: vec![Workspace {
                workspace_id: id.into(),
                label: id.into(),
                agent_status: AgentStatus::Idle,
                active_tab_id: Some(format!("{id}:t")),
            }],
            tabs: vec![Tab {
                tab_id: format!("{id}:t"),
                label: "main".into(),
            }],
            layouts: vec![PaneLayout {
                workspace_id: id.into(),
                tab_id: format!("{id}:t"),
                focused_pane_id: Some(format!("{id}:p")),
            }],
            panes: vec![Pane {
                pane_id: format!("{id}:p"),
                workspace_id: id.into(),
                tab_id: format!("{id}:t"),
                cwd: Some(path.into()),
                foreground_cwd: Some(path.into()),
            }],
            ..Snapshot::default()
        }
    }

    /// A workspace that exists but whose anchor never resolves (no layout entry).
    fn unresolvable(id: &str) -> Snapshot {
        Snapshot {
            workspaces: vec![Workspace {
                workspace_id: id.into(),
                label: id.into(),
                agent_status: AgentStatus::Idle,
                active_tab_id: Some(format!("{id}:t")),
            }],
            ..Snapshot::default()
        }
    }

    fn stub_state(snap: Snapshot) -> (AppState, Arc<RecordingHerdr>) {
        let herdr = Arc::new(RecordingHerdr {
            snap,
            ..RecordingHerdr::default()
        });
        let state = AppState::new(herdr.clone(), Some("s3cret-token".into()), 16)
            .with_agent_presets(vec![crate::config::AgentPreset {
                label: "Claude".to_string(),
                argv: vec!["claude".to_string()],
            }]);
        (state, herdr)
    }

    #[tokio::test]
    async fn createroute_shell_unresolved_path_creates_with_cwd_omitted() {
        let (state, herdr) = stub_state(unresolvable("wx"));
        let cookie = test_login_cookie(&state).await;
        let (status, _body) = post(
            state,
            "/api/panes",
            r#"{"workspace_id":"wx"}"#,
            Some(&cookie),
        )
        .await;
        // The shell is still created; herdr is left to resolve the anchor.
        assert_eq!(status, StatusCode::OK);
        assert_eq!(*herdr.tab_cwd.lock().unwrap(), Some(None));
    }

    #[tokio::test]
    async fn createroute_resolved_anchor_seeds_cwd_on_both_routes() {
        let path = "/home/dev/projects/thing";

        let (shell_state, shell_herdr) = stub_state(resolvable("wok", path));
        let shell_cookie = test_login_cookie(&shell_state).await;
        let (shell_status, _) = post(
            shell_state,
            "/api/panes",
            r#"{"workspace_id":"wok"}"#,
            Some(&shell_cookie),
        )
        .await;
        assert_eq!(shell_status, StatusCode::OK);
        assert_eq!(
            *shell_herdr.tab_cwd.lock().unwrap(),
            Some(Some(path.to_string()))
        );

        let (agent_state, agent_herdr) = stub_state(resolvable("wok", path));
        let agent_cookie = test_login_cookie(&agent_state).await;
        let (agent_status, _) = post(
            agent_state,
            "/api/agents",
            r#"{"workspace_id":"wok","preset":"Claude"}"#,
            Some(&agent_cookie),
        )
        .await;
        assert_eq!(agent_status, StatusCode::OK);
        assert_eq!(
            *agent_herdr.agent_cwd.lock().unwrap(),
            Some(Some(path.to_string()))
        );
    }
}

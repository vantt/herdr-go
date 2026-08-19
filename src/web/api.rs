//! Switcher + health API. `agents` flattens herdr's snapshot into the switcher
//! list; `health` is a lightweight liveness + protocol probe.

use std::collections::HashSet;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use super::auth::AuthSession;
use super::AppState;

/// Shortens `path` to a `~/`-relative display form when it sits under `home`
/// (e.g. `/home/dev/projects` -> `~/projects` when `home` is `/home/dev`).
/// Passes through unchanged when `home` is `None`/empty or `path` isn't
/// under it. Display-only — never feeds back into pane creation or any
/// real filesystem operation.
fn tildify(path: String, home: Option<&str>) -> String {
    let Some(home) = home.filter(|h| !h.is_empty()) else {
        return path;
    };
    if path == home {
        return "~".to_string();
    }
    match path.strip_prefix(home) {
        Some(rest) if rest.starts_with('/') => format!("~{rest}"),
        _ => path,
    }
}

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
    /// The agent's own pane folder, joined via `Snapshot::path_for_pane_id`
    /// (`panes[]` is a superset of `agents[]`) -- `None` on a join miss.
    pub path: Option<String>,
    /// The operator's own pane label (herdr's `pane.rename`) -- `None` until
    /// set. Distinct from `title` above, which the running program sets.
    pub label: Option<String>,
}

/// One shell-only row (home-shell-workspaces D1-D7): a plain-shell pane
/// inside a workspace with zero agents. No `status`/`kind`/`display`/`title`
/// — there is no agent record to source them from (D2/D5).
#[derive(Debug, Serialize)]
pub struct ShellRow {
    pub pane_id: String,
    pub workspace_id: String,
    pub workspace_label: String,
    pub tab_label: String,
    pub path: Option<String>,
    /// The operator's own pane label (herdr's `pane.rename`) -- `None` until
    /// set.
    pub label: Option<String>,
}

/// GET /api/agents's response: the agent list plus every plain-shell pane
/// (home-shell-workspaces), both resolved from the same snapshot fetch (one
/// round trip).
#[derive(Debug, Serialize)]
pub struct AgentsResponse {
    pub agents: Vec<AgentRow>,
    pub shells: Vec<ShellRow>,
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
    let home = std::env::var("HOME").ok();
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
            path: snap
                .path_for_pane_id(&a.pane_id)
                .map(|p| tildify(p, home.as_deref())),
            label: snap.label_for_pane_id(&a.pane_id),
        })
        .collect();
    // `panes[]` is a superset of `agents[]` (every agent pane also has a
    // `Pane` entry) -- excluded here so an agent's own pane never also shows
    // as a duplicate plain-shell row. Unlike the removed hsw-D3 rule, a shell
    // pane surfaces regardless of whether its workspace has other agents: an
    // operator who creates a shell via a workspace's own quick-add
    // (`renderWorkspaceSection`'s `.workspace-header-add`) expects to find it
    // on the agent list afterward, the same as any other pane (feature
    // `shell-visible-alongside-agents`).
    let agent_pane_ids: HashSet<&str> = snap.agents.iter().map(|a| a.pane_id.as_str()).collect();
    let shells: Vec<ShellRow> = snap
        .panes
        .iter()
        .filter(|p| !agent_pane_ids.contains(p.pane_id.as_str()))
        .map(|p| ShellRow {
            pane_id: p.pane_id.clone(),
            workspace_id: p.workspace_id.clone(),
            workspace_label: snap.workspace_label_for_id(&p.workspace_id),
            tab_label: snap.tab_label_for_id(&p.tab_id),
            path: p
                .foreground_cwd
                .clone()
                .or(p.cwd.clone())
                .map(|p| tildify(p, home.as_deref())),
            label: p.label.clone(),
        })
        .collect();
    Json(AgentsResponse {
        agents: rows,
        shells,
    })
    .into_response()
}

/// One destination row (CONTEXT.md P2): `path` is
/// `web::resolve_workspace_git_anchor`'s answer — the resolved anchor,
/// walked up to its nearest enclosing git root when one exists — null on a
/// join miss, the row still ships. `path_is_live` is true only when the
/// underlying anchor (before any git-root walk-up) came from the pane's live
/// `foreground_cwd`, never a substitute for a missing path.
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
/// agents (P1), plus the operator's agent presets. Destinations whose anchor
/// resolves to (or walks up to) a git root are sorted first — a stable sort,
/// so within each group the snapshot's own workspace order is unchanged.
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
    let home = std::env::var("HOME").ok();
    let mut resolved: Vec<(Destination, bool)> = snap
        .workspaces
        .iter()
        .map(|w| {
            let (path, path_is_live, is_git_root) =
                match super::resolve_workspace_git_anchor(&snap, &w.workspace_id) {
                    Some((anchor, is_git_root)) => (
                        Some(tildify(anchor.path, home.as_deref())),
                        anchor.live,
                        is_git_root,
                    ),
                    None => (None, false, false),
                };
            (
                Destination {
                    workspace_id: w.workspace_id.clone(),
                    label: w.label.clone(),
                    path,
                    path_is_live,
                },
                is_git_root,
            )
        })
        .collect();
    resolved.sort_by_key(|(_, is_git_root)| !is_git_root);
    let destinations: Vec<Destination> = resolved.into_iter().map(|(dest, _)| dest).collect();
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

    #[test]
    fn tildify_shortens_path_under_home() {
        assert_eq!(
            tildify("/home/dev/projects/app".into(), Some("/home/dev")),
            "~/projects/app"
        );
    }

    #[test]
    fn tildify_collapses_home_itself_to_tilde() {
        assert_eq!(tildify("/home/dev".into(), Some("/home/dev")), "~");
    }

    #[test]
    fn tildify_leaves_path_outside_home_untouched() {
        assert_eq!(tildify("/srv/data".into(), Some("/home/dev")), "/srv/data");
    }

    #[test]
    fn tildify_rejects_sibling_prefix_match() {
        // "/home/devops" must not be mistaken for a subpath of "/home/dev".
        assert_eq!(
            tildify("/home/devops/app".into(), Some("/home/dev")),
            "/home/devops/app"
        );
    }

    #[test]
    fn tildify_passes_through_when_home_unknown() {
        assert_eq!(tildify("/home/dev/app".into(), None), "/home/dev/app");
        assert_eq!(tildify("/home/dev/app".into(), Some("")), "/home/dev/app");
    }

    async fn get_agents(state: AppState) -> (StatusCode, serde_json::Value) {
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
        let status = res.status();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, serde_json::from_slice(&body).unwrap())
    }

    #[tokio::test]
    async fn agents_lists_flat_snapshot() {
        let (status, body) = get_agents(test_state()).await;
        assert_eq!(status, StatusCode::OK);
        let rows = body["agents"].as_array().unwrap();
        assert_eq!(rows.len(), 4);
        let statuses: Vec<&str> = rows.iter().map(|r| r["status"].as_str().unwrap()).collect();
        assert!(statuses.contains(&"working"));
        assert!(statuses.contains(&"blocked"));
        // workspace_label/tab_label are present on every row (fall back to
        // empty string on a join miss, never absent or null).
        for row in rows {
            assert!(row["workspace_label"].is_string());
            assert!(row["tab_label"].is_string());
            assert!(row["workspace_status"].is_string());
        }
    }

    #[tokio::test]
    async fn agents_lists_label_null_until_set_then_the_operator_set_value() {
        let state = test_state();
        let (_, body) = get_agents(state.clone()).await;
        let rows = body["agents"].as_array().unwrap();
        let p1 = rows.iter().find(|r| r["pane_id"] == "w1:p1").unwrap();
        assert!(p1["label"].is_null());

        state
            .herdr
            .rename_pane("w1:p1", Some("API fix"))
            .await
            .unwrap();
        let (_, body) = get_agents(state).await;
        let rows = body["agents"].as_array().unwrap();
        let p1 = rows.iter().find(|r| r["pane_id"] == "w1:p1").unwrap();
        assert_eq!(p1["label"], "API fix");
    }

    #[tokio::test]
    async fn agent_row_path_joins_own_pane_folder() {
        let (status, body) = get_agents(test_state()).await;
        assert_eq!(status, StatusCode::OK);
        let rows = body["agents"].as_array().unwrap();
        let p1 = rows.iter().find(|r| r["pane_id"] == "w1:p1").unwrap();
        assert_eq!(p1["path"], "/home/dev/projects/frontend-app");
    }

    // --- GET /api/agents shells (cell home-shell-workspaces-1) -------------

    #[tokio::test]
    async fn homeshell_agentless_workspace_produces_shell_rows() {
        let (status, body) = get_agents(test_state()).await;
        assert_eq!(status, StatusCode::OK);
        let shells = body["shells"].as_array().unwrap();
        assert!(shells
            .iter()
            .any(|s| s["pane_id"] == "w3:p6" && s["workspace_id"] == "w3"));
    }

    #[tokio::test]
    async fn homeshell_workspace_with_agents_still_shows_its_own_shell_row() {
        // hsw-D3 (shell hidden whenever its workspace already has an agent) was
        // removed: an operator creating a shell via a workspace's own
        // quick-add expects to find it on the agent list afterward, even
        // when that workspace already has agent cards (feature
        // `shell-visible-alongside-agents`). w2:p5 is a plain shell pane
        // inside w2, which also has agent cards (w2:p3, w2:p4).
        let (status, body) = get_agents(test_state()).await;
        assert_eq!(status, StatusCode::OK);
        let shells = body["shells"].as_array().unwrap();
        assert!(shells
            .iter()
            .any(|s| s["pane_id"] == "w2:p5" && s["workspace_id"] == "w2"));
    }

    #[tokio::test]
    async fn homeshell_an_agents_own_pane_never_also_shows_as_a_shell_row() {
        // panes[] is a superset of agents[] -- w1:p1 has both an Agent entry
        // and a matching Pane entry. Without the pane-level dedup, it would
        // wrongly appear twice: once as an agent card, once as a shell row.
        let (status, body) = get_agents(test_state()).await;
        assert_eq!(status, StatusCode::OK);
        let shells = body["shells"].as_array().unwrap();
        assert!(!shells.iter().any(|s| s["pane_id"] == "w1:p1"));
    }

    #[tokio::test]
    async fn homeshell_multi_shell_workspace_produces_one_row_per_pane() {
        let (status, body) = get_agents(test_state()).await;
        assert_eq!(status, StatusCode::OK);
        let shells = body["shells"].as_array().unwrap();
        let w3_shells: Vec<_> = shells
            .iter()
            .filter(|s| s["workspace_id"] == "w3")
            .collect();
        assert_eq!(w3_shells.len(), 2, "w3 has 2 shell panes, each its own row");
    }

    #[tokio::test]
    async fn homeshell_shell_row_path_reads_own_pane_not_anchor() {
        let (status, body) = get_agents(test_state()).await;
        assert_eq!(status, StatusCode::OK);
        let shells = body["shells"].as_array().unwrap();
        let p6 = shells.iter().find(|s| s["pane_id"] == "w3:p6").unwrap();
        assert_eq!(p6["path"], "/home/dev/projects/backend-api");
        let p7 = shells.iter().find(|s| s["pane_id"] == "w3:p7").unwrap();
        assert_eq!(p7["path"], "/home/dev/projects/backend-api/scripts");
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

    // --- Destination git-root walk-up and sort -----------------------------

    /// A stub `Herdr` returning one fixed, caller-built snapshot — used where
    /// `FakeHerdr`'s built-in seed paths (which never exist on disk) can't
    /// exercise a real filesystem `.git` walk-up.
    struct FixedSnapshotHerdr {
        snap: crate::herdr::wire::Snapshot,
    }

    #[async_trait::async_trait]
    impl crate::herdr::Herdr for FixedSnapshotHerdr {
        async fn snapshot(&self) -> crate::herdr::Result<crate::herdr::wire::Snapshot> {
            Ok(self.snap.clone())
        }
        async fn ping(&self) -> crate::herdr::Result<crate::herdr::ProtocolInfo> {
            unreachable!("create-options never pings")
        }
        async fn read_pane(
            &self,
            _pane_id: &str,
            _source: crate::herdr::ReadSource,
            _lines: usize,
        ) -> crate::herdr::Result<crate::herdr::ScreenRead> {
            unreachable!("create-options never reads")
        }
        async fn send_input(
            &self,
            _pane_id: &str,
            _text: &str,
            _submit: bool,
        ) -> crate::herdr::Result<()> {
            unreachable!("create-options never sends input")
        }
        async fn send_keys(&self, _pane_id: &str, _keys: &[String]) -> crate::herdr::Result<()> {
            unreachable!("create-options never sends keys")
        }
        async fn send_text(&self, _pane_id: &str, _bytes: &str) -> crate::herdr::Result<()> {
            unreachable!("create-options never sends text")
        }
        async fn close_pane(&self, _pane_id: &str) -> crate::herdr::Result<()> {
            unreachable!("create-options never closes panes")
        }
        async fn rename_pane(
            &self,
            _pane_id: &str,
            _label: Option<&str>,
        ) -> crate::herdr::Result<()> {
            unreachable!("create-options never renames panes")
        }
        async fn tab_create(
            &self,
            _workspace_id: &str,
            _cwd: Option<&str>,
        ) -> crate::herdr::Result<crate::herdr::TabCreated> {
            unreachable!("create-options never creates tabs")
        }
        async fn agent_start(
            &self,
            _workspace_id: &str,
            _cwd: Option<&str>,
            _argv: &[String],
        ) -> crate::herdr::Result<crate::herdr::AgentStarted> {
            unreachable!("create-options never starts agents")
        }
    }

    /// Builds one workspace/tab/layout/pane group anchored at `path` — the
    /// minimal join `anchor_for_workspace` needs, built directly rather than
    /// through `FakeHerdr`'s fixed seed so `path` can point at a real temp dir.
    fn snapshot_from(entries: &[(&str, &str)]) -> crate::herdr::wire::Snapshot {
        use crate::herdr::wire::{AgentStatus, Pane, PaneLayout, Snapshot, Tab, Workspace};
        let mut snap = Snapshot::default();
        for (id, path) in entries {
            snap.workspaces.push(Workspace {
                workspace_id: (*id).into(),
                label: (*id).into(),
                agent_status: AgentStatus::Idle,
                active_tab_id: Some(format!("{id}:t")),
            });
            snap.tabs.push(Tab {
                tab_id: format!("{id}:t"),
                label: "main".into(),
            });
            snap.layouts.push(PaneLayout {
                workspace_id: (*id).into(),
                tab_id: format!("{id}:t"),
                focused_pane_id: Some(format!("{id}:p")),
            });
            snap.panes.push(Pane {
                pane_id: format!("{id}:p"),
                workspace_id: (*id).into(),
                tab_id: format!("{id}:t"),
                cwd: Some((*path).to_string()),
                foreground_cwd: Some((*path).to_string()),
                label: None,
            });
        }
        snap
    }

    #[tokio::test]
    async fn createoptions_anchor_under_a_git_repo_shows_the_repo_root() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir(repo.path().join(".git")).unwrap();
        let nested = repo.path().join("src/inner");
        std::fs::create_dir_all(&nested).unwrap();

        let snap = snapshot_from(&[("wgit", nested.to_str().unwrap())]);
        let herdr = std::sync::Arc::new(FixedSnapshotHerdr { snap });
        let state = crate::web::AppState::new(herdr, Some("s3cret-token".into()), 16);
        let (status, body) = get_create_options(state).await;
        assert_eq!(status, StatusCode::OK);
        let destinations = body["destinations"].as_array().unwrap();
        assert_eq!(destinations.len(), 1);
        assert_eq!(destinations[0]["path"], repo.path().to_str().unwrap());
    }

    #[tokio::test]
    async fn createoptions_sorts_git_root_destinations_first() {
        // "wplain" (no .git anywhere above it) comes first in the snapshot's
        // own workspace order, "wgit" second -- the sort must still put the
        // git-root-resolved one ("wgit") first in the response.
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir(repo.path().join(".git")).unwrap();
        let nested = repo.path().join("nested");
        std::fs::create_dir_all(&nested).unwrap();

        let plain = tempfile::tempdir().unwrap();

        let snap = snapshot_from(&[
            ("wplain", plain.path().to_str().unwrap()),
            ("wgit", nested.to_str().unwrap()),
        ]);
        let herdr = std::sync::Arc::new(FixedSnapshotHerdr { snap });
        let state = crate::web::AppState::new(herdr, Some("s3cret-token".into()), 16);
        let (status, body) = get_create_options(state).await;
        assert_eq!(status, StatusCode::OK);
        let destinations = body["destinations"].as_array().unwrap();
        assert_eq!(destinations.len(), 2);
        assert_eq!(destinations[0]["workspace_id"], "wgit");
        assert_eq!(destinations[0]["path"], repo.path().to_str().unwrap());
        assert_eq!(destinations[1]["workspace_id"], "wplain");
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

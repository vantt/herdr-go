//! Web layer — axum. The gateway's face on the tailnet: token+cookie auth, the
//! agent switcher, and the **observe + reply** surface (poll a pane's screen,
//! post a reply). No live WebSocket terminal — herdr's request API has no PTY
//! sizing, so the phone observes a zoom/pan screen and replies via a textarea
//! (decision 675fc93a).

pub mod api;
pub mod auth;
pub mod cf_access;
pub mod create;
pub mod screen;

use std::collections::HashSet;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tokio::sync::Mutex;
use tower_http::services::{ServeDir, ServeFile};

use crate::herdr::Herdr;
use cf_access::CfAccessVerifier;

/// The web UI, embedded into the binary at compile time (D b300856d). The
/// `debug-embed` feature forces embedding in every profile, so `static/` is
/// baked in under `cargo test`/`clippy` too, not just `--release`. `build.rs`
/// guarantees `static/` exists at compile time even before `npm run bundle`.
#[derive(rust_embed::RustEmbed, Clone)]
#[folder = "static/"]
struct Assets;

/// Shared application state (cheap to clone — all `Arc`).
#[derive(Clone)]
pub struct AppState {
    pub herdr: Arc<dyn Herdr>,
    pub sessions: Arc<Mutex<HashSet<String>>>,
    pub web_secret: Arc<Option<String>>,
    pub version: &'static str,
    pub protocol: u32,
    /// The operator's configured agent-create presets. Only the label ever
    /// reaches the client (`api::create_options`) — `argv` stays here.
    pub agent_presets: Arc<Vec<crate::config::AgentPreset>>,
    /// Optional Cloudflare Access JWT verifier. `None` unless the operator set
    /// both `cf_access_team_domain` and `cf_access_aud`; when present,
    /// `AuthSession` also accepts a verified `Cf-Access-Jwt-Assertion` header as
    /// an alternate credential alongside the session cookie.
    pub cf_access: Arc<Option<CfAccessVerifier>>,
}

impl AppState {
    pub fn new(herdr: Arc<dyn Herdr>, web_secret: Option<String>, protocol: u32) -> Self {
        AppState {
            herdr,
            sessions: Arc::new(Mutex::new(HashSet::new())),
            web_secret: Arc::new(web_secret),
            version: crate::VERSION,
            protocol,
            agent_presets: Arc::new(Vec::new()),
            cf_access: Arc::new(None),
        }
    }

    /// Attach the operator's configured agent presets. Builder-style rather
    /// than an `AppState::new` parameter, so `AppState::new`'s existing call
    /// sites — including `tests/observe_reply_e2e.rs`, out of this cell's
    /// file scope — keep compiling unchanged.
    pub fn with_agent_presets(mut self, presets: Vec<crate::config::AgentPreset>) -> Self {
        self.agent_presets = Arc::new(presets);
        self
    }

    /// Attach an optional Cloudflare Access verifier. Builder-style, mirroring
    /// `with_agent_presets`, so existing `AppState::new` call sites keep
    /// compiling with CF Access off (`None`).
    pub fn with_cf_access(mut self, verifier: Option<CfAccessVerifier>) -> Self {
        self.cf_access = Arc::new(verifier);
        self
    }
}

fn api_routes(state: AppState) -> Router {
    Router::new()
        .route("/api/login", post(auth::login))
        .route("/api/logout", post(auth::logout))
        .route("/api/health", get(api::health))
        .route("/api/agents", get(api::agents).post(create::create_agent))
        .route("/api/create-options", get(api::create_options))
        .route("/api/panes", post(create::create_pane))
        .route("/api/panes/:pane/screen", get(screen::read_screen))
        .route("/api/panes/:pane/input", post(screen::send_reply))
        .route("/api/panes/:pane/keys", post(screen::send_keys))
        .with_state(state)
}

/// Full router: API + the SPA. An on-disk build under `static_dir` overrides
/// the embedded UI when `<static_dir>/index.html` is present (config override /
/// local dev after `npm run bundle`); otherwise the UI embedded in the binary
/// is served (a standalone binary with no `static/` beside it). Both paths do
/// SPA fallback: an unmatched client route returns `index.html` with 200.
pub fn router(state: AppState, static_dir: &std::path::Path) -> Router {
    let index = static_dir.join("index.html");
    if index.exists() {
        let spa = ServeDir::new(static_dir).fallback(ServeFile::new(index));
        api_routes(state).fallback_service(spa)
    } else {
        let embedded = axum_embed::ServeEmbed::<Assets>::with_parameters(
            Some("index.html".to_string()),
            axum_embed::FallbackBehavior::Ok,
            Some("index.html".to_string()),
        );
        api_routes(state).fallback_service(embedded)
    }
}

/// API-only router (handler tests).
pub fn api_router(state: AppState) -> Router {
    api_routes(state)
}

#[cfg(test)]
pub(crate) fn test_state() -> AppState {
    let fake = Arc::new(crate::herdr::fake::FakeHerdr::new());
    AppState::new(
        fake,
        Some("s3cret-token".into()),
        crate::herdr::HERDR_PROTOCOL,
    )
}

/// Log in against a router built from `state` and return the session cookie
/// pair (`hg_session=...`) for reuse in handler tests.
#[cfg(test)]
pub(crate) async fn test_login_cookie(state: &AppState) -> String {
    use axum::body::Body;
    use axum::http::{header, Request};
    use tower::ServiceExt;
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    async fn get(app: Router, uri: &str) -> (StatusCode, String) {
        let res = app
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    #[tokio::test]
    async fn disk_build_is_served_from_disk_when_present() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "DISK-OVERRIDE-MARKER").unwrap();

        let app = router(test_state(), dir.path());
        let (status, body) = get(app, "/").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "DISK-OVERRIDE-MARKER");
    }

    #[tokio::test]
    async fn embedded_ui_serves_index_when_no_disk_build() {
        let missing = std::path::Path::new("/nonexistent-herdr-go-static-dir");
        assert!(!missing.join("index.html").exists());

        let app = router(test_state(), missing);
        let (status, body) = get(app, "/").await;
        assert_eq!(status, StatusCode::OK);
        assert!(!body.is_empty());
    }

    #[tokio::test]
    async fn embedded_ui_spa_fallback_returns_index_for_unmatched_route() {
        let missing = std::path::Path::new("/nonexistent-herdr-go-static-dir");
        let app = router(test_state(), missing);

        let (root_status, root_body) = get(router(test_state(), missing), "/").await;
        assert_eq!(root_status, StatusCode::OK);

        let (status, body) = get(app, "/agents/some/client/route").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, root_body);
    }
}

//! Web layer — axum. The gateway's face on the tailnet: token+cookie auth, the
//! agent switcher, and the **observe + reply** surface (poll a pane's screen,
//! post a reply). No live WebSocket terminal — herdr's request API has no PTY
//! sizing, so the phone observes a zoom/pan screen and replies via a textarea
//! (decision 675fc93a).

pub mod api;
pub mod auth;
pub mod screen;

use std::collections::HashSet;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tokio::sync::Mutex;
use tower_http::services::{ServeDir, ServeFile};

use crate::herdr::Herdr;

/// Shared application state (cheap to clone — all `Arc`).
#[derive(Clone)]
pub struct AppState {
    pub herdr: Arc<dyn Herdr>,
    pub sessions: Arc<Mutex<HashSet<String>>>,
    pub web_secret: Arc<Option<String>>,
    pub version: &'static str,
    pub protocol: u32,
}

impl AppState {
    pub fn new(herdr: Arc<dyn Herdr>, web_secret: Option<String>, protocol: u32) -> Self {
        AppState {
            herdr,
            sessions: Arc::new(Mutex::new(HashSet::new())),
            web_secret: Arc::new(web_secret),
            version: crate::VERSION,
            protocol,
        }
    }
}

fn api_routes(state: AppState) -> Router {
    Router::new()
        .route("/api/login", post(auth::login))
        .route("/api/logout", post(auth::logout))
        .route("/api/health", get(api::health))
        .route("/api/agents", get(api::agents))
        .route("/api/panes/:pane/screen", get(screen::read_screen))
        .route("/api/panes/:pane/input", post(screen::send_reply))
        .route("/api/panes/:pane/keys", post(screen::send_keys))
        .with_state(state)
}

/// Full router: API + the static SPA (vite build).
pub fn router(state: AppState, static_dir: &std::path::Path) -> Router {
    let index = static_dir.join("index.html");
    let spa = ServeDir::new(static_dir).fallback(ServeFile::new(index));
    api_routes(state).fallback_service(spa)
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

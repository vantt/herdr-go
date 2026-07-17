//! Web layer — axum. The gateway's public face on the tailnet: static-token
//! auth, the agent switcher API, and the Tier 2 terminal WebSocket relay.
//!
//! The relay (`relay.rs`) is a **transparent pipe** between xterm.js and
//! [`HerdrStream`] — it deliberately never touches `core` and never sees
//! [`HerdrControl`] (decision da82b90f): a raw terminal bridge must not be able
//! to fire a control-plane verb.

pub mod api;
pub mod auth;
pub mod relay;

use std::collections::HashSet;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tokio::sync::Mutex;
use tower_http::services::{ServeDir, ServeFile};

use crate::herdr::{HerdrControl, HerdrStream};

/// Shared application state. Cheap to clone (everything behind `Arc`).
#[derive(Clone)]
pub struct AppState {
    pub control: Arc<dyn HerdrControl>,
    pub stream: Arc<dyn HerdrStream>,
    /// Valid session ids (single-operator, in-memory — durable secret is env).
    pub sessions: Arc<Mutex<HashSet<String>>>,
    /// The configured web secret; `None` means auth is misconfigured and every
    /// login fails closed.
    pub web_secret: Arc<Option<String>>,
    pub version: &'static str,
    pub protocol: u32,
}

impl AppState {
    pub fn new(
        control: Arc<dyn HerdrControl>,
        stream: Arc<dyn HerdrStream>,
        web_secret: Option<String>,
        protocol: u32,
    ) -> Self {
        AppState {
            control,
            stream,
            sessions: Arc::new(Mutex::new(HashSet::new())),
            web_secret: Arc::new(web_secret),
            version: crate::VERSION,
            protocol,
        }
    }
}

/// Build the full router. `static_dir` is the vite build output served as the
/// SPA; a missing directory still yields a working API (useful in tests).
pub fn router(state: AppState, static_dir: &std::path::Path) -> Router {
    let index = static_dir.join("index.html");
    let spa = ServeDir::new(static_dir).fallback(ServeFile::new(index));

    Router::new()
        .route("/api/login", post(auth::login))
        .route("/api/logout", post(auth::logout))
        .route("/api/health", get(api::health))
        .route("/api/agents", get(api::agents))
        .route("/ws/terminal", get(relay::ws_terminal))
        .fallback_service(spa)
        .with_state(state)
}

/// Build a router with no static SPA (API only) — for handler tests.
pub fn api_router(state: AppState) -> Router {
    Router::new()
        .route("/api/login", post(auth::login))
        .route("/api/logout", post(auth::logout))
        .route("/api/health", get(api::health))
        .route("/api/agents", get(api::agents))
        .route("/ws/terminal", get(relay::ws_terminal))
        .with_state(state)
}

#[cfg(test)]
pub(crate) fn test_state() -> AppState {
    let fake = Arc::new(crate::herdr::fake::FakeHerdr::new());
    AppState::new(
        fake.clone(),
        fake,
        Some("s3cret-token".into()),
        crate::herdr::HERDR_PROTOCOL,
    )
}

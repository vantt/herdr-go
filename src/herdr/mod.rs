//! herdr port — the gateway is a **client of the herdr server** (peer to the
//! TUI), talking `herdr.sock`'s JSON request/response API (DISCOVERY 2026-07-18).
//!
//! The surface is request/response only (no live stream): snapshot the runtime,
//! read a pane's screen (polled), and send input as a reply. One trait, two
//! implementations — [`socket::SocketHerdr`] over the real socket and
//! [`fake::FakeHerdr`] for tests/`--demo`.

pub mod fake;
pub mod socket;
pub mod wire;

use async_trait::async_trait;

pub use wire::{Agent, AgentStatus, ProtocolInfo, ScreenRead, Snapshot, HERDR_PROTOCOL};

#[derive(Debug, thiserror::Error)]
pub enum HerdrError {
    #[error("herdr runtime is unavailable: {0}")]
    Unavailable(String),
    #[error("protocol mismatch: gateway pins {expected}, server reports {actual}")]
    ProtocolMismatch { expected: u32, actual: u32 },
    #[error("herdr request failed: {0}")]
    Request(String),
    #[error("malformed herdr response: {0}")]
    Malformed(String),
    #[error("no such pane: {0}")]
    NoSuchPane(String),
    #[error("agent name already in use: {name} ({message})")]
    AgentNameTaken { name: String, message: String },
    #[error("workspace not found: {workspace_id} ({message})")]
    WorkspaceNotFound {
        workspace_id: String,
        message: String,
    },
    #[error("invalid agent argv: {0}")]
    InvalidAgentArgv(String),
    #[error("herdr refused the request ({code}): {message}")]
    Remote { code: String, message: String },
}

pub type Result<T> = std::result::Result<T, HerdrError>;

/// Everything the gateway needs from herdr — all request/response.
#[async_trait]
pub trait Herdr: Send + Sync {
    /// Snapshot the server's runtime (the flat agent list).
    async fn snapshot(&self) -> Result<Snapshot>;

    /// Health + protocol handshake; a mismatch is a typed error.
    async fn ping(&self) -> Result<ProtocolInfo>;

    /// Read one pane's current rendered screen (polled for observation).
    async fn read_pane(&self, pane_id: &str) -> Result<ScreenRead>;

    /// Send a reply into a pane. `text` is typed in; `submit` then sends Enter
    /// (handles herdr's send≠submit: text alone does not submit).
    async fn send_input(&self, pane_id: &str, text: &str, submit: bool) -> Result<()>;

    /// Send raw key presses to a pane — e.g. arrow keys to drive a TUI option
    /// menu, or Enter/Escape/Tab. Key names are herdr's (`up`, `down`, `enter`,
    /// `escape`, `tab`, …).
    async fn send_keys(&self, pane_id: &str, keys: &[String]) -> Result<()>;
}

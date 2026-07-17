//! Durable store — the gateway's only session-durable state (PRD §9): the
//! Telegram poll offset and at-least-once notification delivery markers. It
//! **never** stores terminal output or credentials (airemote never-store rule).
//!
//! A hexagonal port (decision 4e3ef1a1): [`Store`] with a sqlite implementation
//! for production and an in-memory one for tests.

pub mod memory;
pub mod sqlite;

use async_trait::async_trait;

pub use memory::MemoryStore;
pub use sqlite::SqliteStore;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("store backend error: {0}")]
    Backend(String),
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// A pending notification obligation (e.g. "agent X is blocked"). Delivery is
/// at-least-once: recorded first, marked delivered only after a successful send,
/// so a crash between the two resends rather than loses it (airemote D47/D57).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub id: i64,
    pub pane_id: String,
    pub kind: String,
    pub body: String,
}

#[async_trait]
pub trait Store: Send + Sync {
    /// Load the durable Telegram poll offset (act → persist → fetch ordering).
    async fn poll_offset(&self) -> Result<i64>;
    /// Persist the poll offset after a batch has been acted on.
    async fn set_poll_offset(&self, offset: i64) -> Result<()>;

    /// Queue a notification obligation; returns its id.
    async fn enqueue_notification(&self, pane_id: &str, kind: &str, body: &str) -> Result<i64>;
    /// List notifications not yet marked delivered.
    async fn undelivered(&self) -> Result<Vec<Notification>>;
    /// Mark one notification delivered (only after a successful send).
    async fn mark_delivered(&self, id: i64) -> Result<()>;
}

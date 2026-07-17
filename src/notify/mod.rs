//! Notify — outbound alerts when an agent needs a human (`blocked`) or finishes
//! (`done`). The M1 channel is Telegram (decision 302c0544); the design is a
//! hexagonal port so another channel (web push, later) drops in unchanged.
//!
//! Delivery is **at-least-once** (airemote D47/D57): the obligation is enqueued
//! in the [`Store`](crate::store::Store) first and marked delivered only after a
//! successful send, so a crash between the two resends rather than loses it.
//! Outbound text is passed through the single [`redact`](crate::security::redact)
//! before leaving the process.

pub mod telegram;

use std::sync::Arc;

use async_trait::async_trait;

use crate::store::Store;
use crate::watcher::StatusChange;
use crate::herdr::AgentStatus;

pub use telegram::TelegramNotifier;

#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    #[error("notify send failed: {0}")]
    Send(String),
}

pub type Result<T> = std::result::Result<T, NotifyError>;

/// A channel that can deliver one alert. Implementations must not log secrets.
#[async_trait]
pub trait Notifier: Send + Sync {
    async fn send(&self, kind: &str, body: &str) -> Result<()>;
}

/// A notifier that only logs — used in `--demo` and when no channel is set.
pub struct NullNotifier;

#[async_trait]
impl Notifier for NullNotifier {
    async fn send(&self, kind: &str, body: &str) -> Result<()> {
        tracing::info!(kind, body, "notify (null channel)");
        Ok(())
    }
}

/// Which status transitions are worth a human's attention.
pub fn is_notifiable(status: AgentStatus) -> bool {
    matches!(status, AgentStatus::Blocked | AgentStatus::Done)
}

/// Bridges the watcher to a channel with durable, at-least-once delivery.
pub struct NotifyService {
    store: Arc<dyn Store>,
    notifier: Arc<dyn Notifier>,
}

impl NotifyService {
    pub fn new(store: Arc<dyn Store>, notifier: Arc<dyn Notifier>) -> Self {
        NotifyService { store, notifier }
    }

    /// Record a status change as a pending obligation *if* it is notifiable.
    /// Returns true if it was enqueued.
    pub async fn record(&self, change: &StatusChange) -> bool {
        if !is_notifiable(change.status) {
            return false;
        }
        let body = crate::security::redact(&format!(
            "{} agent {} is {}",
            change.kind,
            change.pane_id,
            change.status.as_str()
        ));
        // Enqueue first (act → persist), before any send attempt.
        self.store
            .enqueue_notification(&change.pane_id, change.status.as_str(), &body)
            .await
            .is_ok()
    }

    /// Drain the outbox: send each pending notification, marking it delivered
    /// only on success. A send failure leaves it pending for the next drain
    /// (at-least-once). Returns how many were delivered this pass.
    pub async fn drain(&self) -> usize {
        let pending = match self.store.undelivered().await {
            Ok(p) => p,
            Err(_) => return 0,
        };
        let mut delivered = 0;
        for n in pending {
            match self.notifier.send(&n.kind, &n.body).await {
                Ok(()) => {
                    // Send succeeded → now mark delivered. Order matters: a crash
                    // before this line resends; it never silently drops.
                    if self.store.mark_delivered(n.id).await.is_ok() {
                        delivered += 1;
                    }
                }
                Err(_) => { /* leave pending for the next drain */ }
            }
        }
        delivered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemoryStore;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingNotifier {
        sent: Mutex<Vec<String>>,
        fail_next: AtomicUsize,
    }
    #[async_trait]
    impl Notifier for RecordingNotifier {
        async fn send(&self, _kind: &str, body: &str) -> Result<()> {
            if self.fail_next.load(Ordering::SeqCst) > 0 {
                self.fail_next.fetch_sub(1, Ordering::SeqCst);
                return Err(NotifyError::Send("simulated".into()));
            }
            self.sent.lock().unwrap().push(body.to_string());
            Ok(())
        }
    }

    fn change(pane: &str, status: AgentStatus) -> StatusChange {
        StatusChange {
            pane_id: pane.into(),
            kind: "claude".into(),
            status,
        }
    }

    #[tokio::test]
    async fn only_blocked_and_done_are_recorded() {
        let store = Arc::new(MemoryStore::new());
        let svc = NotifyService::new(store.clone(), Arc::new(NullNotifier));
        assert!(svc.record(&change("p", AgentStatus::Blocked)).await);
        assert!(svc.record(&change("p", AgentStatus::Done)).await);
        assert!(!svc.record(&change("p", AgentStatus::Working)).await);
        assert!(!svc.record(&change("p", AgentStatus::Idle)).await);
        assert_eq!(store.undelivered().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn drain_delivers_and_marks() {
        let store = Arc::new(MemoryStore::new());
        let notifier = Arc::new(RecordingNotifier::default());
        let svc = NotifyService::new(store.clone(), notifier.clone());
        svc.record(&change("p1", AgentStatus::Blocked)).await;
        assert_eq!(svc.drain().await, 1);
        assert_eq!(notifier.sent.lock().unwrap().len(), 1);
        assert!(store.undelivered().await.unwrap().is_empty());
        // Draining again sends nothing (already delivered).
        assert_eq!(svc.drain().await, 0);
    }

    #[tokio::test]
    async fn failed_send_stays_pending_then_redelivers() {
        let store = Arc::new(MemoryStore::new());
        let notifier = Arc::new(RecordingNotifier::default());
        notifier.fail_next.store(1, Ordering::SeqCst);
        let svc = NotifyService::new(store.clone(), notifier.clone());
        svc.record(&change("p1", AgentStatus::Done)).await;
        // First drain: the send fails, so nothing is marked delivered.
        assert_eq!(svc.drain().await, 0);
        assert_eq!(store.undelivered().await.unwrap().len(), 1);
        // Second drain: send succeeds now — at-least-once holds.
        assert_eq!(svc.drain().await, 1);
        assert!(store.undelivered().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn outbound_body_is_redacted() {
        let store = Arc::new(MemoryStore::new());
        let svc = NotifyService::new(store.clone(), Arc::new(NullNotifier));
        // A pane id carrying a token-shaped string must be redacted in the body.
        let c = StatusChange {
            pane_id: "ghp_abcd1234EFGH".into(),
            kind: "claude".into(),
            status: AgentStatus::Blocked,
        };
        svc.record(&c).await;
        let body = &store.undelivered().await.unwrap()[0].body;
        assert!(body.contains("[REDACTED:GITHUB_TOKEN]"));
        assert!(!body.contains("ghp_abcd1234EFGH"));
    }
}

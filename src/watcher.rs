//! Event watcher — turns herdr status into gateway events. M1 uses **polling**
//! (500ms default, per Telegram-B / decision 302c0544); the verified
//! `events.subscribe` upgrade (PBI-001) is a later optimization, not a ship
//! blocker.
//!
//! **Cursor de-dup is mandatory** (DISCOVERY §events): herdr replays a recent
//! ring buffer on every subscribe connect AND `pane.agent_status_changed` has
//! fired twice in the same millisecond — so an emitted `(pane, status)`
//! transition is never delivered twice. The de-dup is intrinsic, not tied to a
//! transport, so polling gets it for free too.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::herdr::{AgentStatus, HerdrControl};

/// A de-duplicated status change worth acting on (e.g. notifying a human).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusChange {
    pub pane_id: String,
    pub kind: String,
    pub status: AgentStatus,
}

/// Tracks the last status seen per pane so only real transitions surface, and
/// the same transition never surfaces twice.
#[derive(Default)]
pub struct StatusCursor {
    last: HashMap<String, AgentStatus>,
}

impl StatusCursor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a fresh snapshot's agent statuses; return only the changes not
    /// already seen. A pane whose status is unchanged yields nothing; a repeated
    /// duplicate event yields nothing.
    pub fn diff(&mut self, statuses: &[(String, String, AgentStatus)]) -> Vec<StatusChange> {
        let mut out = Vec::new();
        for (pane_id, kind, status) in statuses {
            match self.last.get(pane_id) {
                Some(prev) if prev == status => {} // unchanged / duplicate
                _ => {
                    self.last.insert(pane_id.clone(), *status);
                    out.push(StatusChange {
                        pane_id: pane_id.clone(),
                        kind: kind.clone(),
                        status: *status,
                    });
                }
            }
        }
        out
    }
}

/// Flatten a snapshot into (pane_id, agent_kind, status) triples.
pub fn statuses_from(
    snap: &crate::herdr::Snapshot,
) -> Vec<(String, String, AgentStatus)> {
    let mut v = Vec::new();
    for ws in &snap.workspaces {
        for tab in &ws.tabs {
            for pane in &tab.panes {
                if let Some(agent) = &pane.agent {
                    v.push((pane.id.clone(), agent.kind.clone(), agent.status));
                }
            }
        }
    }
    v
}

/// Poll-based event source. Emits de-duplicated status changes on `sink`.
pub struct PollWatcher {
    control: Arc<dyn HerdrControl>,
    interval: Duration,
}

impl PollWatcher {
    pub fn new(control: Arc<dyn HerdrControl>, interval: Duration) -> Self {
        PollWatcher { control, interval }
    }

    /// Run one poll cycle against a cursor, returning fresh changes. Extracted so
    /// tests drive it deterministically without sleeping.
    pub async fn poll_once(&self, cursor: &mut StatusCursor) -> Vec<StatusChange> {
        match self.control.snapshot().await {
            Ok(snap) => cursor.diff(&statuses_from(&snap)),
            // A failed snapshot yields nothing — never a spurious change.
            Err(_) => Vec::new(),
        }
    }

    /// Run the poll loop forever, invoking `on_change` for each fresh change.
    pub async fn run<F>(self, mut on_change: F)
    where
        F: FnMut(StatusChange) + Send,
    {
        let mut cursor = StatusCursor::new();
        let mut ticker = tokio::time::interval(self.interval);
        loop {
            ticker.tick().await;
            for change in self.poll_once(&mut cursor).await {
                on_change(change);
            }
        }
    }

    /// Like [`run`](Self::run) but awaits an async handler for each change — used
    /// to feed the notify service (record → drain) per change.
    pub async fn run_async<F, Fut>(self, mut on_change: F)
    where
        F: FnMut(StatusChange) -> Fut + Send,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let mut cursor = StatusCursor::new();
        let mut ticker = tokio::time::interval(self.interval);
        loop {
            ticker.tick().await;
            for change in self.poll_once(&mut cursor).await {
                on_change(change).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::herdr::fake::FakeHerdr;

    #[test]
    fn cursor_emits_only_real_changes_and_dedups() {
        let mut cursor = StatusCursor::new();
        let batch = vec![
            ("p1".to_string(), "claude".to_string(), AgentStatus::Working),
            ("p2".to_string(), "codex".to_string(), AgentStatus::Idle),
        ];
        // First observation: both are new.
        assert_eq!(cursor.diff(&batch).len(), 2);
        // Same batch again (duplicate events / replay): nothing.
        assert_eq!(cursor.diff(&batch).len(), 0);
        // p1 transitions to blocked: exactly one change.
        let batch2 = vec![
            ("p1".to_string(), "claude".to_string(), AgentStatus::Blocked),
            ("p2".to_string(), "codex".to_string(), AgentStatus::Idle),
        ];
        let changes = cursor.diff(&batch2);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].status, AgentStatus::Blocked);
    }

    #[tokio::test]
    async fn poll_once_reports_driven_transition_once() {
        let fake = Arc::new(FakeHerdr::new());
        let watcher = PollWatcher::new(fake.clone(), Duration::from_millis(500));
        let mut cursor = StatusCursor::new();
        // First poll: seeds all four seeded panes as "new".
        let first = watcher.poll_once(&mut cursor).await;
        assert_eq!(first.len(), 4);
        // No change → no events.
        assert_eq!(watcher.poll_once(&mut cursor).await.len(), 0);
        // Drive idle → done.
        fake.set_status("pane-idle", AgentStatus::Done).await.unwrap();
        let changes = watcher.poll_once(&mut cursor).await;
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].pane_id, "pane-idle");
        assert_eq!(changes[0].status, AgentStatus::Done);
        // Polling again with no change → still nothing (dedup).
        assert_eq!(watcher.poll_once(&mut cursor).await.len(), 0);
    }
}

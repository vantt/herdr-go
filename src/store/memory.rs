//! In-memory store — for tests and for running the gateway ephemerally.

use std::sync::Mutex;

use async_trait::async_trait;

use super::{Notification, Result, Store};

#[derive(Default)]
pub struct MemoryStore {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    offset: i64,
    next_id: i64,
    notifications: Vec<(Notification, bool)>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Store for MemoryStore {
    async fn poll_offset(&self) -> Result<i64> {
        Ok(self.inner.lock().unwrap().offset)
    }

    async fn set_poll_offset(&self, offset: i64) -> Result<()> {
        self.inner.lock().unwrap().offset = offset;
        Ok(())
    }

    async fn enqueue_notification(&self, pane_id: &str, kind: &str, body: &str) -> Result<i64> {
        let mut inner = self.inner.lock().unwrap();
        inner.next_id += 1;
        let id = inner.next_id;
        inner.notifications.push((
            Notification {
                id,
                pane_id: pane_id.to_string(),
                kind: kind.to_string(),
                body: body.to_string(),
            },
            false,
        ));
        Ok(id)
    }

    async fn undelivered(&self) -> Result<Vec<Notification>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .notifications
            .iter()
            .filter(|(_, delivered)| !delivered)
            .map(|(n, _)| n.clone())
            .collect())
    }

    async fn mark_delivered(&self, id: i64) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        for (n, delivered) in inner.notifications.iter_mut() {
            if n.id == id {
                *delivered = true;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn memory_store_matches_contract() {
        let s = MemoryStore::new();
        s.set_poll_offset(7).await.unwrap();
        assert_eq!(s.poll_offset().await.unwrap(), 7);
        let id = s
            .enqueue_notification("p", "done", "finished")
            .await
            .unwrap();
        assert_eq!(s.undelivered().await.unwrap().len(), 1);
        s.mark_delivered(id).await.unwrap();
        assert!(s.undelivered().await.unwrap().is_empty());
    }
}

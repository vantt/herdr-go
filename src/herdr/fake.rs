//! In-memory herdr — mirrors the real socket shapes (flat snapshot, `pane.read`
//! screen buffer, `send_input` echo) so the whole app runs and is tested with no
//! live herdr (`--demo` and all consumer tests).

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use super::wire::*;
use super::{Herdr, HerdrError, Result};

#[derive(Clone)]
pub struct FakeHerdr {
    inner: Arc<Inner>,
}

struct Inner {
    snapshot: Mutex<Snapshot>,
    screens: Mutex<HashMap<String, (String, u64)>>, // pane_id -> (text, revision)
    available: std::sync::atomic::AtomicBool,
}

impl Default for FakeHerdr {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeHerdr {
    /// Seeded with four agents, one per status, each with a starter screen.
    pub fn new() -> Self {
        let agents = vec![
            agent(
                "w1:p1",
                "claude",
                AgentStatus::Working,
                "Building the parser",
            ),
            agent(
                "w1:p2",
                "codex",
                AgentStatus::Blocked,
                "Waiting for your answer",
            ),
            agent(
                "w2:p3",
                "claude",
                AgentStatus::Done,
                "Finished the refactor",
            ),
            agent("w2:p4", "codex", AgentStatus::Idle, "Idle"),
        ];
        let mut screens = HashMap::new();
        for a in &agents {
            screens.insert(
                a.pane_id.clone(),
                (format!("{} [{}]\n❯ ", a.title, a.status.as_str()), 1),
            );
        }
        FakeHerdr {
            inner: Arc::new(Inner {
                snapshot: Mutex::new(Snapshot {
                    agents,
                    ..Default::default()
                }),
                screens: Mutex::new(screens),
                available: std::sync::atomic::AtomicBool::new(true),
            }),
        }
    }

    /// Empty runtime (recovery tests).
    pub fn empty() -> Self {
        let f = Self::new();
        {
            let mut s = f.inner.snapshot.try_lock().expect("fresh, uncontended");
            *s = Snapshot::default();
        }
        f
    }

    pub fn set_available(&self, up: bool) {
        self.inner
            .available
            .store(up, std::sync::atomic::Ordering::SeqCst);
    }

    /// Drive an agent's status (as a live change would).
    pub async fn set_status(&self, pane_id: &str, status: AgentStatus) -> Result<()> {
        let mut snap = self.inner.snapshot.lock().await;
        for a in &mut snap.agents {
            if a.pane_id == pane_id {
                a.status = status;
                return Ok(());
            }
        }
        Err(HerdrError::NoSuchPane(pane_id.to_string()))
    }

    fn ensure_up(&self) -> Result<()> {
        if self
            .inner
            .available
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            Ok(())
        } else {
            Err(HerdrError::Unavailable("fake herdr is down".into()))
        }
    }
}

fn agent(pane_id: &str, kind: &str, status: AgentStatus, title: &str) -> Agent {
    Agent {
        pane_id: pane_id.into(),
        workspace_id: pane_id.split(':').next().unwrap_or("w").into(),
        tab_id: format!("{}:t", pane_id.split(':').next().unwrap_or("w")),
        kind: kind.into(),
        status,
        title: title.into(),
    }
}

#[async_trait]
impl Herdr for FakeHerdr {
    async fn snapshot(&self) -> Result<Snapshot> {
        self.ensure_up()?;
        Ok(self.inner.snapshot.lock().await.clone())
    }

    async fn ping(&self) -> Result<ProtocolInfo> {
        self.ensure_up()?;
        Ok(ProtocolInfo {
            protocol: HERDR_PROTOCOL,
            server_version: "fake-0.7.4".into(),
        })
    }

    async fn read_pane(&self, pane_id: &str) -> Result<ScreenRead> {
        self.ensure_up()?;
        let screens = self.inner.screens.lock().await;
        match screens.get(pane_id) {
            Some((text, rev)) => Ok(ScreenRead {
                text: text.clone(),
                revision: *rev,
            }),
            None => Err(HerdrError::NoSuchPane(pane_id.to_string())),
        }
    }

    async fn send_input(&self, pane_id: &str, text: &str, submit: bool) -> Result<()> {
        self.ensure_up()?;
        let mut screens = self.inner.screens.lock().await;
        let entry = screens
            .get_mut(pane_id)
            .ok_or_else(|| HerdrError::NoSuchPane(pane_id.to_string()))?;
        entry.0.push_str(text);
        if submit {
            entry.0.push('\n');
        }
        entry.1 += 1; // revision bumps so a poller re-renders
        Ok(())
    }

    async fn send_keys(&self, pane_id: &str, keys: &[String]) -> Result<()> {
        self.ensure_up()?;
        let mut screens = self.inner.screens.lock().await;
        let entry = screens
            .get_mut(pane_id)
            .ok_or_else(|| HerdrError::NoSuchPane(pane_id.to_string()))?;
        // Echo keys so --demo and tests can observe them: Enter as a newline,
        // everything else as a visible <key> token.
        for k in keys {
            if k == "enter" {
                entry.0.push('\n');
            } else {
                entry.0.push_str(&format!("<{k}>"));
            }
        }
        entry.1 += 1; // revision bumps so a poller re-renders
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn snapshot_has_all_statuses() {
        let f = FakeHerdr::new();
        let s = f.snapshot().await.unwrap();
        let st: Vec<_> = s.agents.iter().map(|a| a.status).collect();
        assert!(st.contains(&AgentStatus::Working));
        assert!(st.contains(&AgentStatus::Blocked));
        assert!(st.contains(&AgentStatus::Done));
        assert!(st.contains(&AgentStatus::Idle));
    }

    #[tokio::test]
    async fn ping_compatible() {
        assert!(FakeHerdr::new().ping().await.unwrap().is_compatible());
    }

    #[tokio::test]
    async fn read_then_reply_echoes_and_bumps_revision() {
        let f = FakeHerdr::new();
        let before = f.read_pane("w1:p1").await.unwrap();
        f.send_input("w1:p1", "yes please", true).await.unwrap();
        let after = f.read_pane("w1:p1").await.unwrap();
        assert!(after.text.contains("yes please"));
        assert!(after.revision > before.revision);
    }

    #[tokio::test]
    async fn send_keys_echoes_and_bumps_revision() {
        let f = FakeHerdr::new();
        let before = f.read_pane("w1:p1").await.unwrap();
        f.send_keys("w1:p1", &["down".into(), "enter".into()])
            .await
            .unwrap();
        let after = f.read_pane("w1:p1").await.unwrap();
        assert!(after.text.contains("<down>"));
        assert!(after.revision > before.revision);
    }

    #[tokio::test]
    async fn send_keys_unknown_pane_errors() {
        let f = FakeHerdr::new();
        assert!(matches!(
            f.send_keys("nope", &["up".into()]).await,
            Err(HerdrError::NoSuchPane(_))
        ));
    }

    #[tokio::test]
    async fn unknown_pane_errors() {
        let f = FakeHerdr::new();
        assert!(matches!(
            f.read_pane("nope").await,
            Err(HerdrError::NoSuchPane(_))
        ));
    }

    #[tokio::test]
    async fn down_when_set() {
        let f = FakeHerdr::new();
        f.set_available(false);
        assert!(matches!(
            f.snapshot().await,
            Err(HerdrError::Unavailable(_))
        ));
    }
}

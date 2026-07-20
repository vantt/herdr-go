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
        // panes[] is a superset of agents[]: w2:p5 is a plain shell with a
        // folder and no agent, and it is w2's anchor — the same shape as the
        // live capture, so --demo exercises the real join instead of the easy
        // case where every anchor happens to be an agent.
        let panes = vec![
            pane("w1:p1", "/home/dev/projects/frontend-app"),
            pane("w1:p2", "/home/dev/projects/frontend-app"),
            pane("w2:p3", "/home/dev/projects/docs-site"),
            pane("w2:p4", "/home/dev/projects/docs-site"),
            pane("w2:p5", "/home/dev/projects/docs-site/site"),
        ];
        let mut screens = HashMap::new();
        for a in &agents {
            screens.insert(
                a.pane_id.clone(),
                (format!("{} [{}]\n❯ ", a.title, a.status.as_str()), 1),
            );
        }
        // The plain shell has a screen too — it is a real pane in this fake.
        screens.insert("w2:p5".to_string(), ("❯ ".to_string(), 1));
        FakeHerdr {
            inner: Arc::new(Inner {
                snapshot: Mutex::new(Snapshot {
                    agents,
                    workspaces: vec![
                        Workspace {
                            workspace_id: "w1".into(),
                            label: "frontend-app".into(),
                            agent_status: AgentStatus::Working,
                            active_tab_id: Some("w1:t".into()),
                        },
                        Workspace {
                            workspace_id: "w2".into(),
                            label: "docs-site".into(),
                            agent_status: AgentStatus::Done,
                            active_tab_id: Some("w2:t".into()),
                        },
                    ],
                    tabs: vec![
                        Tab {
                            tab_id: "w1:t".into(),
                            label: "main".into(),
                        },
                        Tab {
                            tab_id: "w2:t".into(),
                            label: "main".into(),
                        },
                    ],
                    panes,
                    layouts: vec![
                        PaneLayout {
                            workspace_id: "w1".into(),
                            tab_id: "w1:t".into(),
                            focused_pane_id: Some("w1:p1".into()),
                        },
                        PaneLayout {
                            workspace_id: "w2".into(),
                            tab_id: "w2:t".into(),
                            focused_pane_id: Some("w2:p5".into()),
                        },
                    ],
                    // Only w1 is globally focused; w2 still has its own anchor.
                    focused_workspace_id: Some("w1".into()),
                    focused_tab_id: Some("w1:t".into()),
                    focused_pane_id: Some("w1:p1".into()),
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

fn pane(pane_id: &str, cwd: &str) -> Pane {
    let ws = pane_id.split(':').next().unwrap_or("w");
    Pane {
        pane_id: pane_id.into(),
        workspace_id: ws.into(),
        tab_id: format!("{ws}:t"),
        cwd: Some(cwd.into()),
        foreground_cwd: Some(cwd.into()),
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
    async fn envelope_fake_seed_joins() {
        // --demo runs on this seed, so the seed must satisfy the same anchor
        // shape a live snapshot does: every workspace's active tab has a layout
        // entry whose focused pane really exists in panes[].
        let s = FakeHerdr::new().snapshot().await.unwrap();
        assert!(!s.workspaces.is_empty());

        for w in &s.workspaces {
            let active_tab = w
                .active_tab_id
                .as_deref()
                .unwrap_or_else(|| panic!("{} has no active_tab_id", w.workspace_id));
            assert!(
                s.tabs.iter().any(|t| t.tab_id == active_tab),
                "{active_tab} is not in tabs[]"
            );
            let layout = s
                .layouts
                .iter()
                .find(|l| l.workspace_id == w.workspace_id && l.tab_id == active_tab)
                .unwrap_or_else(|| panic!("no layout for {}/{active_tab}", w.workspace_id));
            let focused = layout.focused_pane_id.as_deref().unwrap();
            let anchor = s
                .panes
                .iter()
                .find(|p| p.pane_id == focused)
                .unwrap_or_else(|| panic!("{focused} is not in panes[]"));
            assert_eq!(anchor.workspace_id, w.workspace_id);
            assert!(anchor
                .foreground_cwd
                .as_deref()
                .or(anchor.cwd.as_deref())
                .is_some());
        }

        // At least one seeded pane is a plain shell absent from agents[] — the
        // case that makes panes[] irreplaceable by agents[].
        assert!(s
            .panes
            .iter()
            .any(|p| !s.agents.iter().any(|a| a.pane_id == p.pane_id)));
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

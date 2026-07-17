//! In-memory herdr adapter — lets the whole gateway run and be end-to-end
//! tested with no live herdr. Emits real-shaped frames and a snapshot with
//! agents in each status, and is drivable (push frames, change a status) so
//! tests can walk an agent idle→working→blocked→done.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use base64::Engine;
use futures_util::stream::StreamExt;
use tokio::sync::{broadcast, Mutex};

use super::wire::*;
use super::{
    ControlSession, FrameStream, HerdrControl, HerdrError, HerdrStream, Result,
};

/// A drivable in-memory herdr. Clone-cheap (shared state behind `Arc`).
#[derive(Clone)]
pub struct FakeHerdr {
    inner: Arc<Inner>,
}

struct Inner {
    snapshot: Mutex<Snapshot>,
    protocol: u32,
    server_version: String,
    /// Per-pane broadcast of frames, so multiple observers see the same stream.
    channels: Mutex<HashMap<String, broadcast::Sender<TerminalFrame>>>,
    seq: AtomicU64,
    available: std::sync::atomic::AtomicBool,
}

impl Default for FakeHerdr {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeHerdr {
    /// A fake pre-seeded with one workspace holding four panes, one agent in
    /// each status — a realistic switcher view for the UI and e2e tests.
    pub fn new() -> Self {
        let snapshot = Snapshot {
            workspaces: vec![Workspace {
                id: "ws-1".into(),
                label: "demo".into(),
                tabs: vec![Tab {
                    id: "tab-1".into(),
                    label: "main".into(),
                    panes: vec![
                        pane("pane-working", "claude", AgentStatus::Working),
                        pane("pane-blocked", "codex", AgentStatus::Blocked),
                        pane("pane-done", "claude", AgentStatus::Done),
                        pane("pane-idle", "codex", AgentStatus::Idle),
                    ],
                }],
            }],
        };
        FakeHerdr {
            inner: Arc::new(Inner {
                snapshot: Mutex::new(snapshot),
                protocol: HERDR_PROTOCOL,
                server_version: "fake-0.7.4".into(),
                channels: Mutex::new(HashMap::new()),
                seq: AtomicU64::new(0),
                available: std::sync::atomic::AtomicBool::new(true),
            }),
        }
    }

    /// Build an empty fake (no workspaces) — for reconcile/recovery tests.
    pub fn empty() -> Self {
        let f = Self::new();
        // Replace the seeded snapshot with an empty one without blocking.
        {
            let mut guard = f.inner.snapshot.try_lock().expect("fresh fake, uncontended");
            *guard = Snapshot::default();
        }
        f
    }

    /// Toggle runtime availability — models herdr being down (supervisor tests).
    pub fn set_available(&self, up: bool) {
        self.inner.available.store(up, Ordering::SeqCst);
    }

    /// Drive an agent's status, as a live status change would. Returns error if
    /// the pane is unknown.
    pub async fn set_status(&self, pane_id: &str, status: AgentStatus) -> Result<()> {
        let mut snap = self.inner.snapshot.lock().await;
        for ws in &mut snap.workspaces {
            for tab in &mut ws.tabs {
                for p in &mut tab.panes {
                    if p.id == pane_id {
                        if let Some(a) = &mut p.agent {
                            a.status = status;
                        }
                        return Ok(());
                    }
                }
            }
        }
        Err(HerdrError::NoSuchTarget(pane_id.to_string()))
    }

    /// Push a chunk of ANSI output into a pane's stream (as a diff frame).
    pub async fn push_output(&self, pane_id: &str, ansi: &[u8]) {
        let frame = self.make_frame(ansi, false);
        let mut chans = self.inner.channels.lock().await;
        let tx = chans
            .entry(pane_id.to_string())
            .or_insert_with(|| broadcast::channel(256).0);
        let _ = tx.send(frame);
    }

    fn make_frame(&self, ansi: &[u8], full: bool) -> TerminalFrame {
        let seq = self.inner.seq.fetch_add(1, Ordering::SeqCst);
        TerminalFrame {
            kind: FrameKind::Frame,
            seq,
            encoding: "ansi".into(),
            width: 80,
            height: 24,
            full,
            bytes: base64::engine::general_purpose::STANDARD.encode(ansi),
        }
    }

    async fn channel_for(&self, pane_id: &str) -> broadcast::Sender<TerminalFrame> {
        let mut chans = self.inner.channels.lock().await;
        chans
            .entry(pane_id.to_string())
            .or_insert_with(|| broadcast::channel(256).0)
            .clone()
    }

    fn ensure_up(&self) -> Result<()> {
        if self.inner.available.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(HerdrError::Unavailable("fake herdr is set down".into()))
        }
    }
}

fn pane(id: &str, agent_kind: &str, status: AgentStatus) -> Pane {
    Pane {
        id: id.into(),
        agent: Some(Agent {
            kind: agent_kind.into(),
            status,
        }),
    }
}

#[async_trait]
impl HerdrControl for FakeHerdr {
    async fn snapshot(&self) -> Result<Snapshot> {
        self.ensure_up()?;
        Ok(self.inner.snapshot.lock().await.clone())
    }

    async fn ping(&self) -> Result<ProtocolInfo> {
        self.ensure_up()?;
        Ok(ProtocolInfo {
            protocol: self.inner.protocol,
            server_version: self.inner.server_version.clone(),
        })
    }
}

#[async_trait]
impl HerdrStream for FakeHerdr {
    async fn observe(&self, target: &PaneTarget, _cols: u16, _rows: u16) -> Result<FrameStream> {
        self.ensure_up()?;
        let tx = self.channel_for(&target.pane_id).await;
        let rx = tx.subscribe();
        // Server sends a full=true frame first (redraw), then live diffs.
        let first = self.make_frame(b"\x1b[2J\x1b[H", true);
        let live = tokio_stream::wrappers::BroadcastStream::new(rx)
            .filter_map(|r| async move { r.ok().map(Ok) });
        let stream = futures_util::stream::once(async move { Ok(first) }).chain(live);
        Ok(stream.boxed())
    }

    async fn control(
        &self,
        target: &PaneTarget,
        _takeover: bool,
        _cols: u16,
        _rows: u16,
    ) -> Result<ControlSession> {
        self.ensure_up()?;
        let frames = self.observe(target, _cols, _rows).await?;
        let (tx, mut rx) = tokio::sync::mpsc::channel::<ControlMessage>(64);
        let this = self.clone();
        let pane_id = target.pane_id.clone();
        // Echo typed input back into the pane's frame stream, as a real terminal
        // would render keystrokes.
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let ControlMessage::Input { text, bytes } = msg {
                    if let Some(t) = text {
                        this.push_output(&pane_id, t.as_bytes()).await;
                    } else if let Some(b) = bytes {
                        if let Ok(raw) = base64::engine::general_purpose::STANDARD.decode(b) {
                            this.push_output(&pane_id, &raw).await;
                        }
                    }
                }
            }
        });
        Ok(ControlSession { frames, input: tx })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn snapshot_has_all_four_statuses() {
        let f = FakeHerdr::new();
        let snap = f.snapshot().await.unwrap();
        let statuses: Vec<AgentStatus> = snap.workspaces[0].tabs[0]
            .panes
            .iter()
            .filter_map(|p| p.agent.as_ref().map(|a| a.status))
            .collect();
        assert!(statuses.contains(&AgentStatus::Working));
        assert!(statuses.contains(&AgentStatus::Blocked));
        assert!(statuses.contains(&AgentStatus::Done));
        assert!(statuses.contains(&AgentStatus::Idle));
    }

    #[tokio::test]
    async fn ping_is_protocol_compatible() {
        let f = FakeHerdr::new();
        assert!(f.ping().await.unwrap().is_compatible());
    }

    #[tokio::test]
    async fn status_can_be_driven() {
        let f = FakeHerdr::new();
        f.set_status("pane-idle", AgentStatus::Working).await.unwrap();
        let snap = f.snapshot().await.unwrap();
        let p = &snap.workspaces[0].tabs[0].panes[3];
        assert_eq!(p.agent.as_ref().unwrap().status, AgentStatus::Working);
    }

    #[tokio::test]
    async fn observe_emits_full_frame_first_then_diffs() {
        let f = FakeHerdr::new();
        let target = f.resolve_target("pane-working").await.unwrap();
        let mut stream = f.observe(&target, 80, 24).await.unwrap();
        let first = stream.next().await.unwrap().unwrap();
        assert!(first.full, "first frame is a full redraw");
        // push a diff and see it arrive
        f.push_output("pane-working", b"hello").await;
        let second = stream.next().await.unwrap().unwrap();
        assert!(!second.full);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(second.bytes)
            .unwrap();
        assert_eq!(decoded, b"hello");
    }

    #[tokio::test]
    async fn control_echoes_typed_input() {
        let f = FakeHerdr::new();
        let target = f.resolve_target("pane-working").await.unwrap();
        let session = f.control(&target, true, 80, 24).await.unwrap();
        let mut frames = session.frames;
        let _full = frames.next().await.unwrap().unwrap();
        session.input.send(ControlMessage::text("ls\n")).await.unwrap();
        let echoed = frames.next().await.unwrap().unwrap();
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(echoed.bytes)
            .unwrap();
        assert_eq!(decoded, b"ls\n");
    }

    #[tokio::test]
    async fn unavailable_when_set_down() {
        let f = FakeHerdr::new();
        f.set_available(false);
        assert!(matches!(
            f.snapshot().await,
            Err(HerdrError::Unavailable(_))
        ));
    }
}

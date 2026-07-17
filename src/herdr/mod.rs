//! herdr port — the two hexagonal traits every consumer sees instead of the
//! subprocess/socket underneath (decision da82b90f). Split so each consumer's
//! surface is as narrow as its need:
//!
//! - [`HerdrControl`] — snapshot, ping/version, provision (workspace/agent) and
//!   later Tier 1 verbs. Used by supervisor, watcher, core.
//! - [`HerdrStream`] — observe/control frame+input. Used ONLY by the Tier 2 web
//!   relay, which must not see control-plane verbs (a transparent pipe should
//!   not be able to fire a side-effecting call — decision da82b90f).
//!
//! Both are implemented today by [`cli::CliHerdr`] and, for tests/e2e without a
//! live herdr, by [`fake::FakeHerdr`]; a socket adapter can be added later
//! without any consumer changing.

pub mod cli;
pub mod fake;
pub mod wire;

use async_trait::async_trait;
use futures_util::stream::BoxStream;

pub use wire::{
    Agent, AgentStatus, ControlMessage, Pane, PaneTarget, ProtocolInfo, Snapshot, Tab,
    TerminalFrame, Workspace, HERDR_PROTOCOL,
};

/// Errors crossing the herdr boundary.
#[derive(Debug, thiserror::Error)]
pub enum HerdrError {
    #[error("herdr runtime is unavailable: {0}")]
    Unavailable(String),
    #[error("protocol mismatch: gateway pins {expected}, server reports {actual}")]
    ProtocolMismatch { expected: u32, actual: u32 },
    #[error("herdr invocation failed: {0}")]
    Invocation(String),
    #[error("malformed herdr response: {0}")]
    Malformed(String),
    #[error("no such target: {0}")]
    NoSuchTarget(String),
}

pub type Result<T> = std::result::Result<T, HerdrError>;

/// A stream of terminal frames from an observe/control session. Frames arrive in
/// `seq` order; there are no gaps to backfill (DISCOVERY §Tier2 — seq is
/// ordering-only). The stream ends on `terminal.closed` OR a raw EOF; the relay
/// treats both identically.
pub type FrameStream = BoxStream<'static, Result<TerminalFrame>>;

/// A writable control session: a frame stream plus a sink for control messages.
pub struct ControlSession {
    pub frames: FrameStream,
    pub input: tokio::sync::mpsc::Sender<ControlMessage>,
}

/// Control-plane operations: snapshot, health, provisioning. NOT the terminal
/// stream (that is [`HerdrStream`]).
#[async_trait]
pub trait HerdrControl: Send + Sync {
    /// Take one snapshot of everything herdr currently has alive. The single
    /// source of truth; resolve opaque ids fresh from here every time.
    async fn snapshot(&self) -> Result<Snapshot>;

    /// Health + protocol handshake. Used by the supervisor and at startup; a
    /// protocol mismatch is a typed error, never a silent "probably fine".
    async fn ping(&self) -> Result<ProtocolInfo>;

    /// Resolve a pane target (opaque id + display path) for one agent by pane id,
    /// reading the current snapshot. Fails if the pane is gone.
    async fn resolve_target(&self, pane_id: &str) -> Result<PaneTarget> {
        let snap = self.snapshot().await?;
        for ws in &snap.workspaces {
            for tab in &ws.tabs {
                for pane in &tab.panes {
                    if pane.id == pane_id {
                        let kind = pane
                            .agent
                            .as_ref()
                            .map(|a| a.kind.as_str())
                            .unwrap_or("pane");
                        return Ok(PaneTarget {
                            pane_id: pane.id.clone(),
                            display: format!("{} › {} › {}", ws.label, tab.label, kind),
                        });
                    }
                }
            }
        }
        Err(HerdrError::NoSuchTarget(pane_id.to_string()))
    }
}

/// Terminal streaming: observe (read-only) and control (writable, single-writer).
#[async_trait]
pub trait HerdrStream: Send + Sync {
    /// Read-only observe. Many observers per terminal are fine. Server sends a
    /// `full=true` frame first, then diffs.
    async fn observe(&self, target: &PaneTarget, cols: u16, rows: u16) -> Result<FrameStream>;

    /// Writable control. herdr enforces one writer/terminal; `takeover` evicts
    /// the current writer. Resize semantics are the caller's (hybrid rotate,
    /// decision 82eff9f7): observer resize reshapes only this viewport, a
    /// controller resize reflows the real PTY.
    async fn control(
        &self,
        target: &PaneTarget,
        takeover: bool,
        cols: u16,
        rows: u16,
    ) -> Result<ControlSession>;
}

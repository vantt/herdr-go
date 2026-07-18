//! herdr wire types — the real herdr 0.7.4 socket shapes, verified live
//! (`.bee/spikes/herdr-socket-observe/`, DISCOVERY 2026-07-18). The socket speaks
//! newline-delimited JSON, one request→response per connection.

use serde::{Deserialize, Serialize};

/// Pinned wire protocol number — exact-match (bumps per herdr release).
pub const HERDR_PROTOCOL: u32 = 16;

/// A socket request: `{ "id", "method", "params" }`, newline-terminated.
#[derive(Debug, Serialize)]
pub struct Request<'a> {
    pub id: String,
    pub method: &'a str,
    pub params: serde_json::Value,
}

/// Agent readiness as herdr reports it. `Unknown` catches any value herdr adds
/// later (`#[serde(other)]`), so a new status never breaks the switcher.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Working,
    Blocked,
    Done,
    Idle,
    #[serde(other)]
    Unknown,
}

impl AgentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentStatus::Working => "working",
            AgentStatus::Blocked => "blocked",
            AgentStatus::Done => "done",
            AgentStatus::Idle => "idle",
            AgentStatus::Unknown => "unknown",
        }
    }
}

/// One agent in the flat snapshot (herdr's `session.snapshot` returns a flat
/// `agents[]`, not a nested tree). `pane_id` is opaque (`w3:p6`), read fresh.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Agent {
    pub pane_id: String,
    pub workspace_id: String,
    pub tab_id: String,
    /// Agent kind (herdr field `agent`, e.g. "claude", "codex").
    #[serde(rename = "agent")]
    pub kind: String,
    #[serde(rename = "agent_status")]
    pub status: AgentStatus,
    /// Human-readable terminal title (herdr `terminal_title_stripped`).
    #[serde(rename = "terminal_title_stripped", default)]
    pub title: String,
}

/// The runtime snapshot: everything herdr currently has alive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Snapshot {
    pub agents: Vec<Agent>,
}

impl Snapshot {
    /// Human display path for one agent's pane.
    pub fn display_for(agent: &Agent) -> String {
        let t = if agent.title.is_empty() {
            agent.kind.clone()
        } else {
            agent.title.clone()
        };
        format!("{} · {}", agent.kind, t)
    }
}

/// A polled screen read of one pane (`pane.read`). `text` is the rendered
/// screen (ANSI or plain); `revision` bumps when the pane content changes, so a
/// poller can skip re-rendering an unchanged screen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ScreenRead {
    pub text: String,
    pub revision: u64,
}

/// Health/handshake info (`ping` → pong). Pinned exact-match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolInfo {
    pub protocol: u32,
    pub server_version: String,
}

impl ProtocolInfo {
    pub fn is_compatible(&self) -> bool {
        self.protocol == HERDR_PROTOCOL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_status_parses_all_including_unknown() {
        let p = |s: &str| serde_json::from_str::<AgentStatus>(&format!("\"{s}\"")).unwrap();
        assert_eq!(p("working"), AgentStatus::Working);
        assert_eq!(p("done"), AgentStatus::Done);
        assert_eq!(p("idle"), AgentStatus::Idle);
        // an unexpected value falls back to Unknown, not an error
        assert_eq!(p("something_new"), AgentStatus::Unknown);
    }

    #[test]
    fn flat_snapshot_parses_real_shape() {
        // Shape captured live from `session.snapshot`.
        let json = r#"{
          "agents": [
            {"agent":"claude","agent_status":"idle","pane_id":"w3:p6","workspace_id":"w3","tab_id":"w3:t6","terminal_title_stripped":"Kiểm tra plan"}
          ]
        }"#;
        let snap: Snapshot = serde_json::from_str(json).unwrap();
        assert_eq!(snap.agents.len(), 1);
        let a = &snap.agents[0];
        assert_eq!(a.pane_id, "w3:p6");
        assert_eq!(a.kind, "claude");
        assert_eq!(a.status, AgentStatus::Idle);
        assert_eq!(a.title, "Kiểm tra plan");
    }

    #[test]
    fn protocol_pin_exact() {
        assert!(ProtocolInfo {
            protocol: 16,
            server_version: "0.7.4".into()
        }
        .is_compatible());
        assert!(!ProtocolInfo {
            protocol: 17,
            server_version: "x".into()
        }
        .is_compatible());
    }
}

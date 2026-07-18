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

/// One workspace entry from `session.snapshot.workspaces[]`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Workspace {
    pub workspace_id: String,
    pub label: String,
    pub agent_status: AgentStatus,
}

/// One tab entry from `session.snapshot.tabs[]`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tab {
    pub tab_id: String,
    pub label: String,
}

/// The runtime snapshot: everything herdr currently has alive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Snapshot {
    pub agents: Vec<Agent>,
    #[serde(default)]
    pub workspaces: Vec<Workspace>,
    #[serde(default)]
    pub tabs: Vec<Tab>,
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

    /// Resolve an agent's workspace label by joining on `workspace_id`.
    /// Empty string on a join miss — never panics.
    pub fn workspace_label_for(&self, agent: &Agent) -> String {
        self.workspaces
            .iter()
            .find(|w| w.workspace_id == agent.workspace_id)
            .map(|w| w.label.clone())
            .unwrap_or_default()
    }

    /// Resolve an agent's tab label by joining on `tab_id`. Empty string on a
    /// join miss — never panics.
    pub fn tab_label_for(&self, agent: &Agent) -> String {
        self.tabs
            .iter()
            .find(|t| t.tab_id == agent.tab_id)
            .map(|t| t.label.clone())
            .unwrap_or_default()
    }

    /// Resolve an agent's workspace status rollup by joining on `workspace_id`.
    /// Falls back to `AgentStatus::Unknown` on a join miss — never panics.
    pub fn workspace_status_for(&self, agent: &Agent) -> AgentStatus {
        self.workspaces
            .iter()
            .find(|w| w.workspace_id == agent.workspace_id)
            .map(|w| w.agent_status)
            .unwrap_or(AgentStatus::Unknown)
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
    fn snapshot_parses_workspaces_and_tabs_and_resolves_labels() {
        // Shape captured live from `session.snapshot`, including workspaces[]/tabs[].
        let json = r#"{
          "agents": [
            {"agent":"claude","agent_status":"idle","pane_id":"w3:p6","workspace_id":"w3","tab_id":"w3:t6","terminal_title_stripped":"Kiểm tra plan"}
          ],
          "workspaces": [
            {"workspace_id":"w3","label":"herdr-gateway","agent_status":"working"}
          ],
          "tabs": [
            {"tab_id":"w3:t6","label":"chat"}
          ]
        }"#;
        let snap: Snapshot = serde_json::from_str(json).unwrap();
        let a = &snap.agents[0];
        assert_eq!(snap.workspace_label_for(a), "herdr-gateway");
        assert_eq!(snap.tab_label_for(a), "chat");
        assert_eq!(snap.workspace_status_for(a), AgentStatus::Working);
    }

    #[test]
    fn workspace_status_falls_back_to_unknown_on_join_miss() {
        let snap = Snapshot {
            agents: vec![],
            workspaces: vec![],
            tabs: vec![],
        };
        let a = Agent {
            pane_id: "w9:p1".into(),
            workspace_id: "w9".into(),
            tab_id: "w9:t1".into(),
            kind: "claude".into(),
            status: AgentStatus::Idle,
            title: String::new(),
        };
        assert_eq!(snap.workspace_status_for(&a), AgentStatus::Unknown);
    }

    #[test]
    fn label_resolution_falls_back_to_empty_string_on_join_miss() {
        let snap = Snapshot {
            agents: vec![],
            workspaces: vec![],
            tabs: vec![],
        };
        let a = Agent {
            pane_id: "w9:p1".into(),
            workspace_id: "w9".into(),
            tab_id: "w9:t1".into(),
            kind: "claude".into(),
            status: AgentStatus::Idle,
            title: String::new(),
        };
        assert_eq!(snap.workspace_label_for(&a), "");
        assert_eq!(snap.tab_label_for(&a), "");
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

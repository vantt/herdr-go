//! herdr wire types — the on-the-socket shapes, verified live in the M0 spikes
//! (see docs/DISCOVERY.md). These are the contract; do not loosen them.

use serde::{Deserialize, Serialize};

/// The pinned wire protocol number. Compatibility is **exact-match**, not `>=`
/// (PBI-002): the number bumps per herdr release. A mismatch is a typed error.
pub const HERDR_PROTOCOL: u32 = 16;

/// One terminal output frame from `terminal session observe`. Exactly the 7
/// fields verified in DISCOVERY §Tier 2 — `full=true` is a whole-screen redraw
/// (first frame and after resize), `false` is a diff. `bytes` is base64 ANSI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalFrame {
    #[serde(rename = "type")]
    pub kind: FrameKind,
    pub seq: u64,
    pub encoding: String,
    pub width: u16,
    pub height: u16,
    pub full: bool,
    /// base64-encoded ANSI bytes.
    pub bytes: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FrameKind {
    #[serde(rename = "terminal.frame")]
    Frame,
}

/// The stream terminator: `terminal.closed` with a reason. NOTE (DISCOVERY):
/// an abrupt IPC EOF emits NO such marker — the relay treats a raw EOF
/// identically. Do not assume every stream end carries this.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalClosed {
    #[serde(rename = "type")]
    pub kind: ClosedKind,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClosedKind {
    #[serde(rename = "terminal.closed")]
    Closed,
}

/// Anything a controller sends back into a writable terminal
/// (`terminal session control`). `Input` carries either `text` OR `bytes`
/// (base64) — bytes covers arrows/Enter/Ctrl-C/Tab.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum ControlMessage {
    #[serde(rename = "terminal.input")]
    Input {
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        bytes: Option<String>,
    },
    #[serde(rename = "terminal.resize")]
    Resize { width: u16, height: u16 },
    #[serde(rename = "terminal.scroll")]
    Scroll { delta: i32 },
    #[serde(rename = "terminal.release")]
    Release,
}

impl ControlMessage {
    /// Convenience constructor for typed text input.
    pub fn text(s: impl Into<String>) -> Self {
        ControlMessage::Input {
            text: Some(s.into()),
            bytes: None,
        }
    }
    /// Convenience constructor for raw base64 bytes (control keys).
    pub fn raw_bytes(b64: impl Into<String>) -> Self {
        ControlMessage::Input {
            text: None,
            bytes: Some(b64.into()),
        }
    }
}

/// Agent readiness status as herdr reports it. NOTE (airemote D10): `Idle` is
/// reported for a genuinely-ready agent AND for one blocked at an unrecognized
/// prompt — the status alone carries no readiness. Tier 1 must confirm by
/// composer shape; Tier 2 (a human at the wheel) does not care.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Working,
    Blocked,
    Done,
    Idle,
}

impl AgentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentStatus::Working => "working",
            AgentStatus::Blocked => "blocked",
            AgentStatus::Done => "done",
            AgentStatus::Idle => "idle",
        }
    }
}

/// A snapshot of everything herdr currently has alive: the source of truth the
/// gateway resolves fresh every time (never caches/derives opaque ids —
/// PRD §6). Hierarchy workspace → tab → pane → agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Snapshot {
    pub workspaces: Vec<Workspace>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Workspace {
    pub id: String,
    pub label: String,
    pub tabs: Vec<Tab>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tab {
    pub id: String,
    pub label: String,
    pub panes: Vec<Pane>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Pane {
    /// Opaque herdr pane id — the internal addressing key. Never constructed by
    /// the gateway, always read fresh from the snapshot (PRD §6).
    pub id: String,
    pub agent: Option<Agent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Agent {
    /// Agent kind/name (e.g. "claude", "codex").
    pub kind: String,
    pub status: AgentStatus,
}

/// A resolved, human-readable + opaque address for one agent's pane. The
/// display path is for humans; `pane_id` is the addressing key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneTarget {
    pub pane_id: String,
    pub display: String,
}

/// Protocol handshake info returned by `ping`/version. Pinned exact-match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolInfo {
    pub protocol: u32,
    pub server_version: String,
}

impl ProtocolInfo {
    /// True only if the protocol number matches the pinned one exactly.
    pub fn is_compatible(&self) -> bool {
        self.protocol == HERDR_PROTOCOL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_round_trips_the_seven_fields() {
        let f = TerminalFrame {
            kind: FrameKind::Frame,
            seq: 42,
            encoding: "ansi".into(),
            width: 80,
            height: 24,
            full: true,
            bytes: "aGVsbG8=".into(),
        };
        let json = serde_json::to_string(&f).unwrap();
        // wire uses "type":"terminal.frame"
        assert!(json.contains("\"type\":\"terminal.frame\""));
        assert!(json.contains("\"seq\":42"));
        let back: TerminalFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(f, back);
    }

    #[test]
    fn input_serializes_text_xor_bytes() {
        let t = ControlMessage::text("ls\n");
        let j = serde_json::to_string(&t).unwrap();
        assert!(j.contains("terminal.input"));
        assert!(j.contains("\"text\":\"ls\\n\""));
        assert!(!j.contains("bytes"));

        let b = ControlMessage::raw_bytes("Aw==");
        let j = serde_json::to_string(&b).unwrap();
        assert!(j.contains("\"bytes\":\"Aw==\""));
        assert!(!j.contains("text"));
    }

    #[test]
    fn resize_and_release_shapes() {
        let r = serde_json::to_string(&ControlMessage::Resize {
            width: 100,
            height: 40,
        })
        .unwrap();
        assert!(r.contains("terminal.resize"));
        let rel = serde_json::to_string(&ControlMessage::Release).unwrap();
        assert!(rel.contains("terminal.release"));
    }

    #[test]
    fn agent_status_snake_case() {
        assert_eq!(
            serde_json::to_string(&AgentStatus::Working).unwrap(),
            "\"working\""
        );
        assert_eq!(AgentStatus::Blocked.as_str(), "blocked");
    }

    #[test]
    fn protocol_pin_is_exact_match() {
        let ok = ProtocolInfo {
            protocol: 16,
            server_version: "0.7.4".into(),
        };
        assert!(ok.is_compatible());
        let bad = ProtocolInfo {
            protocol: 17,
            server_version: "0.8.0".into(),
        };
        assert!(!bad.is_compatible());
    }
}

//! Tier 2 terminal relay — the app's main axis. A transparent pipe between an
//! xterm.js client (WebSocket) and [`HerdrStream`]. It touches nothing else:
//! no `core`, no [`HerdrControl`], no redaction (PRD §7 — the human sees the
//! real screen). Frames flow server→client in `seq` order (no backfill —
//! DISCOVERY §Tier2); input and resize flow client→server in control mode.
//!
//! Rotate is hybrid (decision 82eff9f7): in **observe** mode a resize reshapes
//! only this client's own xterm viewport (handled client-side, never touching
//! the PTY); in **control** mode a resize is a `terminal.resize` that reflows
//! the real PTY.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;

use crate::herdr::wire::{ControlMessage, PaneTarget};

use super::auth::AuthSession;
use super::AppState;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Observe,
    Control,
}

#[derive(Debug, Deserialize)]
pub struct TerminalQuery {
    pane: String,
    #[serde(default)]
    mode: Mode,
    #[serde(default = "default_cols")]
    cols: u16,
    #[serde(default = "default_rows")]
    rows: u16,
    #[serde(default)]
    takeover: bool,
}

fn default_cols() -> u16 {
    80
}
fn default_rows() -> u16 {
    24
}

/// What the browser sends us. `input` is decoded text; `bytes` is base64 for
/// control keys (arrows/Ctrl-C); `resize` is a controller resize.
#[derive(Debug, Deserialize)]
#[serde(tag = "t", rename_all = "lowercase")]
enum ClientMsg {
    Input { data: String },
    Bytes { b64: String },
    Resize { cols: u16, rows: u16 },
}

/// GET /ws/terminal — upgrade to the relay. Auth is enforced by the extractor
/// running before the upgrade; an unauthenticated request gets the same opaque
/// 404 as any protected route.
pub async fn ws_terminal(
    _auth: AuthSession,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(q): Query<TerminalQuery>,
) -> Response {
    ws.on_upgrade(move |socket| run_relay(socket, state, q))
}

async fn run_relay(socket: WebSocket, state: AppState, q: TerminalQuery) {
    // The relay builds the target directly from the client-supplied pane id
    // (which the client got from /api/agents). It deliberately does NOT call
    // HerdrControl to resolve it — a transparent pipe never touches the control
    // plane (decision da82b90f). A stale/unknown id simply fails the stream open.
    let target = PaneTarget {
        pane_id: q.pane.clone(),
        display: q.pane.clone(),
    };

    match q.mode {
        Mode::Observe => run_observe(socket, state, target, q.cols, q.rows).await,
        Mode::Control => run_control(socket, state, target, q).await,
    }
}

async fn run_observe(
    mut socket: WebSocket,
    state: AppState,
    target: PaneTarget,
    cols: u16,
    rows: u16,
) {
    let mut frames = match state.stream.observe(&target, cols, rows).await {
        Ok(f) => f,
        Err(e) => {
            let _ = socket.send(err_msg(&e.to_string())).await;
            return;
        }
    };
    loop {
        tokio::select! {
            frame = frames.next() => match frame {
                Some(Ok(f)) => {
                    if socket.send(frame_msg(&f)).await.is_err() { break; }
                }
                // Frame stream ended (closed OR raw EOF — treated the same).
                _ => break,
            },
            // Observer: inbound is only used to detect the client leaving; a
            // resize is handled entirely inside the browser's xterm viewport.
            incoming = socket.recv() => match incoming {
                Some(Ok(_)) => continue,
                _ => break,
            },
        }
    }
}

async fn run_control(socket: WebSocket, state: AppState, target: PaneTarget, q: TerminalQuery) {
    let session = match state
        .stream
        .control(&target, q.takeover, q.cols, q.rows)
        .await
    {
        Ok(s) => s,
        Err(e) => {
            let mut s = socket;
            let _ = s.send(err_msg(&e.to_string())).await;
            return;
        }
    };
    let mut frames = session.frames;
    let input = session.input;
    let (mut ws_tx, mut ws_rx) = socket.split();

    loop {
        tokio::select! {
            frame = frames.next() => match frame {
                Some(Ok(f)) => {
                    if ws_tx.send(frame_msg(&f)).await.is_err() { break; }
                }
                _ => break,
            },
            incoming = ws_rx.next() => match incoming {
                Some(Ok(Message::Text(txt))) => {
                    if let Ok(msg) = serde_json::from_str::<ClientMsg>(&txt) {
                        let control = match msg {
                            ClientMsg::Input { data } => ControlMessage::text(data),
                            ClientMsg::Bytes { b64 } => ControlMessage::raw_bytes(b64),
                            // Controller resize reflows the real PTY.
                            ClientMsg::Resize { cols, rows } => {
                                ControlMessage::Resize { width: cols, height: rows }
                            }
                        };
                        if input.send(control).await.is_err() { break; }
                    }
                }
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                _ => continue,
            },
        }
    }
    // Release the writer politely on the way out.
    let _ = input.send(ControlMessage::Release).await;
}

/// Serialize a frame as a WS text message (JSON) for the browser.
fn frame_msg(f: &crate::herdr::wire::TerminalFrame) -> Message {
    Message::Text(serde_json::to_string(f).unwrap_or_default())
}

fn err_msg(reason: &str) -> Message {
    Message::Text(serde_json::json!({ "type": "gateway.error", "reason": reason }).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_msg_shapes_parse() {
        let input: ClientMsg = serde_json::from_str(r#"{"t":"input","data":"ls\n"}"#).unwrap();
        assert!(matches!(input, ClientMsg::Input { .. }));
        let resize: ClientMsg =
            serde_json::from_str(r#"{"t":"resize","cols":100,"rows":40}"#).unwrap();
        assert!(matches!(
            resize,
            ClientMsg::Resize {
                cols: 100,
                rows: 40
            }
        ));
        let bytes: ClientMsg = serde_json::from_str(r#"{"t":"bytes","b64":"Aw=="}"#).unwrap();
        assert!(matches!(bytes, ClientMsg::Bytes { .. }));
    }

    #[test]
    fn mode_defaults_to_observe() {
        let q: TerminalQuery = serde_urlencoded::from_str("pane=p1").unwrap();
        assert_eq!(q.mode, Mode::Observe);
        assert_eq!(q.cols, 80);
    }

    #[test]
    fn control_mode_parses() {
        let q: TerminalQuery =
            serde_urlencoded::from_str("pane=p1&mode=control&cols=120&rows=30&takeover=true")
                .unwrap();
        assert_eq!(q.mode, Mode::Control);
        assert_eq!(q.cols, 120);
        assert!(q.takeover);
    }
}

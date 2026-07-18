//! Socket client for the real herdr server. Speaks `herdr.sock`'s newline-JSON
//! request/response API, **one request per connection** (PBI-001): each call
//! opens the socket, writes one `{id,method,params}\n`, reads one response line,
//! closes. Error responses carry no `id` (correlate by being the sole reply).

use std::path::PathBuf;

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use super::wire::*;
use super::{Herdr, HerdrError, Result};

/// Default socket path (herdr's per-user runtime socket).
pub fn default_socket_path() -> PathBuf {
    let base = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(".config/herdr/herdr.sock")
}

/// A herdr client bound to one socket path.
#[derive(Clone)]
pub struct SocketHerdr {
    path: PathBuf,
    counter: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl SocketHerdr {
    pub fn new(path: PathBuf) -> Self {
        SocketHerdr {
            path,
            counter: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    fn next_id(&self) -> String {
        let n = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!("gw-{n}")
    }

    /// One request → one response, on a fresh connection. Returns the `result`
    /// value, or a typed error for an `error` response / transport failure.
    async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let mut stream = UnixStream::connect(&self.path)
            .await
            .map_err(|e| HerdrError::Unavailable(e.to_string()))?;

        let req = Request {
            id: self.next_id(),
            method,
            params,
        };
        let mut line = serde_json::to_vec(&req).map_err(|e| HerdrError::Request(e.to_string()))?;
        line.push(b'\n');
        stream
            .write_all(&line)
            .await
            .map_err(|e| HerdrError::Request(e.to_string()))?;
        stream
            .flush()
            .await
            .map_err(|e| HerdrError::Request(e.to_string()))?;

        // Read until the first newline (one response per connection).
        let mut buf = Vec::with_capacity(4096);
        let mut byte = [0u8; 1];
        loop {
            let n = stream
                .read(&mut byte)
                .await
                .map_err(|e| HerdrError::Request(e.to_string()))?;
            if n == 0 {
                break; // EOF
            }
            if byte[0] == b'\n' {
                break;
            }
            buf.push(byte[0]);
            if buf.len() > 8 * 1024 * 1024 {
                return Err(HerdrError::Malformed("response too large".into()));
            }
        }
        parse_response(&buf)
    }
}

/// Extract the `result` from a response line, or map an `error` / bad shape to a
/// typed error.
fn parse_response(line: &[u8]) -> Result<Value> {
    let v: Value = serde_json::from_slice(line)
        .map_err(|e| HerdrError::Malformed(format!("{e}: {}", String::from_utf8_lossy(line))))?;
    if let Some(result) = v.get("result") {
        return Ok(result.clone());
    }
    if let Some(err) = v.get("error") {
        let msg = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        return Err(HerdrError::Request(msg.to_string()));
    }
    Err(HerdrError::Malformed(
        "response has neither result nor error".into(),
    ))
}

#[async_trait]
impl Herdr for SocketHerdr {
    async fn snapshot(&self) -> Result<Snapshot> {
        let result = self.call("session.snapshot", json!({})).await?;
        // result: { "type": "...", "snapshot": { "agents": [...] } }
        let agents_val = result
            .get("snapshot")
            .and_then(|s| s.get("agents"))
            .cloned()
            .ok_or_else(|| HerdrError::Malformed("snapshot.agents missing".into()))?;
        let agents: Vec<Agent> =
            serde_json::from_value(agents_val).map_err(|e| HerdrError::Malformed(e.to_string()))?;
        Ok(Snapshot { agents })
    }

    async fn ping(&self) -> Result<ProtocolInfo> {
        let result = self.call("ping", json!({})).await?;
        let protocol = result
            .get("protocol")
            .and_then(|p| p.as_u64())
            .ok_or_else(|| HerdrError::Malformed("ping.protocol missing".into()))?
            as u32;
        let version = result
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let info = ProtocolInfo {
            protocol,
            server_version: version,
        };
        if !info.is_compatible() {
            return Err(HerdrError::ProtocolMismatch {
                expected: HERDR_PROTOCOL,
                actual: info.protocol,
            });
        }
        Ok(info)
    }

    async fn read_pane(&self, pane_id: &str) -> Result<ScreenRead> {
        let result = self
            .call(
                "pane.read",
                json!({ "pane_id": pane_id, "source": "recent", "format": "ansi" }),
            )
            .await?;
        // result: { "type":"pane_read", "read": { "text":..., "revision":... } }
        let read = result
            .get("read")
            .ok_or_else(|| HerdrError::Malformed("pane_read.read missing".into()))?;
        let text = read
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or_default()
            .to_string();
        let revision = read.get("revision").and_then(|r| r.as_u64()).unwrap_or(0);
        Ok(ScreenRead { text, revision })
    }

    async fn send_input(&self, pane_id: &str, text: &str, submit: bool) -> Result<()> {
        if !text.is_empty() {
            self.call(
                "pane.send_input",
                json!({ "pane_id": pane_id, "text": text }),
            )
            .await?;
        }
        if submit {
            // Send≠submit: a separate Enter key submits the composer.
            self.call(
                "pane.send_input",
                json!({ "pane_id": pane_id, "keys": ["enter"] }),
            )
            .await?;
        }
        Ok(())
    }

    async fn send_keys(&self, pane_id: &str, keys: &[String]) -> Result<()> {
        if keys.is_empty() {
            return Ok(());
        }
        self.call("pane.send_keys", json!({ "pane_id": pane_id, "keys": keys }))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_response_extracts_result() {
        let line = br#"{"id":"gw-0","result":{"type":"pong","protocol":16,"version":"0.7.4"}}"#;
        let r = parse_response(line).unwrap();
        assert_eq!(r["protocol"], 16);
    }

    #[test]
    fn parse_response_maps_error() {
        let line = br#"{"error":{"message":"no such pane"}}"#;
        assert!(matches!(parse_response(line), Err(HerdrError::Request(_))));
    }

    #[test]
    fn parse_response_rejects_bad_shape() {
        assert!(matches!(
            parse_response(b"{}"),
            Err(HerdrError::Malformed(_))
        ));
    }

    #[test]
    fn default_socket_path_ends_correctly() {
        assert!(default_socket_path().ends_with(".config/herdr/herdr.sock"));
    }
}

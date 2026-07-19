//! Socket client for the real herdr server. Speaks `herdr.sock`'s newline-JSON
//! request/response API, **one request per connection** (PBI-001): each call
//! opens the socket, writes one `{id,method,params}\n`, reads one response line,
//! closes. Error responses carry no `id` (correlate by being the sole reply).

use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::time::Duration;

use async_trait::async_trait;
#[cfg(windows)]
use interprocess::local_socket::tokio::Stream as LocalStream;
#[cfg(windows)]
use interprocess::local_socket::{ConnectOptions, GenericNamespaced, ToNsName};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(unix)]
use tokio::net::UnixStream as LocalStream;

use super::wire::*;
use super::{Herdr, HerdrError, Result};

/// Default socket path (herdr's per-user runtime socket).
pub fn default_socket_path() -> Result<PathBuf> {
    default_socket_path_from_config_dir(herdr_config_dir())
}

fn herdr_config_dir() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        crate::config::native_roaming_app_data()
            .map(|base| base.join("herdr"))
            .map_err(|error| {
                HerdrError::Unavailable(format!(
                    "native Windows roaming application data is unavailable; cannot resolve herdr endpoint ({error})"
                ))
            })
    }
    #[cfg(not(windows))]
    Ok(std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config/herdr"))
}

fn default_socket_path_from_config_dir(config_dir: Result<PathBuf>) -> Result<PathBuf> {
    config_dir.map(|base| base.join("herdr.sock"))
}

/// Resolve the logical herdr endpoint shared by normal startup and doctor.
/// An explicit socket override wins, then a named session, then the historical
/// default endpoint. The logical filesystem path is retained on Windows because
/// herdr also uses it for its ownership marker.
pub fn resolve_socket_path(explicit: &str, session: &str) -> Result<PathBuf> {
    if !explicit.is_empty() {
        return Ok(PathBuf::from(explicit));
    }
    resolve_socket_path_from_config_dir(explicit, session, herdr_config_dir())
}

fn resolve_socket_path_from_config_dir(
    explicit: &str,
    session: &str,
    config_dir: Result<PathBuf>,
) -> Result<PathBuf> {
    if !explicit.is_empty() {
        return Ok(PathBuf::from(explicit));
    }
    if session.is_empty() || session == "default" {
        return default_socket_path_from_config_dir(config_dir);
    }
    if !session
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        || session == "."
        || session == ".."
    {
        return Err(HerdrError::Unavailable(
            "invalid herdr session name; use letters, digits, '.', '-' or '_'".into(),
        ));
    }
    let default = default_socket_path_from_config_dir(config_dir)?;
    let root = default
        .parent()
        .ok_or_else(|| HerdrError::Unavailable("herdr endpoint has no parent directory".into()))?;
    Ok(root.join("sessions").join(session).join("herdr.sock"))
}

#[cfg(windows)]
fn windows_endpoint_name(path: &Path) -> Result<interprocess::local_socket::Name<'_>> {
    path.as_os_str()
        .to_ns_name::<GenericNamespaced>()
        .map_err(|e| HerdrError::Unavailable(format!("invalid Windows herdr endpoint ({e})")))
}

async fn connect_local(path: &Path) -> Result<LocalStream> {
    #[cfg(unix)]
    {
        return LocalStream::connect(path)
            .await
            .map_err(|e| unavailable_connect_error(&e));
    }

    #[cfg(windows)]
    {
        const ERROR_PIPE_BUSY: i32 = 231;
        const ATTEMPTS: usize = 20;
        for attempt in 0..ATTEMPTS {
            let options = ConnectOptions::new().name(windows_endpoint_name(path)?);
            match options.connect_tokio().await {
                Ok(client) => return Ok(client),
                Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) && attempt + 1 < ATTEMPTS => {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                Err(e) => return Err(unavailable_connect_error(&e)),
            }
        }
        unreachable!("bounded named-pipe connection loop always returns")
    }
}

fn unavailable_connect_error(error: &std::io::Error) -> HerdrError {
    let reason = match error.kind() {
        std::io::ErrorKind::NotFound => "endpoint not found; start herdr for this session",
        std::io::ErrorKind::PermissionDenied => "endpoint access denied",
        std::io::ErrorKind::ConnectionRefused => "endpoint refused the connection",
        std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::BrokenPipe => {
            "endpoint closed the connection"
        }
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => "endpoint remained busy",
        _ => "could not connect to endpoint",
    };
    HerdrError::Unavailable(format!("{reason} ({error})"))
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
        let mut stream = connect_local(&self.path).await?;

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
        // result: { "type": "...", "snapshot": { "agents": [...], "workspaces": [...], "tabs": [...] } }
        let snapshot_val = result
            .get("snapshot")
            .cloned()
            .ok_or_else(|| HerdrError::Malformed("snapshot missing".into()))?;
        let agents_val = snapshot_val
            .get("agents")
            .cloned()
            .ok_or_else(|| HerdrError::Malformed("snapshot.agents missing".into()))?;
        let agents: Vec<Agent> =
            serde_json::from_value(agents_val).map_err(|e| HerdrError::Malformed(e.to_string()))?;
        // workspaces[]/tabs[] are resolved best-effort: missing or malformed
        // falls back to an empty list rather than failing the whole snapshot.
        let workspaces: Vec<Workspace> = snapshot_val
            .get("workspaces")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        let tabs: Vec<Tab> = snapshot_val
            .get("tabs")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        Ok(Snapshot {
            agents,
            workspaces,
            tabs,
        })
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
        self.call(
            "pane.send_keys",
            json!({ "pane_id": pane_id, "keys": keys }),
        )
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
        #[cfg(windows)]
        assert!(default_socket_path().unwrap().ends_with(
            Path::new("AppData")
                .join("Roaming")
                .join("herdr")
                .join("herdr.sock")
        ));
        #[cfg(not(windows))]
        assert!(default_socket_path()
            .unwrap()
            .ends_with(".config/herdr/herdr.sock"));
    }

    #[test]
    fn resolver_keeps_default_and_builds_named_session_paths() {
        assert_eq!(
            resolve_socket_path("", "default").unwrap(),
            default_socket_path().unwrap()
        );
        #[cfg(windows)]
        assert!(resolve_socket_path("", "team-1").unwrap().ends_with(
            Path::new("AppData")
                .join("Roaming")
                .join("herdr")
                .join("sessions")
                .join("team-1")
                .join("herdr.sock")
        ));
        #[cfg(not(windows))]
        assert!(resolve_socket_path("", "team-1")
            .unwrap()
            .ends_with(".config/herdr/sessions/team-1/herdr.sock"));
    }

    #[test]
    fn resolver_prefers_explicit_override_and_rejects_unsafe_sessions() {
        assert_eq!(
            resolve_socket_path("/custom/herdr.sock", "team").unwrap(),
            PathBuf::from("/custom/herdr.sock")
        );
        assert!(resolve_socket_path("", "../other").is_err());
        assert!(resolve_socket_path("", "name/other").is_err());
    }

    #[test]
    fn injected_config_dir_resolves_without_home() {
        let config_dir = PathBuf::from("C:/Users/operator/AppData/Roaming/herdr");
        assert_eq!(
            resolve_socket_path_from_config_dir("", "default", Ok(config_dir.clone())).unwrap(),
            config_dir.join("herdr.sock")
        );
        assert_eq!(
            resolve_socket_path_from_config_dir("", "team-1", Ok(config_dir.clone())).unwrap(),
            config_dir.join("sessions/team-1/herdr.sock")
        );
    }

    #[test]
    fn unavailable_config_dir_is_a_controlled_error() {
        let error = resolve_socket_path_from_config_dir(
            "",
            "default",
            Err(HerdrError::Unavailable(
                "native Windows roaming application data is unavailable".into(),
            )),
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("native Windows roaming application data is unavailable"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_endpoint_name_matches_generic_namespaced_mapping() {
        let path = Path::new(r"C:\Users\operator\AppData\Roaming\herdr\herdr.sock");
        let direct = path.as_os_str().to_ns_name::<GenericNamespaced>().unwrap();
        let gateway = windows_endpoint_name(path).unwrap();
        assert_eq!(format!("{direct:?}"), format!("{gateway:?}"));
    }
}

//! CLI subprocess adapter — drives a real herdr via its `herdr` binary. Built
//! on the **one-request-per-connection** model verified in the M0 spike
//! (DISCOVERY): control-plane calls are each their own short-lived subprocess;
//! a stream is its own long-lived subprocess whose sole job is that stream.
//!
//! **`--session` is prepended to every invocation, always** — herdr ignores the
//! `HERDR_SESSION` env var (airemote D8, re-verified in CONTEXT.md), so only the
//! explicit flag isolates the gateway's session from the operator's own.
//!
//! Exercised end-to-end against a live herdr; the Fake adapter is the substrate
//! for automated tests, so unit tests here cover only the pure NDJSON parsing.

use std::process::Stdio;

use async_trait::async_trait;
use futures_util::stream::StreamExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::wire::*;
use super::{ControlSession, FrameStream, HerdrControl, HerdrError, HerdrStream, Result};

/// A handle to a real herdr, pinned to one explicit session name.
#[derive(Clone)]
pub struct CliHerdr {
    binary: String,
    session: String,
}

impl CliHerdr {
    pub fn new(session: impl Into<String>) -> Self {
        CliHerdr {
            binary: "herdr".into(),
            session: session.into(),
        }
    }

    /// Override the herdr binary path (tests / non-standard installs).
    pub fn with_binary(mut self, binary: impl Into<String>) -> Self {
        self.binary = binary.into();
        self
    }

    /// Build a command with `--session <name>` always prepended before `args`.
    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(&self.binary);
        cmd.arg("--session").arg(&self.session);
        for a in args {
            cmd.arg(a);
        }
        cmd.stdin(Stdio::null());
        cmd
    }

    async fn run_json<T: serde::de::DeserializeOwned>(&self, args: &[&str]) -> Result<T> {
        let out = self
            .command(args)
            .output()
            .await
            .map_err(|e| HerdrError::Unavailable(e.to_string()))?;
        if !out.status.success() {
            return Err(HerdrError::Invocation(
                String::from_utf8_lossy(&out.stderr).into_owned(),
            ));
        }
        serde_json::from_slice(&out.stdout).map_err(|e| HerdrError::Malformed(e.to_string()))
    }
}

/// Parse one NDJSON line into a frame, tolerating the terminator and blank
/// lines. `Ok(None)` means "not a frame, keep going or stop"; the caller
/// distinguishes closed vs. more.
pub(crate) fn parse_frame_line(line: &str) -> ParsedLine {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return ParsedLine::Skip;
    }
    if let Ok(frame) = serde_json::from_str::<TerminalFrame>(trimmed) {
        return ParsedLine::Frame(frame);
    }
    if serde_json::from_str::<TerminalClosed>(trimmed).is_ok() {
        return ParsedLine::Closed;
    }
    // Unknown line (e.g. a graphics message we ignore) — skip, don't fail.
    ParsedLine::Skip
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ParsedLine {
    Frame(TerminalFrame),
    Closed,
    Skip,
}

#[async_trait]
impl HerdrControl for CliHerdr {
    async fn snapshot(&self) -> Result<Snapshot> {
        // `session.snapshot` — its own short-lived connection (one-req-per-conn).
        self.run_json(&["session", "snapshot", "--json"]).await
    }

    async fn ping(&self) -> Result<ProtocolInfo> {
        #[derive(serde::Deserialize)]
        struct Raw {
            protocol: u32,
            #[serde(default)]
            version: String,
        }
        let raw: Raw = self.run_json(&["status", "server", "--json"]).await?;
        let info = ProtocolInfo {
            protocol: raw.protocol,
            server_version: raw.version,
        };
        if !info.is_compatible() {
            return Err(HerdrError::ProtocolMismatch {
                expected: HERDR_PROTOCOL,
                actual: info.protocol,
            });
        }
        Ok(info)
    }
}

#[async_trait]
impl HerdrStream for CliHerdr {
    async fn observe(&self, target: &PaneTarget, cols: u16, rows: u16) -> Result<FrameStream> {
        let cols = cols.to_string();
        let rows = rows.to_string();
        let mut child = self
            .command(&[
                "terminal",
                "session",
                "observe",
                &target.pane_id,
                "--cols",
                &cols,
                "--rows",
                &rows,
            ])
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| HerdrError::Unavailable(e.to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| HerdrError::Invocation("no stdout".into()))?;
        Ok(frame_stream_from_reader(stdout, child).boxed())
    }

    async fn control(
        &self,
        target: &PaneTarget,
        takeover: bool,
        cols: u16,
        rows: u16,
    ) -> Result<ControlSession> {
        let cols = cols.to_string();
        let rows = rows.to_string();
        let mut args = vec![
            "terminal",
            "session",
            "control",
            &target.pane_id,
            "--cols",
            &cols,
            "--rows",
            &rows,
        ];
        if takeover {
            args.push("--takeover");
        }
        let mut child = self
            .command(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| HerdrError::Unavailable(e.to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| HerdrError::Invocation("no stdout".into()))?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| HerdrError::Invocation("no stdin".into()))?;

        let (tx, mut rx) = tokio::sync::mpsc::channel::<ControlMessage>(64);
        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            while let Some(msg) = rx.recv().await {
                if let Ok(mut line) = serde_json::to_vec(&msg) {
                    line.push(b'\n');
                    if stdin.write_all(&line).await.is_err() {
                        break;
                    }
                    let _ = stdin.flush().await;
                }
            }
        });
        Ok(ControlSession {
            frames: frame_stream_from_reader(stdout, child).boxed(),
            input: tx,
        })
    }
}

/// Turn a child's stdout into a frame stream. A raw EOF (no `terminal.closed`)
/// ends the stream just like a graceful close — the relay treats them
/// identically (DISCOVERY §Tier2).
fn frame_stream_from_reader(
    stdout: tokio::process::ChildStdout,
    child: tokio::process::Child,
) -> impl futures_util::Stream<Item = Result<TerminalFrame>> {
    let reader = BufReader::new(stdout);
    let lines = reader.lines();
    futures_util::stream::unfold((lines, Some(child)), |(mut lines, child)| async move {
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => match parse_frame_line(&line) {
                    ParsedLine::Frame(f) => return Some((Ok(f), (lines, child))),
                    ParsedLine::Closed => return None,
                    ParsedLine::Skip => continue,
                },
                // Raw EOF or read error — end the stream (treated as closed).
                Ok(None) | Err(_) => return None,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_frame_line() {
        let line = r#"{"type":"terminal.frame","seq":1,"encoding":"ansi","width":80,"height":24,"full":true,"bytes":"aGk="}"#;
        match parse_frame_line(line) {
            ParsedLine::Frame(f) => {
                assert_eq!(f.seq, 1);
                assert!(f.full);
            }
            other => panic!("expected frame, got {other:?}"),
        }
    }

    #[test]
    fn recognizes_closed() {
        let line = r#"{"type":"terminal.closed","reason":"exited"}"#;
        assert_eq!(parse_frame_line(line), ParsedLine::Closed);
    }

    #[test]
    fn skips_blank_and_unknown() {
        assert_eq!(parse_frame_line("   "), ParsedLine::Skip);
        assert_eq!(
            parse_frame_line(r#"{"type":"graphics","data":"x"}"#),
            ParsedLine::Skip
        );
    }

    #[test]
    fn command_always_prepends_session() {
        // We cannot exec a real herdr here, but we can assert the arg ordering by
        // building the command and inspecting its program + args indirectly via
        // a known-nonexistent binary path (construction never execs).
        let cli = CliHerdr::new("gw-session").with_binary("/nonexistent/herdr");
        let cmd = cli.command(&["status", "server"]);
        let std_cmd = cmd.as_std();
        let args: Vec<_> = std_cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args[0], "--session");
        assert_eq!(args[1], "gw-session");
        assert_eq!(args[2], "status");
    }
}

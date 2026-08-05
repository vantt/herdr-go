//! Transcript live tail — reads a Claude Code session's own on-disk JSONL
//! transcript (`~/.claude/projects/<encoded-cwd>/<session>.jsonl`) as a
//! second, *semantic* observation source beside `pane.read`'s rendered
//! screen. The file is append-only and written by Claude Code itself, so a
//! byte cursor over it is gap-free: everything between two polls is read,
//! regardless of poll interval — unlike the screen, where content that
//! scrolls past between ticks is unrecoverable
//! (`plans/260805-1500-transcript-as-history-source/plan.md`).
//!
//! Scope is deliberately "tail from now": the first call returns a cursor at
//! the file's current end and no lines (no retroactive history — user-locked
//! scope 2026-08-05); every later call returns only what was appended since.
//! The gateway keeps no per-pane state — the cursor lives in the client and
//! comes back on each request. Nothing is ever persisted (the never-store
//! rule, `src/store/mod.rs`, is untouched: the transcript is Claude Code's
//! artifact, not ours).

use std::fs;
use std::io::{Read as _, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Max complete lines of one `tool_result` rendered before eliding — a single
/// huge output must not flush the client's whole 200-line ring.
const TOOL_OUTPUT_MAX_LINES: usize = 20;
/// Max lines of one text block (user prompt / assistant message).
const TEXT_MAX_LINES: usize = 40;
/// Per-display-line char cap (clipped with an ellipsis, char-boundary safe).
const MAX_LINE_CHARS: usize = 400;
/// Max display lines returned by one poll — beyond this the *oldest* lines of
/// the poll are elided (the client trims to its own ring anyway).
const POLL_MAX_LINES: usize = 400;
/// Max transcript bytes consumed per poll. More simply waits for the next
/// tick — the cursor only advances past what was actually read, so nothing
/// is skipped.
const MAX_READ_BYTES: u64 = 4 * 1024 * 1024;

/// One poll's worth of freshly appended, already-rendered display lines plus
/// the cursor to hand back on the next poll.
#[derive(Debug)]
pub struct ActivityChunk {
    pub lines: Vec<String>,
    /// Opaque to the client: `<session-file-name>:<byte-offset>`.
    pub cursor: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TranscriptError {
    /// No transcript exists for this cwd — not an error condition for the
    /// caller, just "this pane has no Activity view".
    #[error("no transcript available for this pane")]
    NotAvailable,
    #[error("malformed activity cursor")]
    BadCursor,
    #[error("transcript read failed: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, TranscriptError>;

/// Claude Code's project-directory encoding of a working directory: every
/// non-ASCII-alphanumeric byte becomes `-` (e.g. `D:\a\b` → `D--a-b`,
/// `/home/dev/x` → `-home-dev-x`).
pub fn encode_project_dir(cwd: &str) -> String {
    cwd.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// `$CLAUDE_CONFIG_DIR/projects` when set, else `<home>/.claude/projects`
/// (`USERPROFILE` on Windows, `HOME` elsewhere — same env Claude Code uses).
fn claude_projects_root() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("CLAUDE_CONFIG_DIR") {
        return Some(PathBuf::from(dir).join("projects"));
    }
    let home_var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(home_var).map(|h| PathBuf::from(h).join(".claude").join("projects"))
}

/// Tail the transcript for `cwd` against the default Claude Code root.
pub fn read_activity(cwd: &str, cursor: Option<&str>) -> Result<ActivityChunk> {
    let root = claude_projects_root().ok_or(TranscriptError::NotAvailable)?;
    read_activity_at(&root, cwd, cursor)
}

/// [`read_activity`] with an explicit projects root — the testable seam.
pub fn read_activity_at(root: &Path, cwd: &str, cursor: Option<&str>) -> Result<ActivityChunk> {
    let dir = root.join(encode_project_dir(cwd));
    if !dir.is_dir() {
        return Err(TranscriptError::NotAvailable);
    }
    let newest = newest_jsonl(&dir)?.ok_or(TranscriptError::NotAvailable)?;
    let Some(cursor) = cursor else {
        // Open = now: cursor at the current end, no backfill.
        let len = fs::metadata(&newest)?.len();
        return Ok(ActivityChunk {
            lines: Vec::new(),
            cursor: cursor_for(&newest, len),
        });
    };
    let (file_name, offset) = parse_cursor(cursor)?;
    let path = dir.join(&file_name);
    if !path.is_file() {
        // The watched session file vanished (rotated away) — continue from
        // the newest sibling's end, saying so.
        let len = fs::metadata(&newest)?.len();
        return Ok(ActivityChunk {
            lines: vec!["— session switched —".to_string()],
            cursor: cursor_for(&newest, len),
        });
    }
    let (raw_lines, new_offset) = tail_raw(&path, offset)?;
    // Stateless mid-watch switch (plan decision 2): only once the watched
    // file is fully consumed AND a strictly newer sibling exists do we hop —
    // from the new file's end, with a visible divider.
    if raw_lines.is_empty() && newest != path && is_newer(&newest, &path)? {
        let len = fs::metadata(&newest)?.len();
        return Ok(ActivityChunk {
            lines: vec!["— session switched —".to_string()],
            cursor: cursor_for(&newest, len),
        });
    }
    let mut lines = Vec::new();
    for raw in &raw_lines {
        lines.extend(render_record(raw));
    }
    Ok(ActivityChunk {
        lines: cap_poll_lines(lines),
        cursor: cursor_for(&path, new_offset),
    })
}

/// Newest-mtime `*.jsonl` in `dir` (name as tie-break, for deterministic
/// tests on coarse-mtime filesystems).
fn newest_jsonl(dir: &Path) -> std::io::Result<Option<PathBuf>> {
    let mut best: Option<(SystemTime, String, PathBuf)> = None;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let modified = entry.metadata()?.modified()?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let candidate = (modified, name, path);
        if best
            .as_ref()
            .map(|(m, n, _)| (candidate.0, &candidate.1) > (*m, n))
            .unwrap_or(true)
        {
            best = Some(candidate);
        }
    }
    Ok(best.map(|(_, _, p)| p))
}

fn is_newer(a: &Path, b: &Path) -> std::io::Result<bool> {
    Ok(fs::metadata(a)?.modified()? > fs::metadata(b)?.modified()?)
}

fn cursor_for(path: &Path, offset: u64) -> String {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    format!("{name}:{offset}")
}

/// Parse and validate `<file-name>:<offset>`. The name is used to re-join the
/// project directory, so it must be a bare `.jsonl` file name — never a path.
fn parse_cursor(cursor: &str) -> Result<(String, u64)> {
    let (name, offset) = cursor.rsplit_once(':').ok_or(TranscriptError::BadCursor)?;
    let offset: u64 = offset.parse().map_err(|_| TranscriptError::BadCursor)?;
    let safe = !name.is_empty()
        && name.ends_with(".jsonl")
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..");
    if !safe {
        return Err(TranscriptError::BadCursor);
    }
    Ok((name.to_string(), offset))
}

/// Read complete lines appended after `offset`. A partial trailing line (no
/// `\n` yet — Claude Code is mid-append) is left for the next poll: the
/// returned offset only ever advances past the last complete line.
fn tail_raw(path: &Path, offset: u64) -> std::io::Result<(Vec<String>, u64)> {
    let mut file = fs::File::open(path)?;
    let len = file.metadata()?.len();
    if len <= offset {
        // Nothing new; `len < offset` (truncation — Claude Code never does
        // this, but stay safe) re-anchors at the current end without a dump.
        return Ok((Vec::new(), len));
    }
    file.seek(SeekFrom::Start(offset))?;
    let mut buf = Vec::new();
    file.take(MAX_READ_BYTES).read_to_end(&mut buf)?;
    let Some(last_newline) = buf.iter().rposition(|&b| b == b'\n') else {
        return Ok((Vec::new(), offset));
    };
    let complete = last_newline + 1;
    let text = String::from_utf8_lossy(&buf[..complete]);
    let lines = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_string)
        .collect();
    Ok((lines, offset + complete as u64))
}

fn cap_poll_lines(mut lines: Vec<String>) -> Vec<String> {
    if lines.len() > POLL_MAX_LINES {
        let dropped = lines.len() - POLL_MAX_LINES;
        lines = lines.split_off(dropped);
        lines.insert(0, format!("… (+{dropped} earlier lines this poll)"));
    }
    lines
}

// ---------------------------------------------------------------------------
// Record rendering — one transcript JSONL record to zero-or-more display
// lines. Defensive throughout: an unparseable line or unknown shape becomes a
// clipped raw line, never an error (nothing is silently dropped).
// ---------------------------------------------------------------------------

/// Render one raw transcript line into display lines.
pub fn render_record(raw: &str) -> Vec<String> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else {
        return vec![clip(raw.trim())];
    };
    let sidechain = v["isSidechain"].as_bool().unwrap_or(false);
    let mut lines = match v["type"].as_str().unwrap_or("") {
        // File-head index record — meta, not conversation.
        "summary" => Vec::new(),
        "assistant" => render_content(&v["message"]["content"], Role::Assistant),
        "user" => render_content(&v["message"]["content"], Role::User),
        "system" => {
            let text = v["content"]
                .as_str()
                .or_else(|| v["message"].as_str())
                .unwrap_or("");
            match text.lines().next().map(str::trim).filter(|l| !l.is_empty()) {
                Some(first) => vec![clip(&format!("[system] {first}"))],
                None => Vec::new(),
            }
        }
        "" => vec![clip(raw.trim())],
        other => vec![format!("[{other}]")],
    };
    if sidechain {
        for line in &mut lines {
            line.insert_str(0, "⑂ ");
        }
    }
    lines
}

#[derive(Clone, Copy, PartialEq)]
enum Role {
    User,
    Assistant,
}

fn render_content(content: &serde_json::Value, role: Role) -> Vec<String> {
    match content {
        serde_json::Value::String(text) => push_text(text, role),
        serde_json::Value::Array(blocks) => {
            let mut out = Vec::new();
            for block in blocks {
                match block["type"].as_str().unwrap_or("") {
                    "text" => out.extend(push_text(block["text"].as_str().unwrap_or(""), role)),
                    "thinking" => {
                        let first = block["thinking"]
                            .as_str()
                            .unwrap_or("")
                            .lines()
                            .next()
                            .unwrap_or("")
                            .trim();
                        if !first.is_empty() {
                            out.push(clip(&format!("· thinking: {first}")));
                        }
                    }
                    "tool_use" => out.push(summarize_tool_use(
                        block["name"].as_str().unwrap_or("tool"),
                        &block["input"],
                    )),
                    "tool_result" => out.extend(render_tool_result(block)),
                    "" => {}
                    other => out.push(format!("[{other}]")),
                }
            }
            out
        }
        _ => Vec::new(),
    }
}

fn push_text(text: &str, role: Role) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    for (i, line) in lines.iter().take(TEXT_MAX_LINES).enumerate() {
        let lead = if role == Role::User && i == 0 {
            "» "
        } else {
            ""
        };
        if line.trim().is_empty() && lines.len() == 1 {
            continue;
        }
        out.push(clip(&format!("{lead}{line}")));
    }
    if lines.len() > TEXT_MAX_LINES {
        out.push(format!("… (+{} lines)", lines.len() - TEXT_MAX_LINES));
    }
    out
}

fn summarize_tool_use(name: &str, input: &serde_json::Value) -> String {
    let s = |key: &str| input[key].as_str().unwrap_or("");
    let line = match name {
        "Bash" => {
            let command = s("command");
            let mut it = command.lines();
            let first = it.next().unwrap_or("");
            let more = it.count();
            if more > 0 {
                format!("> bash: {first} (+{more} more lines)")
            } else {
                format!("> bash: {first}")
            }
        }
        "Edit" | "MultiEdit" => format!("✎ edit: {}", s("file_path")),
        "Write" => format!("✎ write: {}", s("file_path")),
        "NotebookEdit" => format!("✎ edit: {}", s("notebook_path")),
        "Read" => format!("≡ read: {}", s("file_path")),
        "Glob" => format!("? glob: {}", s("pattern")),
        "Grep" => format!("? grep: {}", s("pattern")),
        "Task" => {
            let what = input["description"]
                .as_str()
                .or(input["subagent_type"].as_str())
                .unwrap_or("");
            format!("⑂ task: {what}")
        }
        "TodoWrite" => "☰ todo update".to_string(),
        "WebFetch" => format!("? fetch: {}", s("url")),
        "WebSearch" => format!("? search: {}", s("query")),
        other => {
            let compact = serde_json::to_string(input).unwrap_or_default();
            format!("> {other}: {compact}")
        }
    };
    clip(&line)
}

fn render_tool_result(block: &serde_json::Value) -> Vec<String> {
    let text = match &block["content"] {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(|i| i["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    };
    let is_error = block["is_error"].as_bool().unwrap_or(false);
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let mut out = Vec::new();
    for (i, line) in lines.iter().take(TOOL_OUTPUT_MAX_LINES).enumerate() {
        let lead = if is_error && i == 0 { "  ✗ " } else { "  " };
        out.push(clip(&format!("{lead}{line}")));
    }
    if lines.len() > TOOL_OUTPUT_MAX_LINES {
        out.push(format!(
            "  … (+{} lines)",
            lines.len() - TOOL_OUTPUT_MAX_LINES
        ));
    }
    out
}

/// Clip a display line to [`MAX_LINE_CHARS`] chars (char-boundary safe) and
/// strip control noise: `\r`, and ANSI escape sequences that tool outputs can
/// carry — this view renders text into a `<pre>`, not a terminal.
fn clip(line: &str) -> String {
    let mut out = String::new();
    let mut chars = line.chars().peekable();
    let mut count = 0usize;
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // Skip a CSI/OSC-ish sequence: ESC [ params final-byte, or ESC
            // plus one following char for the simple two-byte forms.
            if chars.peek() == Some(&'[') {
                chars.next();
                for f in chars.by_ref() {
                    if f.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                chars.next();
            }
            continue;
        }
        if c == '\r' || (c.is_control() && c != '\t') {
            continue;
        }
        if count >= MAX_LINE_CHARS {
            out.push('…');
            break;
        }
        out.push(c);
        count += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn project_dir(root: &Path, cwd: &str) -> PathBuf {
        let dir = root.join(encode_project_dir(cwd));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn encoding_matches_claude_codes_scheme() {
        assert_eq!(
            encode_project_dir(r"D:\projects\tools\AI\tools"),
            "D--projects-tools-AI-tools"
        );
        assert_eq!(
            encode_project_dir("/home/dev/projects/x"),
            "-home-dev-projects-x"
        );
    }

    #[test]
    fn open_returns_eof_cursor_and_no_backfill() {
        let root = tempfile::tempdir().unwrap();
        let dir = project_dir(root.path(), "/w/a");
        fs::write(
            dir.join("s1.jsonl"),
            "{\"type\":\"user\",\"message\":{\"content\":\"old history\"}}\n",
        )
        .unwrap();

        let chunk = read_activity_at(root.path(), "/w/a", None).unwrap();
        assert!(chunk.lines.is_empty(), "no retroactive history");
        assert!(chunk.cursor.starts_with("s1.jsonl:"));
        // Cursor sits at the file's end, so the very next poll is empty too.
        let next = read_activity_at(root.path(), "/w/a", Some(&chunk.cursor)).unwrap();
        assert!(next.lines.is_empty());
    }

    #[test]
    fn appended_records_arrive_incrementally_and_in_order() {
        let root = tempfile::tempdir().unwrap();
        let dir = project_dir(root.path(), "/w/b");
        let path = dir.join("s1.jsonl");
        fs::write(&path, "").unwrap();

        let open = read_activity_at(root.path(), "/w/b", None).unwrap();
        let mut f = fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"hello"}},{{"type":"tool_use","name":"Bash","input":{{"command":"cargo test"}}}}]}}}}"#).unwrap();
        writeln!(f, r#"{{"type":"user","message":{{"content":[{{"type":"tool_result","content":"ok: 12 passed"}}]}}}}"#).unwrap();

        let chunk = read_activity_at(root.path(), "/w/b", Some(&open.cursor)).unwrap();
        assert_eq!(
            chunk.lines,
            vec!["hello", "> bash: cargo test", "  ok: 12 passed"]
        );
        // Fully consumed — next poll is empty, cursor stable.
        let next = read_activity_at(root.path(), "/w/b", Some(&chunk.cursor)).unwrap();
        assert!(next.lines.is_empty());
        assert_eq!(next.cursor, chunk.cursor);
    }

    #[test]
    fn partial_trailing_line_is_left_for_the_next_poll() {
        let root = tempfile::tempdir().unwrap();
        let dir = project_dir(root.path(), "/w/c");
        let path = dir.join("s1.jsonl");
        fs::write(&path, "").unwrap();
        let open = read_activity_at(root.path(), "/w/c", None).unwrap();

        // Half a record, no newline yet.
        let mut f = fs::OpenOptions::new().append(true).open(&path).unwrap();
        write!(f, r#"{{"type":"assistant","mess"#).unwrap();
        let mid = read_activity_at(root.path(), "/w/c", Some(&open.cursor)).unwrap();
        assert!(mid.lines.is_empty(), "incomplete line must not render");
        assert_eq!(mid.cursor, open.cursor, "cursor must not advance");

        // The rest lands.
        writeln!(
            f,
            r#"age":{{"content":[{{"type":"text","text":"done"}}]}}}}"#
        )
        .unwrap();
        let after = read_activity_at(root.path(), "/w/c", Some(&mid.cursor)).unwrap();
        assert_eq!(after.lines, vec!["done"]);
    }

    #[test]
    fn missing_project_dir_is_not_available() {
        let root = tempfile::tempdir().unwrap();
        match read_activity_at(root.path(), "/nowhere", None) {
            Err(TranscriptError::NotAvailable) => {}
            other => panic!("expected NotAvailable, got {other:?}"),
        }
    }

    #[test]
    fn vanished_session_file_switches_to_newest_with_divider() {
        let root = tempfile::tempdir().unwrap();
        let dir = project_dir(root.path(), "/w/d");
        fs::write(dir.join("s1.jsonl"), "").unwrap();
        let open = read_activity_at(root.path(), "/w/d", None).unwrap();

        fs::remove_file(dir.join("s1.jsonl")).unwrap();
        fs::write(dir.join("s2.jsonl"), "{\"type\":\"summary\"}\n").unwrap();
        let chunk = read_activity_at(root.path(), "/w/d", Some(&open.cursor)).unwrap();
        assert_eq!(chunk.lines, vec!["— session switched —"]);
        assert!(chunk.cursor.starts_with("s2.jsonl:"));
    }

    #[test]
    fn cursor_never_escapes_the_project_dir() {
        let root = tempfile::tempdir().unwrap();
        project_dir(root.path(), "/w/e");
        fs::write(
            root.path()
                .join(encode_project_dir("/w/e"))
                .join("s1.jsonl"),
            "",
        )
        .unwrap();
        for bad in [
            "../../etc/passwd.jsonl:0",
            "..\\secrets.jsonl:0",
            "a/b.jsonl:0",
            "s1.jsonl:not-a-number",
            "no-extension:0",
            ":0",
        ] {
            match read_activity_at(root.path(), "/w/e", Some(bad)) {
                Err(TranscriptError::BadCursor) => {}
                other => panic!("cursor {bad:?} must be rejected, got {other:?}"),
            }
        }
    }

    #[test]
    fn render_covers_the_mid_session_record_shapes() {
        // Thinking collapses to one line.
        assert_eq!(
            render_record(
                r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"first line\nsecond line"}]}}"#
            ),
            vec!["· thinking: first line"]
        );
        // User prompt gets the » lead.
        assert_eq!(
            render_record(r#"{"type":"user","message":{"content":"do the thing"}}"#),
            vec!["» do the thing"]
        );
        // Sidechain frames are marked.
        assert_eq!(
            render_record(
                r#"{"type":"assistant","isSidechain":true,"message":{"content":[{"type":"text","text":"sub says hi"}]}}"#
            ),
            vec!["⑂ sub says hi"]
        );
        // Unknown tool keeps its raw input (nothing silently dropped).
        let lines = render_record(
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"mcp__x__y","input":{"a":1}}]}}"#,
        );
        assert_eq!(lines, vec![r#"> mcp__x__y: {"a":1}"#]);
        // Unparseable line falls back to clipped raw passthrough.
        assert_eq!(render_record("not json at all"), vec!["not json at all"]);
        // Summary records are meta, not conversation.
        assert!(render_record(r#"{"type":"summary","summary":"t"}"#).is_empty());
    }

    #[test]
    fn huge_tool_result_is_elided_not_flushed() {
        let big = (0..100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\\n");
        let raw = format!(
            r#"{{"type":"user","message":{{"content":[{{"type":"tool_result","content":"{big}"}}]}}}}"#
        );
        let lines = render_record(&raw);
        assert_eq!(lines.len(), TOOL_OUTPUT_MAX_LINES + 1);
        assert_eq!(lines.last().unwrap(), "  … (+80 lines)");
    }

    #[test]
    fn clip_strips_ansi_and_bounds_line_length() {
        assert_eq!(clip("a\u{1b}[31mred\u{1b}[0mb"), "aredb");
        let long: String = "x".repeat(MAX_LINE_CHARS + 50);
        let clipped = clip(&long);
        assert_eq!(clipped.chars().count(), MAX_LINE_CHARS + 1);
        assert!(clipped.ends_with('…'));
    }
}

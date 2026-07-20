//! Pure, unit-testable config write functions for the interactive doctor
//! surface (D6, D7, D9). No prompting, no stdin/stdout, no terminal use —
//! callers (future doctor code) own all interaction; this module only reads
//! and writes files and validates JSON.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{Config, ConfigError};

/// The known `config.json` field names, in canonical order. Anything outside
/// this set (including any secret/token field — deny_unknown_fields already
/// rejects those) is never written by [`repair_fields`].
pub const CONFIG_FIELDS: &[&str] = &[
    "bind_addr",
    "herdr_session",
    "allowed_roots",
    "poll_interval_ms",
    "herdr_protocol",
    "static_dir",
    "herdr_socket",
    "telegram_chat_id",
];

/// D6: persist a candidate config document only if it re-validates through
/// the same [`Config::load_str`] path used at startup. On rejection, the
/// file at `path` (if any) is left completely untouched.
pub fn persist_validated(path: &Path, candidate_json: &str) -> Result<(), PersistError> {
    Config::load_str(candidate_json).map_err(PersistError::Invalid)?;
    write_atomic(path, candidate_json.as_bytes()).map_err(PersistError::Io)
}

#[derive(Debug)]
pub enum PersistError {
    Invalid(ConfigError),
    Io(std::io::Error),
}

impl std::fmt::Display for PersistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistError::Invalid(e) => write!(f, "candidate config is invalid: {e}"),
            PersistError::Io(e) => write!(f, "failed to write config: {e}"),
        }
    }
}

impl std::error::Error for PersistError {}

/// Write `contents` to `path` such that a process interrupted mid-write can
/// never leave a truncated file in `path`'s place: write to a sibling temp
/// file, fsync it, then atomically rename over the destination (rename
/// replaces an existing destination on both unix and Windows).
fn write_atomic(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(dir) = dir {
        std::fs::create_dir_all(dir)?;
    }
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("config.json");
    let tmp_name = format!(".{file_name}.tmp-{}", rand::random::<u64>());
    let tmp_path = match dir {
        Some(dir) => dir.join(tmp_name),
        None => PathBuf::from(tmp_name),
    };
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&tmp_path)?;
        file.write_all(contents)?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

/// D7 unparseable path: `path`'s existing content cannot be parsed as JSON
/// at all, so no field is recoverable. Writes a timestamped backup of the
/// existing file BEFORE any overwrite, verifies the backup landed, then
/// persists `new_contents` through the same validated-write gate as
/// [`persist_validated`]. Returns the backup path on success.
pub fn backup_and_recreate(path: &Path, new_contents: &str) -> Result<PathBuf, BackupError> {
    let existing = std::fs::read(path).map_err(BackupError::ReadExisting)?;
    let backup_path = backup_path_for(path);
    std::fs::write(&backup_path, &existing).map_err(BackupError::WriteBackup)?;
    if !backup_path.exists() {
        return Err(BackupError::BackupNotVerified);
    }
    persist_validated(path, new_contents).map_err(BackupError::Persist)?;
    Ok(backup_path)
}

fn backup_path_for(path: &Path) -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("config.json");
    path.with_file_name(format!("{file_name}.bak-{ts}"))
}

#[derive(Debug)]
pub enum BackupError {
    ReadExisting(std::io::Error),
    WriteBackup(std::io::Error),
    BackupNotVerified,
    Persist(PersistError),
}

impl std::fmt::Display for BackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupError::ReadExisting(e) => write!(f, "failed to read existing config: {e}"),
            BackupError::WriteBackup(e) => write!(f, "failed to write backup: {e}"),
            BackupError::BackupNotVerified => {
                write!(f, "backup file was written but does not exist")
            }
            BackupError::Persist(e) => write!(f, "failed to persist recreated config: {e}"),
        }
    }
}

impl std::error::Error for BackupError {}

/// Result of a field-by-field repair attempt (D7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairOutcome {
    /// The raw document isn't parseable as a JSON object at all; there is
    /// nothing field-level to repair. Callers fall back to
    /// [`backup_and_recreate`].
    Unparseable,
    /// Every known field is valid after merging `replacements`. `json` is
    /// ready for [`persist_validated`].
    Repaired { json: String },
    /// At least one known field is still invalid after merging
    /// `replacements` — either no replacement was supplied for it, or the
    /// supplied replacement was itself invalid.
    StillInvalid { invalid_fields: Vec<String> },
}

/// Diagnose which known fields in `raw_json` are invalid or missing,
/// without changing anything. Equivalent to calling [`repair_fields`] with
/// no replacements.
pub fn diagnose_fields(raw_json: &str) -> RepairOutcome {
    repair_fields(raw_json, &HashMap::new())
}

/// D7: repair `raw_json` field-by-field. Every field already valid in the
/// raw document is preserved untouched; `replacements` supplies new values
/// for fields the caller wants to set or fix. Fields outside
/// [`CONFIG_FIELDS`] (including any secret/token field) are never carried
/// into the result.
pub fn repair_fields(raw_json: &str, replacements: &HashMap<String, Value>) -> RepairOutcome {
    let Ok(Value::Object(raw_obj)) = serde_json::from_str::<Value>(raw_json) else {
        return RepairOutcome::Unparseable;
    };

    let mut merged = serde_json::Map::new();
    for field in CONFIG_FIELDS {
        if let Some(v) = replacements.get(*field) {
            merged.insert((*field).to_string(), v.clone());
        } else if let Some(v) = raw_obj.get(*field) {
            merged.insert((*field).to_string(), v.clone());
        }
    }

    let invalid = invalid_field_names(&merged);
    if !invalid.is_empty() {
        return RepairOutcome::StillInvalid {
            invalid_fields: invalid,
        };
    }

    let json = Value::Object(merged).to_string();
    match Config::load_str(&json) {
        Ok(_) => RepairOutcome::Repaired { json },
        // Per-field checks passed but the real validator still disagrees
        // (e.g. a cross-field rule this module doesn't model) — report
        // every field rather than pretend the doc is sound.
        Err(_) => RepairOutcome::StillInvalid {
            invalid_fields: CONFIG_FIELDS.iter().map(|s| s.to_string()).collect(),
        },
    }
}

fn invalid_field_names(obj: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut invalid = Vec::new();
    if !bind_addr_ok(obj.get("bind_addr")) {
        invalid.push("bind_addr".to_string());
    }
    if !herdr_session_ok(obj.get("herdr_session")) {
        invalid.push("herdr_session".to_string());
    }
    if !allowed_roots_ok(obj.get("allowed_roots")) {
        invalid.push("allowed_roots".to_string());
    }
    if !poll_interval_ms_ok(obj.get("poll_interval_ms")) {
        invalid.push("poll_interval_ms".to_string());
    }
    if !herdr_protocol_ok(obj.get("herdr_protocol")) {
        invalid.push("herdr_protocol".to_string());
    }
    if !static_dir_ok(obj.get("static_dir")) {
        invalid.push("static_dir".to_string());
    }
    if !herdr_socket_ok(obj.get("herdr_socket")) {
        invalid.push("herdr_socket".to_string());
    }
    if !telegram_chat_id_ok(obj.get("telegram_chat_id")) {
        invalid.push("telegram_chat_id".to_string());
    }
    invalid
}

fn bind_addr_ok(v: Option<&Value>) -> bool {
    match v {
        None => true,
        Some(Value::String(s)) => s.parse::<std::net::SocketAddr>().is_ok(),
        Some(_) => false,
    }
}

fn herdr_session_ok(v: Option<&Value>) -> bool {
    matches!(v, Some(Value::String(_)))
}

fn allowed_roots_ok(v: Option<&Value>) -> bool {
    let Some(Value::Array(items)) = v else {
        // Absent defaults to an empty list, which fails closed (D-invariant
        // in Config::load_str) — so absence is itself invalid here.
        return false;
    };
    if items.is_empty() {
        return false;
    }
    items.iter().all(|item| match item {
        Value::String(s) => Path::new(s).is_absolute(),
        _ => false,
    })
}

fn poll_interval_ms_ok(v: Option<&Value>) -> bool {
    match v {
        None => true,
        Some(Value::Number(n)) => n.as_u64().is_some(),
        Some(_) => false,
    }
}

fn herdr_protocol_ok(v: Option<&Value>) -> bool {
    match v {
        None => true,
        Some(Value::Number(n)) => n.as_u64().map(|n| n <= u32::MAX as u64).unwrap_or(false),
        Some(_) => false,
    }
}

fn static_dir_ok(v: Option<&Value>) -> bool {
    matches!(v, None | Some(Value::String(_)))
}

fn herdr_socket_ok(v: Option<&Value>) -> bool {
    matches!(v, None | Some(Value::String(_)))
}

fn telegram_chat_id_ok(v: Option<&Value>) -> bool {
    matches!(v, None | Some(Value::Null) | Some(Value::String(_)))
}

/// D9: how over-broad a candidate `allowed_roots` entry is, so a caller can
/// demand typed confirmation before accepting it. `home` is the resolved
/// home directory to compare against (callers supply it — this module has
/// no opinion on how home is resolved on a given platform).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootBreadth {
    /// An ordinary directory narrower than the filesystem root or home.
    Narrow,
    /// The path has no parent — it is a filesystem root (`/`, `C:\`, ...).
    FilesystemRoot,
    /// The path is exactly the user's home directory.
    HomeDirectory,
    /// The path is a symlink, so its real target is not what it appears to be.
    Symlink,
}

pub fn classify_root_breadth(candidate: &Path, home: &Path) -> RootBreadth {
    if candidate.parent().is_none() {
        return RootBreadth::FilesystemRoot;
    }
    if candidate == home {
        return RootBreadth::HomeDirectory;
    }
    if is_symlink(candidate) {
        return RootBreadth::Symlink;
    }
    RootBreadth::Narrow
}

fn is_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn absolute_root() -> &'static str {
        "/home/op/projects"
    }

    fn valid_config_json() -> String {
        serde_json::json!({
            "herdr_session": "gateway",
            "allowed_roots": [absolute_root()],
        })
        .to_string()
    }

    #[test]
    fn persist_validated_writes_a_valid_candidate() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let json = valid_config_json();
        persist_validated(&path, &json).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), json);
    }

    #[test]
    fn persist_validated_refuses_and_leaves_existing_file_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let original = valid_config_json();
        std::fs::write(&path, &original).unwrap();

        let invalid = r#"{ "herdr_session": "g", "allowed_roots": [] }"#;
        let err = persist_validated(&path, invalid).unwrap_err();
        assert!(matches!(err, PersistError::Invalid(_)));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            original,
            "rejected candidate must never touch the existing file"
        );
    }

    #[test]
    fn persist_validated_never_writes_a_secret_field() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let with_token = serde_json::json!({
            "herdr_session": "g",
            "allowed_roots": [absolute_root()],
            "github_token": "ghp_leaked",
        })
        .to_string();
        let err = persist_validated(&path, &with_token).unwrap_err();
        assert!(matches!(err, PersistError::Invalid(ConfigError::Parse(_))));
        assert!(!path.exists());
    }

    #[test]
    fn backup_and_recreate_backs_up_before_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "not json at all {{{").unwrap();

        let new_json = valid_config_json();
        let backup_path = backup_and_recreate(&path, &new_json).unwrap();

        assert!(backup_path.exists(), "backup file must exist");
        assert_eq!(
            std::fs::read_to_string(&backup_path).unwrap(),
            "not json at all {{{"
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), new_json);
    }

    #[test]
    fn backup_and_recreate_leaves_original_when_new_contents_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "not json at all {{{").unwrap();

        let invalid = r#"{ "herdr_session": "g", "allowed_roots": [] }"#;
        let err = backup_and_recreate(&path, invalid).unwrap_err();
        assert!(matches!(err, BackupError::Persist(_)));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "not json at all {{{",
            "a rejected recreate must never overwrite the original"
        );
    }

    #[test]
    fn diagnose_fields_reports_unparseable_for_broken_json() {
        assert_eq!(diagnose_fields("not json {{{"), RepairOutcome::Unparseable);
    }

    #[test]
    fn diagnose_fields_lists_exactly_the_invalid_fields() {
        // herdr_session missing, allowed_roots empty, bind_addr malformed —
        // everything else absent-but-defaulted (so valid).
        let raw = serde_json::json!({
            "allowed_roots": [],
            "bind_addr": "not-an-addr",
        })
        .to_string();
        match diagnose_fields(&raw) {
            RepairOutcome::StillInvalid { invalid_fields } => {
                assert_eq!(
                    invalid_fields,
                    vec!["bind_addr", "herdr_session", "allowed_roots"]
                );
            }
            other => panic!("expected StillInvalid, got {other:?}"),
        }
    }

    #[test]
    fn repair_fields_preserves_valid_fields_and_fixes_only_the_invalid_one() {
        let raw = serde_json::json!({
            "herdr_session": "gateway",
            "allowed_roots": [absolute_root()],
            "bind_addr": "not-an-addr",
            "poll_interval_ms": 750,
        })
        .to_string();
        let mut replacements = HashMap::new();
        replacements.insert(
            "bind_addr".to_string(),
            Value::String("127.0.0.1:9000".to_string()),
        );

        match repair_fields(&raw, &replacements) {
            RepairOutcome::Repaired { json } => {
                let cfg = Config::load_str(&json).unwrap();
                assert_eq!(cfg.herdr_session, "gateway", "valid field preserved");
                assert_eq!(cfg.poll_interval_ms, 750, "valid field preserved");
                assert_eq!(cfg.bind_addr.to_string(), "127.0.0.1:9000", "field repaired");
            }
            other => panic!("expected Repaired, got {other:?}"),
        }
    }

    #[test]
    fn repair_fields_still_invalid_when_no_replacement_supplied() {
        let raw = serde_json::json!({
            "herdr_session": "gateway",
            "allowed_roots": [],
        })
        .to_string();
        match repair_fields(&raw, &HashMap::new()) {
            RepairOutcome::StillInvalid { invalid_fields } => {
                assert_eq!(invalid_fields, vec!["allowed_roots"]);
            }
            other => panic!("expected StillInvalid, got {other:?}"),
        }
    }

    #[test]
    fn repair_fields_drops_fields_outside_the_known_schema() {
        let raw = serde_json::json!({
            "herdr_session": "gateway",
            "allowed_roots": [absolute_root()],
            "github_token": "ghp_should_never_survive",
        })
        .to_string();
        match repair_fields(&raw, &HashMap::new()) {
            RepairOutcome::Repaired { json } => {
                assert!(!json.contains("github_token"));
            }
            other => panic!("expected Repaired, got {other:?}"),
        }
    }

    #[test]
    fn classify_root_breadth_flags_filesystem_root() {
        let home = Path::new("/home/op");
        assert_eq!(
            classify_root_breadth(Path::new("/"), home),
            RootBreadth::FilesystemRoot
        );
    }

    #[test]
    fn classify_root_breadth_flags_home_directory() {
        let home = Path::new("/home/op");
        assert_eq!(
            classify_root_breadth(Path::new("/home/op"), home),
            RootBreadth::HomeDirectory
        );
    }

    #[test]
    fn classify_root_breadth_does_not_flag_a_narrow_directory() {
        let home = Path::new("/home/op");
        assert_eq!(
            classify_root_breadth(Path::new("/home/op/projects"), home),
            RootBreadth::Narrow
        );
    }

    #[cfg(unix)]
    #[test]
    fn classify_root_breadth_flags_a_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("real");
        std::fs::create_dir(&target).unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let home = Path::new("/home/op");
        assert_eq!(classify_root_breadth(&link, home), RootBreadth::Symlink);
    }
}

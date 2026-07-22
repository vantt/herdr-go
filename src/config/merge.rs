//! Additive-only config merge for `herdr-go update` (D5, D6): seed fields the
//! running config is missing from the new version's default, never touch a
//! field the user already has, never drop an orphaned field the new default
//! no longer knows about.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::write::{self, BackupError};

/// Every reason a merge is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeError {
    /// `existing_json` or `default_json` failed to parse, or parsed to
    /// something other than a JSON object.
    NotAnObject(String),
}

impl std::fmt::Display for MergeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MergeError::NotAnObject(which) => {
                write!(f, "{which} is not a JSON object — refusing to merge")
            }
        }
    }
}

impl std::error::Error for MergeError {}

/// Merge `default_json`'s fields into `existing_json`, additive-only.
///
/// A key present in `default_json` but absent from `existing_json` is seeded
/// with the default's value. A key already present in `existing_json` is
/// never modified (D6 — user values are sacred), regardless of its value or
/// type. A key present only in `existing_json` (an orphaned field from an
/// older config shape) is preserved untouched (D6 — no rename-mapping in
/// v1). Returns the merged object serialized back to a pretty JSON string.
pub fn merge_missing_fields(existing_json: &str, default_json: &str) -> Result<String, MergeError> {
    let existing: Value = serde_json::from_str(existing_json)
        .map_err(|e| MergeError::NotAnObject(format!("existing_json ({e})")))?;
    let default: Value = serde_json::from_str(default_json)
        .map_err(|e| MergeError::NotAnObject(format!("default_json ({e})")))?;

    let mut existing_obj = existing
        .as_object()
        .cloned()
        .ok_or_else(|| MergeError::NotAnObject("existing_json".to_string()))?;
    let default_obj = default
        .as_object()
        .ok_or_else(|| MergeError::NotAnObject("default_json".to_string()))?;

    for (key, value) in default_obj {
        existing_obj
            .entry(key.clone())
            .or_insert_with(|| value.clone());
    }

    serde_json::to_string_pretty(&Value::Object(existing_obj))
        .map_err(|e| MergeError::NotAnObject(format!("serializing merged result ({e})")))
}

/// Every reason [`merge_config_on_upgrade`] refuses to complete.
#[derive(Debug)]
pub enum MergeUpgradeError {
    /// The existing config file at the given path could not be read.
    ReadExisting(std::io::Error),
    /// [`merge_missing_fields`] refused to combine the existing and default
    /// documents.
    Merge(MergeError),
    /// The merged document could not be backed up and persisted -- this also
    /// covers the fail-closed case documented on
    /// [`merge_config_on_upgrade`]: the backup lands, but the write itself is
    /// refused.
    Backup(BackupError),
}

impl std::fmt::Display for MergeUpgradeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MergeUpgradeError::ReadExisting(e) => write!(f, "failed to read existing config: {e}"),
            MergeUpgradeError::Merge(e) => write!(f, "{e}"),
            MergeUpgradeError::Backup(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for MergeUpgradeError {}

/// Compose the config-merge pipeline for `herdr-go update` (D7): read the
/// existing config at `path`, compute the running binary's default config
/// JSON (mirroring `ensure_config`'s root-selection: `home()/projects` if
/// that directory exists, else `home()`), merge missing fields into the
/// existing document via [`merge_missing_fields`], then persist the result
/// through the existing [`super::write::backup_and_recreate`] -- which backs
/// up `path` before any overwrite and validates the merged result through
/// `Config::load_str` before writing it. Returns the backup path on success.
///
/// `Config` uses `#[serde(deny_unknown_fields)]`, so if the merged JSON ever
/// carries a field the current `Config` schema doesn't recognize (only
/// possible if a future version's schema removes a field an older one had --
/// out of scope for v1 per D6, and unreachable today since no field has ever
/// been removed), `backup_and_recreate`'s own persist step refuses to write
/// it: the backup still lands, but this function returns an error rather
/// than silently writing invalid-per-schema content. That is a safe,
/// fail-closed outcome, not silent corruption -- though it means D6's
/// "orphan left untouched, no warning" promise holds only at the
/// `merge_missing_fields` layer, not end-to-end through this function, for
/// that one input shape that can't occur today.
pub fn merge_config_on_upgrade(path: &Path) -> Result<PathBuf, MergeUpgradeError> {
    let existing_json = std::fs::read_to_string(path).map_err(MergeUpgradeError::ReadExisting)?;

    let projects = super::home().join("projects");
    let root = if projects.is_dir() {
        projects
    } else {
        super::home()
    };
    let default_json = super::default_config_json(&root);

    let merged_json =
        merge_missing_fields(&existing_json, &default_json).map_err(MergeUpgradeError::Merge)?;

    write::backup_and_recreate(path, &merged_json).map_err(MergeUpgradeError::Backup)
}

/// Entry point for the hidden `--internal-merge-config` CLI verb (D5, D7):
/// this only runs after `update` has already self-exec'd the newly-swapped
/// binary, so `merge_config_on_upgrade`'s default-config source is the NEW
/// version's compiled defaults, not the old (still-running) binary's. The
/// caller's process captures this function's stdout to learn the backup
/// path for its own rollback bookkeeping — never panics, so a malformed
/// config or a busy filesystem always surfaces as a clean nonzero exit.
pub fn run_internal_merge_config(path: &Path) -> i32 {
    match merge_config_on_upgrade(path) {
        Ok(backup_path) => {
            println!("{}", backup_path.display());
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    fn absolute_root() -> &'static str {
        r"C:\projects"
    }

    #[cfg(not(windows))]
    fn absolute_root() -> &'static str {
        "/home/op/projects"
    }

    #[test]
    fn merge_adds_missing_field_with_default_value() {
        let existing = r#"{"bind_addr": "0.0.0.0:8787"}"#;
        let default = r#"{"bind_addr": "0.0.0.0:9999", "poll_interval_ms": 500}"#;

        let merged = merge_missing_fields(existing, default).expect("merge should succeed");
        let value: Value = serde_json::from_str(&merged).expect("merged output is valid JSON");

        assert_eq!(value["poll_interval_ms"], 500);
    }

    #[test]
    fn merge_preserves_existing_user_value_unchanged() {
        let existing = r#"{"bind_addr": "127.0.0.1:1234"}"#;
        let default = r#"{"bind_addr": "0.0.0.0:8787"}"#;

        let merged = merge_missing_fields(existing, default).expect("merge should succeed");
        let value: Value = serde_json::from_str(&merged).expect("merged output is valid JSON");

        assert_eq!(value["bind_addr"], "127.0.0.1:1234");
    }

    #[test]
    fn merge_preserves_orphaned_field_not_in_default() {
        let existing = r#"{"legacy_field": "keep-me", "bind_addr": "0.0.0.0:8787"}"#;
        let default = r#"{"bind_addr": "0.0.0.0:8787"}"#;

        let merged = merge_missing_fields(existing, default).expect("merge should succeed");
        let value: Value = serde_json::from_str(&merged).expect("merged output is valid JSON");

        assert_eq!(value["legacy_field"], "keep-me");
    }

    #[test]
    fn merge_errors_when_existing_is_not_a_json_object() {
        let existing = r#"["not", "an", "object"]"#;
        let default = r#"{"bind_addr": "0.0.0.0:8787"}"#;

        let result = merge_missing_fields(existing, default);

        assert_eq!(
            result,
            Err(MergeError::NotAnObject("existing_json".to_string()))
        );
    }

    #[test]
    fn merge_config_on_upgrade_backs_up_before_writing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let original = serde_json::json!({
            "herdr_session": "gateway",
            "allowed_roots": [absolute_root()],
            "bind_addr": "127.0.0.1:9000",
        })
        .to_string();
        std::fs::write(&path, &original).unwrap();

        let backup_path = merge_config_on_upgrade(&path).expect("merge should succeed");

        assert!(backup_path.exists(), "backup file must exist");
        assert_eq!(std::fs::read_to_string(&backup_path).unwrap(), original);
    }

    #[test]
    fn merge_config_on_upgrade_adds_missing_fields_and_keeps_existing_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let original = serde_json::json!({
            "herdr_session": "gateway",
            "allowed_roots": [absolute_root()],
            "bind_addr": "127.0.0.1:9000",
        })
        .to_string();
        std::fs::write(&path, &original).unwrap();

        merge_config_on_upgrade(&path).expect("merge should succeed");

        let merged = std::fs::read_to_string(&path).unwrap();
        let cfg = super::super::Config::load_str(&merged).expect("merged config is valid");
        assert_eq!(cfg.herdr_session, "gateway", "existing value kept");
        assert_eq!(
            cfg.bind_addr.to_string(),
            "127.0.0.1:9000",
            "existing value kept"
        );
        assert_eq!(
            cfg.poll_interval_ms, 500,
            "missing field seeded with default"
        );
        assert!(
            !cfg.agent_presets.is_empty(),
            "missing agent_presets seeded with default"
        );
    }

    #[test]
    fn merge_config_on_upgrade_fails_closed_on_field_unknown_to_current_schema() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let original = serde_json::json!({
            "herdr_session": "gateway",
            "allowed_roots": [absolute_root()],
            "a_field_removed_from_config_that_no_longer_exists": "orphaned-value",
        })
        .to_string();
        std::fs::write(&path, &original).unwrap();

        let err = merge_config_on_upgrade(&path).unwrap_err();

        assert!(matches!(err, MergeUpgradeError::Backup(_)));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            original,
            "a refused merge must never overwrite the existing file"
        );
    }

    #[test]
    fn run_internal_merge_config_returns_zero_and_prints_backup_path_on_success() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let original = serde_json::json!({
            "herdr_session": "gateway",
            "allowed_roots": [absolute_root()],
            "bind_addr": "127.0.0.1:9000",
        })
        .to_string();
        std::fs::write(&path, &original).unwrap();

        let code = run_internal_merge_config(&path);

        assert_eq!(code, 0);
        let backup_path = backup_path_from_merge(&path);
        assert!(backup_path.exists(), "backup file must exist on success");
    }

    #[test]
    fn run_internal_merge_config_returns_nonzero_on_merge_failure() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "not valid json").unwrap();

        let code = run_internal_merge_config(&path);

        assert_eq!(code, 1);
    }

    /// Re-derive the backup path a prior [`run_internal_merge_config`] call
    /// must have produced, without re-running the merge (which would
    /// re-consume the already-merged file as its own "existing" input).
    fn backup_path_from_merge(path: &Path) -> PathBuf {
        let parent = path.parent().unwrap();
        std::fs::read_dir(parent)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|p| p != path)
            .expect("a backup file must have been written alongside the config")
    }
}

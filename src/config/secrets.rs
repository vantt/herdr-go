//! Env-only secret writing and startup resolution (D8, D10).
//!
//! Two responsibilities, both file-only (no prompting, no stdin/stdout):
//! - **Write** (D10): create or update a single `KEY=value` line in
//!   `herdr-go.env`, replacing an existing key in place rather than appending
//!   a duplicate. Owner-only permissions are set by reusing the parent
//!   module's [`super::prepare_token_directory`] / [`super::write_new_token`] /
//!   [`super::validate_token_protection`]; permission logic is never reinvented.
//! - **Resolve** (D8): read `herdr-go.env` as a *fallback* for secrets absent
//!   from the process environment. The process environment always wins, the
//!   file is trusted only after [`super::validate_token_protection`] passes,
//!   and a missing file is not an error.
//!
//! No secret value is ever printed, logged, or rendered — including through the
//! [`super::Secrets`] `Debug` implementation.

use std::path::Path;

use super::{non_empty_env, Secrets};

/// The three environment keys that back [`Secrets`], each paired with the
/// field it populates.
const SECRET_KEYS: [&str; 3] = [
    "HERDR_GO_WEB_SECRET",
    "HERDR_GO_GITHUB_TOKEN",
    "HERDR_GO_TELEGRAM_TOKEN",
];

/// D10: write or update `key`=`value` in the canonical `herdr-go.env`,
/// replacing an existing entry in place. Owner-only permissions throughout.
pub fn write_secret(key: &str, value: &str) -> std::io::Result<()> {
    write_secret_in(&super::config_dir(), key, value)
}

/// Testable core of [`write_secret`]: operate on `herdr-go.env` inside `dir`.
///
/// The replace is done by rendering the full new file body (existing lines with
/// the target key swapped or appended, all other lines untouched) into a fresh
/// owner-only temp file in the same directory, then atomically renaming it over
/// `herdr-go.env`. [`super::write_new_token`] is never called against the live
/// path (it refuses an existing file); the temp path is always new, so it opens
/// owner-only from the first byte — there is no window at the wrong permissions.
fn write_secret_in(dir: &Path, key: &str, value: &str) -> std::io::Result<()> {
    super::prepare_token_directory(dir)?;
    let env_path = dir.join("herdr-go.env");
    let existing = std::fs::read_to_string(&env_path).unwrap_or_default();
    let new_body = replace_or_append(&existing, key, value);

    let tmp_path = dir.join(format!(".herdr-go.env.tmp-{}", rand::random::<u64>()));
    if let Err(e) = super::write_new_token(&tmp_path, new_body.as_bytes()) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e);
    }
    if let Err(e) = std::fs::rename(&tmp_path, &env_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e);
    }
    super::validate_token_protection(&env_path)
}

/// Render the new `herdr-go.env` body: replace the first `key=` line in place
/// (dropping any later duplicates of the same key), or append `key=value` when
/// the key is absent. Every other line is preserved verbatim. The result always
/// ends with a trailing newline.
fn replace_or_append(existing: &str, key: &str, value: &str) -> String {
    let prefix = format!("{key}=");
    let new_line = format!("{key}={value}");
    let mut out: Vec<String> = Vec::new();
    let mut replaced = false;
    for line in existing.lines() {
        if line.trim_start().starts_with(&prefix) {
            if !replaced {
                out.push(new_line.clone());
                replaced = true;
            }
            continue;
        }
        out.push(line.to_string());
    }
    if !replaced {
        out.push(new_line);
    }
    let mut body = out.join("\n");
    body.push('\n');
    body
}

/// D8: resolve all three secrets, consulting the process environment first and
/// falling back to a trusted `herdr-go.env` at `env_path` for any key the
/// environment does not provide.
pub fn resolve_from_env_and_file(env_path: &Path) -> Secrets {
    let file = read_trusted_env_file(env_path);
    let resolve = |key: &str| -> Option<String> {
        non_empty_env(key).or_else(|| {
            file.as_ref()
                .and_then(|m| m.get(key))
                .filter(|v| !v.trim().is_empty())
                .cloned()
        })
    };
    Secrets {
        web_session_secret: resolve("HERDR_GO_WEB_SECRET"),
        github_token: resolve("HERDR_GO_GITHUB_TOKEN"),
        telegram_bot_token: resolve("HERDR_GO_TELEGRAM_TOKEN"),
    }
}

/// Read `herdr-go.env` only if it exists and passes owner-only protection.
/// A missing file returns `None` (not an error). A file that fails protection
/// returns `None` after a non-fatal diagnostic that names the path but never a
/// value — the file is ignored, never silently trusted.
fn read_trusted_env_file(env_path: &Path) -> Option<std::collections::HashMap<String, String>> {
    if !env_path.exists() {
        return None;
    }
    if let Err(e) = super::validate_token_protection(env_path) {
        eprintln!(
            "warning: ignoring {} as a secret source: {e}",
            env_path.display()
        );
        return None;
    }
    let text = std::fs::read_to_string(env_path).ok()?;
    let mut map = std::collections::HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if SECRET_KEYS.contains(&k.trim()) {
                map.insert(k.trim().to_string(), v.to_string());
            }
        }
    }
    Some(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_secret_creates_env_file_owner_only() {
        let dir = tempfile::tempdir().unwrap();
        write_secret_in(dir.path(), "HERDR_GO_GITHUB_TOKEN", "ghp_abc").unwrap();
        let env_path = dir.path().join("herdr-go.env");
        assert!(env_path.exists(), "env file created");
        assert_eq!(
            std::fs::read_to_string(&env_path).unwrap(),
            "HERDR_GO_GITHUB_TOKEN=ghp_abc\n"
        );
        super::super::validate_token_protection(&env_path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&env_path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn write_secret_replaces_existing_key_and_preserves_others() {
        let dir = tempfile::tempdir().unwrap();
        write_secret_in(dir.path(), "HERDR_GO_WEB_SECRET", "keep-me").unwrap();
        write_secret_in(dir.path(), "HERDR_GO_GITHUB_TOKEN", "old").unwrap();
        write_secret_in(dir.path(), "HERDR_GO_GITHUB_TOKEN", "new").unwrap();
        let text = std::fs::read_to_string(dir.path().join("herdr-go.env")).unwrap();
        assert_eq!(
            text.matches("HERDR_GO_GITHUB_TOKEN=").count(),
            1,
            "exactly one line for the rewritten key, no duplicate"
        );
        assert!(text.contains("HERDR_GO_GITHUB_TOKEN=new"), "new value present");
        assert!(!text.contains("HERDR_GO_GITHUB_TOKEN=old"), "old value gone");
        assert!(
            text.contains("HERDR_GO_WEB_SECRET=keep-me"),
            "other key preserved unchanged"
        );
    }

    #[cfg(unix)]
    #[test]
    fn rewritten_env_file_stays_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        write_secret_in(dir.path(), "HERDR_GO_GITHUB_TOKEN", "a").unwrap();
        write_secret_in(dir.path(), "HERDR_GO_GITHUB_TOKEN", "b").unwrap();
        let env_path = dir.path().join("herdr-go.env");
        assert_eq!(
            std::fs::metadata(&env_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        super::super::validate_token_protection(&env_path).unwrap();
    }

    #[test]
    fn absent_env_file_yields_all_none_and_no_error() {
        std::env::remove_var("HERDR_GO_TELEGRAM_TOKEN");
        let dir = tempfile::tempdir().unwrap();
        let env_path = dir.path().join("herdr-go.env");
        let secrets = resolve_from_env_and_file(&env_path);
        assert!(secrets.telegram_bot_token.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn env_file_provides_fallback_when_process_env_absent() {
        std::env::remove_var("HERDR_GO_TELEGRAM_TOKEN");
        let dir = tempfile::tempdir().unwrap();
        write_secret_in(dir.path(), "HERDR_GO_TELEGRAM_TOKEN", "file-tele").unwrap();
        let secrets = resolve_from_env_and_file(&dir.path().join("herdr-go.env"));
        assert_eq!(secrets.telegram_bot_token.as_deref(), Some("file-tele"));
    }

    #[cfg(unix)]
    #[test]
    fn process_env_wins_over_env_file() {
        let dir = tempfile::tempdir().unwrap();
        write_secret_in(dir.path(), "HERDR_GO_GITHUB_TOKEN", "from-file").unwrap();
        std::env::set_var("HERDR_GO_GITHUB_TOKEN", "from-env");
        let secrets = resolve_from_env_and_file(&dir.path().join("herdr-go.env"));
        std::env::remove_var("HERDR_GO_GITHUB_TOKEN");
        assert_eq!(secrets.github_token.as_deref(), Some("from-env"));
    }

    #[cfg(unix)]
    #[test]
    fn env_file_failing_protection_is_ignored_not_used() {
        use std::os::unix::fs::PermissionsExt;
        std::env::remove_var("HERDR_GO_TELEGRAM_TOKEN");
        let dir = tempfile::tempdir().unwrap();
        let env_path = dir.path().join("herdr-go.env");
        std::fs::write(&env_path, "HERDR_GO_TELEGRAM_TOKEN=leaked\n").unwrap();
        std::fs::set_permissions(&env_path, std::fs::Permissions::from_mode(0o640)).unwrap();
        let secrets = resolve_from_env_and_file(&env_path);
        assert!(
            secrets.telegram_bot_token.is_none(),
            "a world/group-readable env file must not be trusted as a secret source"
        );
    }

    #[test]
    fn secrets_debug_never_renders_a_token_value() {
        let secrets = Secrets {
            web_session_secret: Some("web-super-secret".to_string()),
            github_token: Some("ghp_do_not_print".to_string()),
            telegram_bot_token: Some("tele-do-not-print".to_string()),
        };
        let rendered = format!("{secrets:?}");
        assert!(!rendered.contains("web-super-secret"));
        assert!(!rendered.contains("ghp_do_not_print"));
        assert!(!rendered.contains("tele-do-not-print"));
    }
}

//! Binary backup and atomic swap (D3/D9). Writes new bytes via a
//! write-then-rename so an interrupted write never leaves a truncated
//! binary in place, and backs up the previous binary so a failed
//! post-restart health check can roll back to it.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, thiserror::Error)]
pub enum SwapError {
    #[error("failed to read current binary at {path:?}: {source}")]
    ReadCurrent {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write backup to {path:?}: {source}")]
    WriteBackup {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write staged binary to {path:?}: {source}")]
    WriteStaged {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to set executable permission on {path:?}: {source}")]
    SetPermissions {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to swap staged binary into {path:?}: {source}")]
    Rename {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("target path {0:?} has no parent directory")]
    NoParentDir(PathBuf),
    #[error("target path {0:?} has no filename")]
    NoFileName(PathBuf),
}

/// Backs up `target_path`'s current bytes to a sibling `<filename>.bak-<unix-nanos>`
/// file, then atomically swaps `new_bytes` into `target_path` via write-to-temp
/// + rename. Returns the backup file's path on success.
pub fn backup_and_swap_binary(target_path: &Path, new_bytes: &[u8]) -> Result<PathBuf, SwapError> {
    let parent = target_path
        .parent()
        .ok_or_else(|| SwapError::NoParentDir(target_path.to_path_buf()))?;
    let file_name = target_path
        .file_name()
        .ok_or_else(|| SwapError::NoFileName(target_path.to_path_buf()))?
        .to_string_lossy()
        .into_owned();

    let current_bytes = std::fs::read(target_path).map_err(|source| SwapError::ReadCurrent {
        path: target_path.to_path_buf(),
        source,
    })?;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let backup_path = parent.join(format!("{file_name}.bak-{nanos}"));
    std::fs::write(&backup_path, &current_bytes).map_err(|source| SwapError::WriteBackup {
        path: backup_path.clone(),
        source,
    })?;

    let staged_path = parent.join(format!("{file_name}.staged-{nanos}"));
    std::fs::write(&staged_path, new_bytes).map_err(|source| SwapError::WriteStaged {
        path: staged_path.clone(),
        source,
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged_path, std::fs::Permissions::from_mode(0o755)).map_err(
            |source| SwapError::SetPermissions {
                path: staged_path.clone(),
                source,
            },
        )?;
    }

    std::fs::rename(&staged_path, target_path).map_err(|source| SwapError::Rename {
        path: target_path.to_path_buf(),
        source,
    })?;

    Ok(backup_path)
}

/// Thin wrapper over [`backup_and_swap_binary`] for the real update flow,
/// targeting the currently running executable. Deliberately untested here —
/// exercising it would require replacing this test process's own binary.
pub fn backup_and_swap_running_binary(new_bytes: &[u8]) -> Result<PathBuf, SwapError> {
    let current_exe = std::env::current_exe().map_err(|source| SwapError::ReadCurrent {
        path: PathBuf::from("<current_exe>"),
        source,
    })?;
    backup_and_swap_binary(&current_exe, new_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_and_swap_binary_backs_up_old_content_and_writes_new_bytes() {
        let dir = std::env::temp_dir().join(format!(
            "herdr-swap-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("fake-binary");
        std::fs::write(&target, b"old-content").unwrap();

        let backup_path = backup_and_swap_binary(&target, b"new-content").unwrap();

        assert_eq!(std::fs::read(&backup_path).unwrap(), b"old-content");
        assert_eq!(std::fs::read(&target).unwrap(), b"new-content");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn backup_and_swap_binary_sets_executable_permission_on_unix() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!(
            "herdr-swap-perm-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("fake-binary");
        std::fs::write(&target, b"old-content").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).unwrap();

        backup_and_swap_binary(&target, b"new-content").unwrap();

        let mode = std::fs::metadata(&target).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);

        std::fs::remove_dir_all(&dir).unwrap();
    }
}

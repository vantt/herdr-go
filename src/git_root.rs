//! Resolve the nearest enclosing git repository root above a folder, so a
//! workspace anchored at a subdirectory of a project seeds new panes at the
//! project's own root instead of wherever the operator happened to `cd` into.

use std::path::{Path, PathBuf};

/// Walk up from `path` (inclusive) to the nearest ancestor holding a `.git`
/// entry — a directory for an ordinary clone, a file for a worktree (whose
/// `.git` is a pointer, not a directory) — so both are recognized the same
/// way. Returns `None` when no ancestor has one; the caller then falls back
/// to `path` itself.
pub fn nearest_git_root(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn finds_git_root_from_nested_subdirectory() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        let nested = dir.path().join("a/b/c");
        fs::create_dir_all(&nested).unwrap();
        assert_eq!(nearest_git_root(&nested), Some(dir.path().to_path_buf()));
    }

    #[test]
    fn finds_git_root_when_path_itself_is_the_root() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        assert_eq!(nearest_git_root(dir.path()), Some(dir.path().to_path_buf()));
    }

    #[test]
    fn recognizes_a_worktree_git_file_not_only_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(".git"),
            "gitdir: /elsewhere/.git/worktrees/x\n",
        )
        .unwrap();
        assert_eq!(nearest_git_root(dir.path()), Some(dir.path().to_path_buf()));
    }

    #[test]
    fn returns_none_when_no_ancestor_has_a_git_entry() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("x/y");
        fs::create_dir_all(&nested).unwrap();
        assert_eq!(nearest_git_root(&nested), None);
    }

    #[test]
    fn stops_at_the_nearest_root_not_a_further_ancestor() {
        // An outer .git (e.g. a parent monorepo) must not shadow the inner
        // repo actually containing the path.
        let outer = tempfile::tempdir().unwrap();
        fs::create_dir(outer.path().join(".git")).unwrap();
        let inner = outer.path().join("vendor/inner-repo");
        fs::create_dir_all(&inner).unwrap();
        fs::create_dir(inner.join(".git")).unwrap();
        let nested = inner.join("src");
        fs::create_dir_all(&nested).unwrap();
        assert_eq!(nearest_git_root(&nested), Some(inner));
    }
}

//! Resolve the nearest enclosing git repository root above a folder, so a
//! workspace anchored at a subdirectory of a project seeds new panes at the
//! project's own root instead of wherever the operator happened to `cd` into.

use std::path::{Path, PathBuf};

/// Walk up from `path` (inclusive) to the nearest ancestor holding a `.git`
/// **directory** — an ordinary clone's real root. A `.git` **file** (a
/// worktree's or submodule's pointer, e.g. `gitdir: /elsewhere/.git/
/// worktrees/x`) is deliberately not a stopping point: every worktree this
/// tool creates lives nested inside its own project's checkout (`.claude/
/// worktrees/...`), so continuing the walk-up past that pointer lands on the
/// actual main checkout a few directories further up, exactly what a pane
/// seeded from inside a worktree should anchor to instead. Returns `None`
/// when no ancestor has a real `.git` directory; the caller then falls back
/// to `path` itself.
pub fn nearest_git_root(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(dir) = current {
        if dir.join(".git").is_dir() {
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
    fn walks_past_a_worktrees_git_file_to_the_real_enclosing_checkout() {
        // A worktree's own `.git` is a file (a pointer), not a directory --
        // it must never be mistaken for the root itself. Mirrors this
        // repo's own layout: a worktree nested under a project's
        // `.claude/worktrees/<id>`, whose project root is the real
        // (directory) `.git`.
        let project = tempfile::tempdir().unwrap();
        fs::create_dir(project.path().join(".git")).unwrap();
        let worktree = project.path().join(".claude/worktrees/tsk-abc");
        fs::create_dir_all(&worktree).unwrap();
        fs::write(
            worktree.join(".git"),
            "gitdir: /elsewhere/.git/worktrees/x\n",
        )
        .unwrap();
        let nested = worktree.join("src");
        fs::create_dir(&nested).unwrap();
        assert_eq!(
            nearest_git_root(&nested),
            Some(project.path().to_path_buf())
        );
    }

    #[test]
    fn returns_none_when_only_a_worktree_git_file_exists_with_no_real_root_above_it() {
        // A worktree living outside any project checkout (no directory
        // .git anywhere above it) has nothing valid to resolve to -- the
        // caller's own None fallback (the raw anchor path) applies.
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(".git"),
            "gitdir: /elsewhere/.git/worktrees/x\n",
        )
        .unwrap();
        assert_eq!(nearest_git_root(dir.path()), None);
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

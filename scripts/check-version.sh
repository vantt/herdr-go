#!/usr/bin/env bash
# Version-bump discipline for herdr-go (docs/distillery/deep-dives/version-lockstep-release-discipline.md).
#
# herdr-go has exactly one canonical version file, Cargo.toml, so unlike
# collie there is nothing to keep in lockstep across files. What matters
# instead: functional changes must bump it, and it must never move backwards.
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cargo_toml="$repo_root/Cargo.toml"

# Functional paths that require a version bump when changed. Doc-only paths
# (docs/, plans/, *.md) are deliberately excluded.
functional_paths=(src/ web/src/ scripts/ Cargo.toml)

version_from_blob() {
  grep -m1 '^version = ' | sed -E 's/^version = "([^"]+)"/\1/'
}

version_at_ref() {
  git show "$1:Cargo.toml" | version_from_blob
}

version_staged() {
  git show :Cargo.toml | version_from_blob
}

version_worktree() {
  version_from_blob < "$cargo_toml"
}

monotonic_or_die() {
  local prev="$1" cur="$2" top
  top=$(printf '%s\n%s\n' "$prev" "$cur" | sort -V | tail -1)
  if [ "$top" != "$cur" ]; then
    echo "check-version: version went backwards ($prev -> $cur)" >&2
    exit 1
  fi
}

cmd="${1:-pre-commit}"

case "$cmd" in
  pre-commit)
    prev=$(version_at_ref HEAD)
    cur=$(version_staged)
    monotonic_or_die "$prev" "$cur"

    if [ "$prev" = "$cur" ]; then
      functional_changed=$(git diff --cached --name-only -- "${functional_paths[@]}")
      if [ -n "$functional_changed" ]; then
        echo "check-version: functional changes staged but Cargo.toml version unchanged ($cur)." >&2
        echo "  Bump the version in Cargo.toml, or bypass with: SKIP_VERSION_CHECK=1 git commit ..." >&2
        exit 1
      fi
    fi
    ;;
  ci)
    # Safety net for the pre-commit hook (which can be skipped with
    # --no-verify or SKIP_VERSION_CHECK=1): compares the working tree
    # version against the latest released tag.
    latest_tag=$(git tag --list 'v*' --sort=-v:refname | head -1 || true)
    if [ -z "$latest_tag" ]; then
      echo "check-version: no vX.Y.Z tags found yet, skipping monotonic check"
      exit 0
    fi
    prev="${latest_tag#v}"
    cur=$(version_worktree)
    monotonic_or_die "$prev" "$cur"
    ;;
  *)
    echo "usage: check-version.sh [pre-commit|ci]" >&2
    exit 1
    ;;
esac

echo "check-version: ok"

#!/usr/bin/env bash
# Version-bump discipline for herdr-go (docs/distillery/deep-dives/version-lockstep-release-discipline.md).
#
# herdr-go has exactly one canonical version file, Cargo.toml. What matters:
# functional changes must bump it, it must never move backwards, and
# herdr-plugin.toml's own `version` field (required by herdr's plugin
# manifest spec, docs/distillery/deep-dives/thin-herdr-plugin-launcher.md)
# must always mirror it exactly -- the one duplicate version string this repo
# carries, kept from drifting the way collie needed a 3-layer lockstep to
# prevent.
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cargo_toml="$repo_root/Cargo.toml"
plugin_manifest="$repo_root/herdr-plugin.toml"

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

manifest_version_worktree() {
  version_from_blob < "$plugin_manifest"
}

manifest_version_staged() {
  git show :herdr-plugin.toml | version_from_blob
}

manifest_in_sync_or_die() {
  local cargo_version="$1" manifest_version="$2"
  if [ "$cargo_version" != "$manifest_version" ]; then
    echo "check-version: herdr-plugin.toml version ($manifest_version) does not match Cargo.toml ($cargo_version) -- bump both together." >&2
    exit 1
  fi
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
    manifest_in_sync_or_die "$cur" "$(manifest_version_staged)"

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
    manifest_in_sync_or_die "$cur" "$(manifest_version_worktree)"
    ;;
  drift)
    # Informational only -- never fails the build. The pre-commit hook bumps
    # Cargo.toml on every functional commit, but nothing bumps a release tag
    # to match; the two can silently drift apart for weeks (0.1.9-0.1.12
    # shipped four bumps with zero releases before anyone noticed). This
    # surfaces that gap in every CI run on main instead of leaving it
    # invisible until someone happens to ask.
    latest_tag=$(git tag --list 'v*' --sort=-v:refname | head -1 || true)
    if [ -z "$latest_tag" ]; then
      echo "check-version: no vX.Y.Z tags found yet, skipping drift check"
      exit 0
    fi
    latest_version="${latest_tag#v}"
    cur=$(version_worktree)
    if [ "$latest_version" = "$cur" ]; then
      echo "check-version: main ($cur) matches the latest release ($latest_tag) -- nothing pending"
    else
      tag_date=$(git log -1 --format=%cd --date=short "$latest_tag" 2>/dev/null || echo "unknown date")
      echo "check-version: main is at $cur; the latest release is $latest_tag ($tag_date)." >&2
      echo "  Not a failure -- version bumps land ahead of releases by design. If this gap looks" >&2
      echo "  older than expected, cut a release: scripts/release.sh" >&2
    fi
    ;;
  *)
    echo "usage: check-version.sh [pre-commit|ci|drift]" >&2
    exit 1
    ;;
esac

echo "check-version: ok"

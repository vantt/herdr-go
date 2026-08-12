#!/usr/bin/env bash
# Cuts a release: checks the preconditions a manual `git tag && git push` has
# no way to enforce, then tags Cargo.toml's version and pushes the tag, which
# is the one action that actually triggers .github/workflows/release.yml
# (docs/distillery/deep-dives/version-lockstep-release-discipline.md --
# tagging is deliberately the single manual step in the whole version
# discipline, since it is also the irreversible "this shipped" signal).
#
# Usage:
#   scripts/release.sh              # check, confirm, tag, push
#   scripts/release.sh --dry-run    # check and report only, tag nothing
#   scripts/release.sh --yes        # skip the confirmation prompt
set -euo pipefail

say() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m!!\033[0m %s\n' "$*" >&2; }
die() { echo "error: $*" >&2; exit 1; }

dry_run=0
assume_yes=0
for arg in "$@"; do
  case "$arg" in
    --dry-run) dry_run=1 ;;
    --yes|-y) assume_yes=1 ;;
    *) die "unknown argument: $arg (usage: $0 [--dry-run] [--yes])" ;;
  esac
done

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

command -v git >/dev/null 2>&1 || die "git is required"

# --- branch and working tree -----------------------------------------------
branch=$(git rev-parse --abbrev-ref HEAD)
[ "$branch" = "main" ] || die "on branch '$branch', not main -- switch to main before releasing"

if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  git status --short --untracked-files=no >&2
  die "working tree has uncommitted changes -- commit, stash, or discard them first"
fi

# --- in sync with origin ----------------------------------------------------
say "Fetching origin..."
git fetch origin --quiet --tags

local_head=$(git rev-parse HEAD)
remote_head=$(git rev-parse origin/main 2>/dev/null) || die "origin/main not found -- is the remote reachable?"
if [ "$local_head" != "$remote_head" ]; then
  die "local main ($local_head) != origin/main ($remote_head) -- push or pull first. The tag that triggers release.yml only matters once it points at a commit origin already has."
fi

# --- version and tag ---------------------------------------------------------
version=$(grep -m1 '^version = ' Cargo.toml | sed -E 's/^version = "([^"]+)"$/\1/')
[ -n "$version" ] || die "could not read version from Cargo.toml"
tag="v$version"

bash "$repo_root/scripts/check-version.sh" ci

if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
  die "tag $tag already exists locally -- has this version already been released? (git tag -d $tag to remove a mistaken local tag)"
fi
if git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null 2>&1; then
  die "tag $tag already exists on origin -- this version was already released"
fi

# --- CI status on this commit ------------------------------------------------
# Advisory, not a hard gate: windows-server-2022 is a known-broken job
# unrelated to most changes, and a maintainer may have a specific reason to
# ship anyway. Required jobs are the ones that actually build/test what
# release.yml will package.
required_jobs=(version rust web)
ci_ok=1
if command -v gh >/dev/null 2>&1; then
  repo_slug=$(gh repo view --json nameWithOwner -q .nameWithOwner 2>/dev/null) || repo_slug=""
  if [ -n "$repo_slug" ]; then
    say "Checking CI status for $local_head..."
    for job in "${required_jobs[@]}"; do
      conclusion=$(gh api "repos/$repo_slug/commits/$local_head/check-runs" \
        --jq ".check_runs[] | select(.name==\"$job\") | .conclusion" 2>/dev/null | head -1)
      case "$conclusion" in
        success) say "  $job: success" ;;
        "") warn "  $job: no check-run found yet (still running, or CI hasn't started)"; ci_ok=0 ;;
        *) warn "  $job: $conclusion"; ci_ok=0 ;;
      esac
    done
  else
    warn "could not resolve the GitHub repo slug via gh -- skipping CI status check"
  fi
else
  warn "gh not installed -- skipping CI status check"
fi

# --- summary -----------------------------------------------------------------
echo
say "Ready to release:"
echo "  version : $version"
echo "  tag     : $tag"
echo "  commit  : $local_head"
if [ "$ci_ok" -eq 0 ]; then
  warn "one or more required CI jobs are not green on this commit (see above)"
fi
echo

if [ "$dry_run" -eq 1 ]; then
  say "Dry run -- stopping before tagging."
  exit 0
fi

if [ "$assume_yes" -ne 1 ]; then
  read -r -p "Tag and push $tag? This is irreversible once pushed. [y/N] " reply
  case "$reply" in
    y|Y|yes|YES) ;;
    *) say "Aborted -- nothing tagged."; exit 1 ;;
  esac
fi

say "Tagging $tag..."
git tag -a "$tag" -m "herdr-go $version"
say "Pushing $tag..."
git push origin "$tag"

say "Pushed. release.yml is building the release now:"
if [ -n "${repo_slug:-}" ]; then
  echo "  https://github.com/$repo_slug/actions"
  echo "  https://github.com/$repo_slug/releases/tag/$tag  (live once the build jobs finish)"
fi

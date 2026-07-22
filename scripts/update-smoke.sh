#!/usr/bin/env bash
# Proves the herdr-go update lifecycle end-to-end on macOS: install an older
# pinned release via install.sh, run `herdr-go update`, and confirm a real
# newer binary is now running (health fingerprint changed) before tearing
# down. Sibling of scripts/macos-install-smoke.sh, which proves the
# install/crash-restart/uninstall lifecycle instead of the update lifecycle
# (D1-D4, D8-D10 of docs/history/self-update-merge-config/CONTEXT.md).
set -uo pipefail

HEALTH_URL="http://127.0.0.1:8787/api/health"
LABEL="io.github.vantt.herdr-go"
CONFIG_DIR="$HOME/Library/Application Support/herdr-go"
CONFIG_FILE="$CONFIG_DIR/config.json"
TOKEN_FILE="$CONFIG_DIR/herdr-go.env"
PLIST="$HOME/Library/LaunchAgents/$LABEL.plist"
BIN_DIR="${PREFIX:-$HOME/.local}/bin"
BIN_PATH="$BIN_DIR/herdr-go"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_SH="$(cd "$SCRIPT_DIR/.." && pwd)/install.sh"

say() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
die() { printf 'ASSERTION FAILED: %s\n' "$*" >&2; exit 1; }

wait_until() {
  local description="$1" timeout="$2" probe="$3"
  local deadline=$((SECONDS + timeout))
  while (( SECONDS < deadline )); do
    if "$probe"; then return 0; fi
    sleep 0.5
  done
  die "timed out waiting for $description"
}

health_up() {
  local code
  code="$(curl -s -o /dev/null -w '%{http_code}' --max-time 5 "$HEALTH_URL" 2>/dev/null)" || true
  [[ "$code" == "200" ]]
}

health_down() { ! health_up; }

# /api/health's body is `{"version":"...","protocol":N,"herdr_up":bool}`
# (src/web/api.rs Health struct, serde_json field order). No jq dependency --
# a plain grep/sed extraction of the quoted version string is enough here.
fetch_version() {
  curl -s --max-time 5 "$HEALTH_URL" 2>/dev/null \
    | grep -o '"version":"[^"]*"' \
    | sed -E 's/"version":"([^"]*)"/\1/'
}

# install.sh's own say() prints the token via ANSI-colored "==> Login token: <token>".
# Redact everything after the literal marker regardless of surrounding escape codes,
# so the plaintext token never reaches the CI log before ::add-mask:: below applies.
redact_install_output() {
  sed -E 's/(Login token: ).*/\1<redacted>/'
}

cleanup() {
  local domain="gui/$(id -u)"
  launchctl bootout "$domain/$LABEL" >/dev/null 2>&1 || true
  local pid
  pid="$(pgrep -f "$BIN_PATH" 2>/dev/null || true)"
  [[ -n "$pid" ]] && kill -9 $pid 2>/dev/null || true
}
trap cleanup EXIT

[[ -n "${HERDR_GO_SMOKE_FROM_VERSION:-}" ]] || die "HERDR_GO_SMOKE_FROM_VERSION must be set"
[[ "$HERDR_GO_SMOKE_FROM_VERSION" != "latest" ]] || die "HERDR_GO_SMOKE_FROM_VERSION must not be latest -- pin the exact older tag under test"
[[ -f "$INSTALL_SH" ]] || die "install.sh not found at $INSTALL_SH"

# --- install the OLDER pinned version ------------------------------------
# install.sh itself only reads HERDR_GO_VERSION -- it has no concept of
# HERDR_GO_SMOKE_FROM_VERSION, so translate explicitly before invoking it.
export HERDR_GO_VERSION="$HERDR_GO_SMOKE_FROM_VERSION"
say "Running install.sh (older version $HERDR_GO_VERSION, to be updated below)"
install_output="$(bash "$INSTALL_SH" 2>&1)"
install_status=$?
printf '%s\n' "$install_output" | redact_install_output
[[ $install_status -eq 0 ]] || die "install.sh failed"

[[ -f "$PLIST" ]] || die "LaunchAgent plist not found at $PLIST"
wait_until "gateway to respond on /api/health after install" 30 health_up
say "Gateway is live after install"

# --- capture + mask the login token -------------------------------------
[[ -f "$TOKEN_FILE" ]] || die "token file not found at $TOKEN_FILE"
token=""
while IFS= read -r line; do
  if [[ "$line" == HERDR_GO_WEB_SECRET=* ]]; then
    value="${line#HERDR_GO_WEB_SECRET=}"
    if [[ -n "$value" ]]; then
      token="$value"
      break
    fi
  fi
done < "$TOKEN_FILE"
[[ -n "$token" ]] || die "login token was not created"
echo "::add-mask::$token"
say "Captured and masked login token"

# --- capture the pre-update fingerprint ----------------------------------
version_before="$(fetch_version)"
[[ -n "$version_before" ]] || die "could not read version from /api/health before update"
say "Running version before update: $version_before"

# --- run the real `herdr-go update` command -------------------------------
# No special env needed for the update command itself -- it always targets
# the real latest release (D1).
say "Running herdr-go update"
update_output="$("$BIN_PATH" update 2>&1)"
update_status=$?
printf '%s\n' "$update_output"
[[ $update_status -eq 0 ]] || die "herdr-go update exited $update_status"

# --- confirm a real update happened ---------------------------------------
# `update` stops/swaps/restarts the service internally (perform_update), so
# health may blip down and back up around the restart.
wait_until "gateway to respond on /api/health after update" 30 health_up
say "Gateway is live after update"

version_after="$(fetch_version)"
[[ -n "$version_after" ]] || die "could not read version from /api/health after update"
say "Running version after update: $version_after"
[[ "$version_after" != "$version_before" ]] || die "version fingerprint did not change after update -- update was a no-op"
say "Confirmed real update: fingerprint changed from $version_before to $version_after"

# --- uninstall and verify clean removal ----------------------------------
say "Running install.sh --uninstall"
uninstall_output="$(bash "$INSTALL_SH" --uninstall 2>&1)"
uninstall_status=$?
printf '%s\n' "$uninstall_output"
[[ $uninstall_status -eq 0 ]] || die "install.sh --uninstall failed"

[[ ! -f "$PLIST" ]] || die "LaunchAgent plist still exists at $PLIST after uninstall"
[[ ! -f "$BIN_PATH" ]] || die "binary still exists at $BIN_PATH after uninstall"
[[ -d "$CONFIG_DIR" ]] || die "config dir $CONFIG_DIR was removed by uninstall -- must be left untouched"
[[ -f "$CONFIG_FILE" ]] || die "config.json was removed by uninstall -- must be left untouched"
[[ -f "$TOKEN_FILE" ]] || die "token file was removed by uninstall -- must be left untouched"
say "Uninstall verified: LaunchAgent and binary removed, config/data/token left untouched"

unset token
echo "herdr-go update smoke passed (no secrets emitted)."

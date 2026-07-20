#!/usr/bin/env bash
# Proves the install.sh -> LaunchAgent -> crash-restart -> uninstall lifecycle
# end-to-end on macOS. macOS sibling of scripts/windows-install-smoke.ps1
# (D2 of docs/history/macos-installer-runtime-smoke/CONTEXT.md).
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

[[ -n "${HERDR_GO_VERSION:-}" ]] || die "HERDR_GO_VERSION must be set"
[[ "$HERDR_GO_VERSION" != "latest" ]] || die "HERDR_GO_VERSION must not be latest -- pin the exact tag under test"
[[ -f "$INSTALL_SH" ]] || die "install.sh not found at $INSTALL_SH"

# --- install -----------------------------------------------------------
say "Running install.sh (version $HERDR_GO_VERSION)"
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

# --- crash the running process and prove LaunchAgent recovery -----------
pid="$(pgrep -f "$BIN_PATH" | head -n1)"
[[ -n "$pid" ]] || die "no running herdr-go process found at $BIN_PATH"
say "Killing herdr-go (pid $pid) to simulate a crash"
kill -9 "$pid"

wait_until "gateway to stop responding after the simulated crash" 15 health_down
say "Gateway confirmed down after crash; waiting for launchd ThrottleInterval recovery"
wait_until "gateway to recover via launchd restart" 20 health_up
say "Gateway recovered after crash -- launchd restart proven"

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
echo "macOS installer runtime smoke passed (no secrets emitted)."

#!/usr/bin/env bash
# Self-contained installer for herdr-go (Linux + macOS). Requires no repository checkout.
set -euo pipefail

REPO="vantt/herdr-go"
PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"

say() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m warning:\033[0m %s\n' "$*" >&2; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

ACTION="install"
for arg in "$@"; do
  case "$arg" in
    --uninstall) ACTION="uninstall" ;;
    *) die "unknown argument: $arg (supported flags: --uninstall)" ;;
  esac
done

OS="$(uname -s)"
case "$OS" in
  Linux|Darwin) ;;
  *) die "the one-command service installer currently supports Linux and macOS only; build from source: https://github.com/${REPO}/blob/main/docs/advanced/source-build.md" ;;
esac

# Per-OS native locations. These MUST match the paths the binary itself
# resolves in src/config/mod.rs (config_dir()/data_dir()) so the installer and
# the running service agree: Linux uses XDG, macOS uses ~/Library/Application
# Support (a single native directory, per this feature's D1).
if [[ "$OS" == Darwin ]]; then
  APP_SUPPORT="$HOME/Library/Application Support"
  CONFIG_DIR="$APP_SUPPORT/herdr-go"
  DATA_DIR="$APP_SUPPORT/herdr-go"
  LEGACY_CONFIG_DIR="$APP_SUPPORT/herdr-gateway"
  LEGACY_DATA_DIR="$APP_SUPPORT/herdr-gateway"
  LABEL="io.github.vantt.herdr-go"
  LAUNCH_AGENTS_DIR="$HOME/Library/LaunchAgents"
  PLIST="$LAUNCH_AGENTS_DIR/$LABEL.plist"
else
  CONFIG_BASE="${XDG_CONFIG_HOME:-$HOME/.config}"
  DATA_BASE="${XDG_DATA_HOME:-$HOME/.local/share}"
  CONFIG_DIR="$CONFIG_BASE/herdr-go"
  DATA_DIR="$DATA_BASE/herdr-go"
  LEGACY_CONFIG_DIR="$CONFIG_BASE/herdr-gateway"
  LEGACY_DATA_DIR="$DATA_BASE/herdr-gateway"
  UNIT_DIR="$CONFIG_BASE/systemd/user"
  UNIT="herdr-go.service"
fi

CONFIG_FILE="$CONFIG_DIR/config.json"
ENV_FILE="$CONFIG_DIR/herdr-go.env"

# --uninstall: remove only the binary and the platform service registration.
# Config, data, and the login token are always left untouched (D10).
if [[ "$ACTION" == uninstall ]]; then
  if [[ "$OS" == Darwin ]]; then
    domain="gui/$(id -u)"
    if [[ -f "$PLIST" ]]; then
      launchctl bootout "$domain/$LABEL" >/dev/null 2>&1 \
        || launchctl unload "$PLIST" >/dev/null 2>&1 || true
      rm -f "$PLIST"
      say "Removed LaunchAgent $PLIST"
    else
      warn "no LaunchAgent found at $PLIST"
    fi
  else
    if command -v systemctl >/dev/null 2>&1; then
      systemctl --user disable --now "$UNIT" >/dev/null 2>&1 || true
    fi
    if [[ -f "$UNIT_DIR/$UNIT" ]]; then
      rm -f "$UNIT_DIR/$UNIT"
      command -v systemctl >/dev/null 2>&1 && systemctl --user daemon-reload >/dev/null 2>&1 || true
      say "Removed systemd unit $UNIT_DIR/$UNIT"
    else
      warn "no systemd unit found at $UNIT_DIR/$UNIT"
    fi
  fi
  if [[ -f "$BIN_DIR/herdr-go" ]]; then
    rm -f "$BIN_DIR/herdr-go"
    say "Removed binary $BIN_DIR/herdr-go"
  else
    warn "no binary found at $BIN_DIR/herdr-go"
  fi
  say "Left untouched: config, data, and login token under $CONFIG_DIR and $DATA_DIR"
  exit 0
fi

case "$OS" in
  Linux)
    case "$(uname -m)" in
      x86_64|amd64) TARGET="x86_64-unknown-linux-musl" ;;
      aarch64|arm64) TARGET="aarch64-unknown-linux-musl" ;;
      *) die "no published binary for this architecture; build from source: https://github.com/${REPO}/blob/main/docs/advanced/source-build.md" ;;
    esac
    ;;
  Darwin)
    # Only Apple Silicon has a published asset; Intel Macs are out of scope (D11).
    case "$(uname -m)" in
      arm64|aarch64) TARGET="aarch64-apple-darwin" ;;
      *) die "no published macOS binary for this architecture (Intel Macs are not supported by the prebuilt installer); build from source: https://github.com/${REPO}/blob/main/docs/advanced/source-build.md" ;;
    esac
    ;;
esac

# Prove the service manager is reachable before moving legacy state or
# creating/downloading/installing anything.
if [[ "$OS" == Darwin ]]; then
  command -v launchctl >/dev/null 2>&1 \
    || die "launchctl is required; this installer supports macOS with a working per-user launchd session"
else
  command -v systemctl >/dev/null 2>&1 \
    || die "systemctl is required; this installer supports systemd-based Linux with a working user service manager"
  systemctl --user show-environment >/dev/null 2>&1 \
    || die "the systemd user service manager is not reachable; log in through a systemd session or use the source-build instructions: https://github.com/${REPO}/blob/main/docs/advanced/source-build.md"
fi

migrate_dir() {
  local legacy="$1" canonical="$2" kind="$3"
  if [[ -e "$canonical" ]]; then
    [[ ! -e "$legacy" ]] || warn "both legacy and canonical $kind directories exist; using $canonical and leaving $legacy untouched"
  elif [[ -e "$legacy" ]]; then
    say "Migrating $kind → $canonical"
    mv "$legacy" "$canonical" || die "could not migrate $legacy to $canonical; no replacement state was created"
  fi
}

# Fail closed before creating canonical directories or starting a service.
migrate_dir "$LEGACY_CONFIG_DIR" "$CONFIG_DIR" config
migrate_dir "$LEGACY_DATA_DIR" "$DATA_DIR" data

version="${HERDR_GO_VERSION:-latest}"
asset="herdr-go-${TARGET}"
if [[ "$version" == latest ]]; then
  url="https://github.com/${REPO}/releases/latest/download/${asset}.tar.gz"
else
  url="https://github.com/${REPO}/releases/download/${version}/${asset}.tar.gz"
fi
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
say "Downloading $asset"
curl -fSL --proto '=https' -o "$tmp_dir/release.tar.gz" "$url" || die "no suitable release asset was available; build from source: https://github.com/${REPO}/blob/main/docs/advanced/source-build.md"
tar xzf "$tmp_dir/release.tar.gz" -C "$tmp_dir" || die "downloaded release archive could not be extracted"
[[ -f "$tmp_dir/$asset/herdr-go" ]] || die "release archive does not contain $asset/herdr-go"

mkdir -p "$BIN_DIR" "$CONFIG_DIR" "$DATA_DIR"
install -m 0755 "$tmp_dir/$asset/herdr-go" "$BIN_DIR/herdr-go"

if [[ ! -f "$CONFIG_FILE" ]]; then
  "$BIN_DIR/herdr-go" --internal-print-default-config > "$CONFIG_FILE" \
    || die "could not generate default config.json from $BIN_DIR/herdr-go"
  [[ -s "$CONFIG_FILE" ]] || die "$BIN_DIR/herdr-go --internal-print-default-config produced empty output"
fi
if [[ ! -f "$ENV_FILE" ]]; then
  umask 077
  token="$(openssl rand -hex 24 2>/dev/null || od -An -N24 -tx1 /dev/urandom | tr -d ' \n')"
  printf 'HERDR_GO_WEB_SECRET=%s\n' "$token" > "$ENV_FILE"
  chmod 600 "$ENV_FILE"
  say "Login token: $token"
fi

if [[ "$OS" == Darwin ]]; then
  mkdir -p "$LAUNCH_AGENTS_DIR"
  # The plist carries NO secret and NO environment-injection block. The binary
  # resolves HERDR_GO_WEB_SECRET itself at startup from its own config
  # directory (src/config/mod.rs ensure_web_secret), the direct analog of
  # systemd's EnvironmentFile= — so the launcher never handles the token.
  cat > "$PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>$LABEL</string>
  <key>ProgramArguments</key>
  <array>
    <string>$BIN_DIR/herdr-go</string>
    <string>--config</string>
    <string>$CONFIG_FILE</string>
  </array>
  <key>KeepAlive</key>
  <dict>
    <key>SuccessfulExit</key>
    <false/>
  </dict>
  <key>ThrottleInterval</key>
  <integer>3</integer>
  <key>RunAtLoad</key>
  <true/>
  <key>ProcessType</key>
  <string>Background</string>
</dict>
</plist>
EOF

  # Idempotent load: bootout any existing registration first so re-running the
  # installer never fails with "service already loaded" or duplicates it.
  domain="gui/$(id -u)"
  if launchctl print "$domain/$LABEL" >/dev/null 2>&1; then
    launchctl bootout "$domain/$LABEL" >/dev/null 2>&1 || true
  fi
  if ! launchctl bootstrap "$domain" "$PLIST" >/dev/null 2>&1; then
    # Older macOS without `bootstrap`: fall back to the legacy load verb.
    launchctl load -w "$PLIST" >/dev/null 2>&1 \
      || die "could not load the LaunchAgent; inspect it with: launchctl bootstrap $domain \"$PLIST\""
  fi
  say "Installed. The herdr-go LaunchAgent is loaded and starts at login."
  say "Status: launchctl print $domain/$LABEL"
  say "Uninstall: curl -fSL --proto '=https' https://github.com/$REPO/releases/latest/download/install.sh | bash -s -- --uninstall"
  say "On repeat installs, retrieve or rotate the existing login token using: https://github.com/$REPO/blob/main/docs/installation.md#login-token"
else
  mkdir -p "$UNIT_DIR"
  for conflict in herdr-gateway.service herdr-gateway-dev.service herdr-go-dev.service; do
    systemctl --user disable --now "$conflict" >/dev/null 2>&1 || true
  done
  rm -f "$UNIT_DIR/herdr-gateway.service" "$UNIT_DIR/herdr-gateway-dev.service"

  cat > "$UNIT_DIR/$UNIT" <<EOF
[Unit]
Description=herdr-go — web remote gateway + supervisor for herdr
Documentation=https://github.com/$REPO
After=network-online.target
Wants=network-online.target
[Service]
Type=simple
ExecStart=$BIN_DIR/herdr-go --config $CONFIG_FILE
EnvironmentFile=$ENV_FILE
Restart=always
RestartSec=3
NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=$DATA_DIR $CONFIG_DIR
[Install]
WantedBy=default.target
EOF

  command -v loginctl >/dev/null && loginctl enable-linger "$USER" 2>/dev/null || true
  systemctl --user daemon-reload
  systemctl --user enable "$UNIT"
  say "Installed. Start with: systemctl --user start $UNIT"
  say "Logs: journalctl --user -u $UNIT -f"
  say "On repeat installs, retrieve or rotate the existing login token using: https://github.com/$REPO/blob/main/docs/installation.md#login-token"
fi

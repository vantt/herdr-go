#!/usr/bin/env bash
# Self-contained Linux installer for herdr-go. Requires no repository checkout.
set -euo pipefail

REPO="vantt/herdr-go"
PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
CONFIG_BASE="${XDG_CONFIG_HOME:-$HOME/.config}"
DATA_BASE="${XDG_DATA_HOME:-$HOME/.local/share}"
CONFIG_DIR="$CONFIG_BASE/herdr-go"
DATA_DIR="$DATA_BASE/herdr-go"
LEGACY_CONFIG_DIR="$CONFIG_BASE/herdr-gateway"
LEGACY_DATA_DIR="$DATA_BASE/herdr-gateway"
UNIT_DIR="$CONFIG_BASE/systemd/user"
UNIT="herdr-go.service"

say() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m warning:\033[0m %s\n' "$*" >&2; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

[[ "$(uname -s)" == Linux ]] || die "the one-command service installer currently supports Linux only; build from source: https://github.com/${REPO}/blob/main/docs/advanced/source-build.md"
case "$(uname -m)" in
  x86_64|amd64) TARGET="x86_64-unknown-linux-musl" ;;
  aarch64|arm64) TARGET="aarch64-unknown-linux-musl" ;;
  *) die "no published binary for this architecture; build from source: https://github.com/${REPO}/blob/main/docs/advanced/source-build.md" ;;
esac

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

version="${HERDCTL_VERSION:-latest}"
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
[[ -f "$tmp_dir/$asset/herdctl" ]] || die "release archive does not contain $asset/herdctl"

mkdir -p "$BIN_DIR" "$CONFIG_DIR" "$DATA_DIR" "$UNIT_DIR"
install -m 0755 "$tmp_dir/$asset/herdctl" "$BIN_DIR/herdctl"

CONFIG_FILE="$CONFIG_DIR/config.json"
if [[ ! -f "$CONFIG_FILE" ]]; then
  projects="$HOME/projects"; [[ -d "$projects" ]] || projects="$HOME"
  printf '{\n  "bind_addr": "0.0.0.0:8787",\n  "herdr_session": "default",\n  "allowed_roots": ["%s"],\n  "poll_interval_ms": 500,\n  "herdr_protocol": 16,\n  "static_dir": "static"\n}\n' "$projects" > "$CONFIG_FILE"
fi
ENV_FILE="$CONFIG_DIR/herdctl.env"
if [[ ! -f "$ENV_FILE" ]]; then
  umask 077
  token="$(openssl rand -hex 24 2>/dev/null || od -An -N24 -tx1 /dev/urandom | tr -d ' \n')"
  printf 'HERDCTL_WEB_SECRET=%s\n' "$token" > "$ENV_FILE"
  chmod 600 "$ENV_FILE"
  say "Login token: $token"
fi

for conflict in herdr-gateway.service herdr-gateway-dev.service herdr-go-dev.service; do
  systemctl --user disable --now "$conflict" >/dev/null 2>&1 || true
done
rm -f "$UNIT_DIR/herdr-gateway.service" "$UNIT_DIR/herdr-gateway-dev.service"

cat > "$UNIT_DIR/$UNIT" <<EOF
[Unit]
Description=herdr-go (herdctl) — web remote gateway + supervisor for herdr
Documentation=https://github.com/$REPO
After=network-online.target
Wants=network-online.target
[Service]
Type=simple
ExecStart=$BIN_DIR/herdctl --config $CONFIG_FILE
EnvironmentFile=$ENV_FILE
Restart=always
RestartSec=3
NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=$DATA_DIR $CONFIG_DIR
ProtectHome=read-only
[Install]
WantedBy=default.target
EOF

command -v loginctl >/dev/null && loginctl enable-linger "$USER" 2>/dev/null || true
systemctl --user daemon-reload
systemctl --user enable "$UNIT"
say "Installed. Start with: systemctl --user start $UNIT"
say "Logs: journalctl --user -u $UNIT -f"

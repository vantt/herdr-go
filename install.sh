#!/usr/bin/env bash
# herdr-gateway installer — installs a prebuilt herdctl binary when a matching
# GitHub release exists, falling back to building herdctl + the web UI from
# source (Rust + Node) otherwise, and sets it up as a self-healing systemd
# *user* service (one unit, lingering enabled), matching the supervision
# model in the PRD: systemd watches the gateway, the gateway watches herdr.
#
# Idempotent: re-running upgrades in place. Nothing here needs root.
#
# Env overrides:
#   HERDCTL_VERSION   release tag to install, e.g. v0.2.0 (default: latest)
set -euo pipefail

REPO="vantt/herdr-gateway"

# ---- paths -------------------------------------------------------------------
PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/herdr-gateway"
UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

say() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m warning:\033[0m %s\n' "$*"; }

# ---- target detection ---------------------------------------------------------
# Mirrors release.yml's build matrix: musl static builds for Linux (any
# distro, no glibc dependency), Apple Silicon for macOS. Anything else (incl.
# Intel macs, Windows) has no prebuilt asset and falls back to a source build.
detect_target() {
  local os arch os_part arch_part
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os" in
    Linux) os_part="unknown-linux-musl" ;;
    Darwin) os_part="apple-darwin" ;;
    *) return 1 ;;
  esac
  case "$arch" in
    x86_64|amd64) arch_part="x86_64" ;;
    aarch64|arm64) arch_part="aarch64" ;;
    *) return 1 ;;
  esac
  case "${arch_part}-${os_part}" in
    x86_64-unknown-linux-musl|aarch64-unknown-linux-musl|aarch64-apple-darwin)
      printf '%s\n' "${arch_part}-${os_part}" ;;
    *) return 1 ;;
  esac
}

# ---- prebuilt-binary download --------------------------------------------------
# On success, prints the path to the extracted herdctl binary (inside an
# isolated temp dir the caller is responsible for cleaning up) and returns 0.
# No checksum/signature verification (per D 3168932d — HTTPS-from-GitHub is
# the trust boundary). Requires neither cargo nor npm.
download_prebuilt() {
  local target="$1"
  local version="${HERDCTL_VERSION:-latest}"
  local url tmp_dir archive bin_path

  if [[ "$version" == "latest" ]]; then
    url="https://github.com/${REPO}/releases/latest/download/herdr-gateway-${target}.tar.gz"
  else
    url="https://github.com/${REPO}/releases/download/${version}/herdr-gateway-${target}.tar.gz"
  fi

  tmp_dir="$(mktemp -d)"
  archive="$tmp_dir/herdr-gateway.tar.gz"

  if ! curl -fSL --proto '=https' -o "$archive" "$url" 2>/dev/null; then
    rm -rf "$tmp_dir"
    return 1
  fi

  if ! tar xzf "$archive" -C "$tmp_dir" 2>/dev/null; then
    rm -rf "$tmp_dir"
    return 1
  fi

  bin_path="$tmp_dir/herdr-gateway-${target}/herdctl"
  if [[ ! -f "$bin_path" ]]; then
    rm -rf "$tmp_dir"
    return 1
  fi

  chmod +x "$bin_path"
  printf '%s\n' "$bin_path"
}

# ---- obtain the binary: prebuilt download, or build from source ---------------
BIN_SRC=""
PREBUILT_TMP_DIR=""

if TARGET="$(detect_target)"; then
  say "Detected target: $TARGET — checking for a prebuilt release…"
  if BIN_SRC="$(download_prebuilt "$TARGET")"; then
    say "Downloaded prebuilt binary for $TARGET"
    PREBUILT_TMP_DIR="$(dirname "$(dirname "$BIN_SRC")")"
  else
    warn "no matching prebuilt release found; building from source"
  fi
else
  warn "no prebuilt release available for this platform; building from source"
fi

if [[ -z "$BIN_SRC" ]]; then
  command -v cargo >/dev/null || { echo "cargo (Rust) is required"; exit 1; }
  command -v npm   >/dev/null || { echo "npm (Node.js) is required"; exit 1; }

  say "Bundling the web UI…"
  ( cd "$REPO_DIR/web" && npm install --silent && npm run bundle --silent )

  say "Compiling herdctl (release)…"
  ( cd "$REPO_DIR" && cargo build --release --quiet )

  BIN_SRC="$REPO_DIR/target/release/herdctl"
fi

# ---- install artifacts -------------------------------------------------------
say "Installing binary → $BIN_DIR/herdctl"
mkdir -p "$BIN_DIR" "$CONFIG_DIR" "$UNIT_DIR"
install -m 0755 "$BIN_SRC" "$BIN_DIR/herdctl"

[[ -n "$PREBUILT_TMP_DIR" ]] && rm -rf "$PREBUILT_TMP_DIR"

# ---- config (never overwrite an existing one) --------------------------------
CONFIG_FILE="$CONFIG_DIR/config.json"
if [[ ! -f "$CONFIG_FILE" ]]; then
  say "Writing starter config → $CONFIG_FILE (edit it before first run)"
  cp "$REPO_DIR/config.example.json" "$CONFIG_FILE"
else
  say "Keeping existing config → $CONFIG_FILE"
fi

# ---- secrets -----------------------------------------------------------------
ENV_FILE="$CONFIG_DIR/herdctl.env"
if [[ ! -f "$ENV_FILE" ]]; then
  say "Creating secrets file → $ENV_FILE (mode 600)"
  umask 077
  cat > "$ENV_FILE" <<'EOF'
# herdr-gateway secrets — NEVER commit this file. Loaded by the systemd unit.
# Web login token (required). Generate one, e.g.: openssl rand -hex 24
HERDCTL_WEB_SECRET=
# Optional — GitHub token for project provisioning (repo scope).
HERDCTL_GITHUB_TOKEN=
# Optional — Telegram bot token for blocked/done notifications.
HERDCTL_TELEGRAM_TOKEN=
EOF
  chmod 600 "$ENV_FILE"
  warn "Set HERDCTL_WEB_SECRET in $ENV_FILE before starting the service."
else
  say "Keeping existing secrets → $ENV_FILE"
fi

# ---- systemd user unit -------------------------------------------------------
say "Installing systemd user unit → $UNIT_DIR/herdr-gateway.service"
sed -e "s#@BIN@#$BIN_DIR/herdctl#g" \
    -e "s#@CONFIG@#$CONFIG_FILE#g" \
    -e "s#@ENVFILE@#$ENV_FILE#g" \
    "$REPO_DIR/packaging/herdr-gateway.service" > "$UNIT_DIR/herdr-gateway.service"

# Survive logout/reboot: enable lingering so the user manager runs at boot.
if command -v loginctl >/dev/null; then
  loginctl enable-linger "$USER" 2>/dev/null || warn "could not enable linger (need it for boot-start)"
fi

systemctl --user daemon-reload
systemctl --user enable herdr-gateway.service

say "Done. Next:"
echo "  1. Set HERDCTL_WEB_SECRET in $ENV_FILE"
echo "  2. Review $CONFIG_FILE (bind_addr, allowed_roots, herdr_session)"
echo "  3. Start it:  systemctl --user start herdr-gateway"
echo "  4. Logs:      journalctl --user -u herdr-gateway -f"
echo
echo "  Try it right now without herdr:  herdctl --demo"

#!/usr/bin/env bash
# herdr-gateway installer — builds herdctl + the web UI and installs them as a
# self-healing systemd *user* service (one unit, lingering enabled), matching
# the supervision model in the PRD: systemd watches the gateway, the gateway
# watches herdr.
#
# Idempotent: re-running upgrades in place. Nothing here needs root.
set -euo pipefail

# ---- paths -------------------------------------------------------------------
PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
SHARE_DIR="$PREFIX/share/herdr-gateway"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/herdr-gateway"
UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

say() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m warning:\033[0m %s\n' "$*"; }

# ---- prerequisites -----------------------------------------------------------
command -v cargo >/dev/null || { echo "cargo (Rust) is required"; exit 1; }
command -v npm   >/dev/null || { echo "npm (Node.js) is required"; exit 1; }

# ---- build -------------------------------------------------------------------
say "Compiling herdctl (release)…"
( cd "$REPO_DIR" && cargo build --release --quiet )

say "Bundling the web UI…"
( cd "$REPO_DIR/web" && npm install --silent && npm run bundle --silent )

# ---- install artifacts -------------------------------------------------------
say "Installing binary → $BIN_DIR/herdctl"
mkdir -p "$BIN_DIR" "$SHARE_DIR" "$CONFIG_DIR" "$UNIT_DIR"
install -m 0755 "$REPO_DIR/target/release/herdctl" "$BIN_DIR/herdctl"

say "Installing web assets → $SHARE_DIR/static"
rm -rf "$SHARE_DIR/static"
cp -r "$REPO_DIR/static" "$SHARE_DIR/static"

# ---- config (never overwrite an existing one) --------------------------------
CONFIG_FILE="$CONFIG_DIR/config.json"
if [[ ! -f "$CONFIG_FILE" ]]; then
  say "Writing starter config → $CONFIG_FILE (edit it before first run)"
  sed "s#\"static\"#\"$SHARE_DIR/static\"#" "$REPO_DIR/config.example.json" > "$CONFIG_FILE"
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

#!/usr/bin/env bash
# Dev-as-live deploy: compile the repo and make THIS build the live instance.
# Each run rebuilds herdctl + the web UI and (re)starts a systemd *user* service
# that runs straight from the repo's build output — no copy, no reinstall. So
# "compile finished" == "the running real instance is updated".
#
# Usage:  ./dev-deploy.sh            # build + (re)start the dev service
#         ./dev-deploy.sh --logs     # ... then follow the logs
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/herdr-gateway"
ENV_FILE="$CONFIG_DIR/herdctl.env"
UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
UNIT="herdr-gateway-dev.service"

say() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }

command -v cargo >/dev/null || { echo "cargo (Rust) is required"; exit 1; }
command -v npm >/dev/null || { echo "npm (Node.js) is required"; exit 1; }

say "Compiling herdctl (release)…"
( cd "$REPO_DIR" && cargo build --release )

say "Bundling the web UI…"
( cd "$REPO_DIR/web" && npm install --silent && npm run bundle --silent )

mkdir -p "$CONFIG_DIR" "$UNIT_DIR"
touch "$ENV_FILE"; chmod 600 "$ENV_FILE"

say "Installing dev unit → $UNIT_DIR/$UNIT (runs the repo build output in place)"
sed -e "s#@REPO@#$REPO_DIR#g" -e "s#@ENVFILE@#$ENV_FILE#g" \
    "$REPO_DIR/packaging/$UNIT" > "$UNIT_DIR/$UNIT"

# Survive logout/reboot so the dev instance behaves like the real one.
command -v loginctl >/dev/null && loginctl enable-linger "$USER" 2>/dev/null || true

systemctl --user daemon-reload
systemctl --user enable "$UNIT" >/dev/null 2>&1 || true
say "Restarting $UNIT — this build is now the live instance"
systemctl --user restart "$UNIT"

sleep 1
systemctl --user --no-pager --lines=0 status "$UNIT" | head -4 || true
echo
say "Live. herdctl auto-created its config + login token under $CONFIG_DIR."
echo "  token:   grep HERDCTL_WEB_SECRET $ENV_FILE"
echo "  logs:    journalctl --user -u $UNIT -f"
echo "  config:  $CONFIG_DIR/config.json  (edit bind_addr for phone/tailnet access)"

if [[ "${1:-}" == "--logs" ]]; then
  exec journalctl --user -u "$UNIT" -f
fi

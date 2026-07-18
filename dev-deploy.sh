#!/usr/bin/env bash
set -euo pipefail
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_BASE="${XDG_CONFIG_HOME:-$HOME/.config}"
DATA_BASE="${XDG_DATA_HOME:-$HOME/.local/share}"
CONFIG_DIR="$CONFIG_BASE/herdr-go"; DATA_DIR="$DATA_BASE/herdr-go"
LEGACY_CONFIG_DIR="$CONFIG_BASE/herdr-gateway"; LEGACY_DATA_DIR="$DATA_BASE/herdr-gateway"
UNIT_DIR="$CONFIG_BASE/systemd/user"; UNIT="herdr-go-dev.service"
say() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
die() { echo "error: $*" >&2; exit 1; }
migrate_dir() {
  local legacy="$1" canonical="$2"
  if [[ -e "$canonical" ]]; then [[ ! -e "$legacy" ]] || echo "warning: both $canonical and $legacy exist; legacy left untouched" >&2
  elif [[ -e "$legacy" ]]; then mv "$legacy" "$canonical" || die "migration failed before service start"; fi
}
migrate_dir "$LEGACY_CONFIG_DIR" "$CONFIG_DIR"
migrate_dir "$LEGACY_DATA_DIR" "$DATA_DIR"
command -v cargo >/dev/null || die "cargo is required"
command -v npm >/dev/null || die "npm is required"
( cd "$REPO_DIR/web" && npm install --silent && npm run bundle --silent )
( cd "$REPO_DIR" && cargo build --release )
mkdir -p "$CONFIG_DIR" "$DATA_DIR" "$UNIT_DIR"
ENV_FILE="$CONFIG_DIR/herdctl.env"; touch "$ENV_FILE"; chmod 600 "$ENV_FILE"
sed -e "s#@REPO@#$REPO_DIR#g" -e "s#@ENVFILE@#$ENV_FILE#g" -e "s#@CONFIG_DIR@#$CONFIG_DIR#g" -e "s#@DATA_DIR@#$DATA_DIR#g" "$REPO_DIR/packaging/$UNIT" > "$UNIT_DIR/$UNIT"
for conflict in herdr-gateway.service herdr-gateway-dev.service herdr-go.service; do systemctl --user disable --now "$conflict" >/dev/null 2>&1 || true; done
rm -f "$UNIT_DIR/herdr-gateway.service" "$UNIT_DIR/herdr-gateway-dev.service"
systemctl --user daemon-reload
systemctl --user enable "$UNIT" >/dev/null
systemctl --user restart "$UNIT"
say "Live. Logs: journalctl --user -u $UNIT -f"
[[ "${1:-}" != --logs ]] || exec journalctl --user -u "$UNIT" -f

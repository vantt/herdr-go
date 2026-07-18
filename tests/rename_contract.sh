#!/usr/bin/env bash
set -euo pipefail
fail() { echo "rename contract: $*" >&2; exit 1; }
grep -q 'REPO="vantt/herdr-go"' install.sh || fail "installer repo"
grep -q 'name="herdr-go-${{ matrix.target }}"' .github/workflows/release.yml || fail "release archive producer"
grep -q 'asset="herdr-go-${TARGET}"' install.sh || fail "archive consumer"
test -f packaging/herdr-go.service && test -f packaging/herdr-go-dev.service
test ! -e packaging/herdr-gateway.service && test ! -e packaging/herdr-gateway-dev.service
grep -q 'curl -fSL.*install.sh | bash' README.md || fail "README no-clone path"
for doc in README.md docs/installation.md docs/usage.md; do
  while IFS= read -r link; do
    [[ "$link" == http* || "$link" == \#* ]] && continue
    link_path="${link%%#*}"
    [[ -e "$(dirname "$doc")/$link_path" ]] || fail "broken link $link in $doc"
  done < <(grep -oE '\]\([^)]+' "$doc" | sed 's/^](//' || true)
done
if rg -n 'herdr-gateway' AGENTS.md Cargo.toml README.md src/main.rs src/lib.rs src/doctor.rs web/package.json web/package-lock.json web/src docs/usage.md docs/deployment.md; then fail "stale current product name"; fi
grep -q 'both legacy and canonical' src/config/mod.rs || fail "both-exist warning"
grep -q 'migrate_legacy_state()?' src/main.rs || fail "startup migration"
grep -q '"bind_addr": "127.0.0.1:8787"' src/main.rs || fail "demo loopback default"
grep -q -- '--demo --bind 0.0.0.0:8787' README.md || fail "explicit demo bind override"
grep -q 'ReadWritePaths=@DATA_DIR@ @CONFIG_DIR@' packaging/herdr-go.service || fail "custom XDG unit paths"
if rg -n 'ProtectHome=read-only' install.sh packaging/herdr-go.service; then fail "service blocks user workspaces"; fi
if rg 'ProtectHome=' install.sh packaging/herdr-go.service | grep -vq 'ProtectHome=false'; then fail "unexpected ProtectHome policy"; fi
for unit in install.sh packaging/herdr-go.service; do
  grep -q 'NoNewPrivileges=true' "$unit" || fail "NoNewPrivileges missing from $unit"
  grep -q 'ProtectSystem=strict' "$unit" || fail "ProtectSystem missing from $unit"
done
grep -q 'systemd-based Linux' README.md docs/installation.md || fail "systemd platform boundary"
grep -q 'prints a token only\|Only a first install creates and prints' docs/usage.md docs/installation.md || fail "repeat token guidance"
grep -q "HERDCTL_WEB_SECRET=//p" docs/installation.md docs/usage.md || fail "token retrieval guidance"
for doc in docs/installation.md docs/usage.md; do
  grep -Fq '${XDG_CONFIG_HOME:-$HOME/.config}/herdr-go/herdctl.env' "$doc" || fail "canonical token path missing from $doc"
  grep -q 'chmod 600' "$doc" || fail "token file mode guidance missing from $doc"
  grep -q 'systemctl --user restart herdr-go.service' "$doc" || fail "token restart guidance missing from $doc"
done
systemctl_line="$(grep -n -m1 'command -v systemctl' install.sh | cut -d: -f1)"
preflight_line="$(grep -n -m1 'systemctl --user show-environment' install.sh | cut -d: -f1)"
[[ -n "$systemctl_line" && "$systemctl_line" -lt "$preflight_line" ]] || fail "systemctl check must precede user-manager probe"
assert_after_preflight() {
  local pattern="$1" label="$2" line
  line="$(grep -n -F -m1 "$pattern" install.sh | cut -d: -f1)"
  [[ -n "$line" && "$preflight_line" -lt "$line" ]] || fail "$label precedes systemd preflight"
}
# First runtime occurrence of each mutating class: state move (through the
# migration call), temporary/durable directory creation, binary/config/env/unit
# writes, removal, and service-manager mutation.
assert_after_preflight 'migrate_dir "$LEGACY_CONFIG_DIR"' "state migration"
assert_after_preflight 'tmp_dir="$(mktemp -d)"' "temporary directory creation"
assert_after_preflight 'curl -fSL' "archive write"
assert_after_preflight 'mkdir -p' "directory creation"
assert_after_preflight 'install -m 0755' "binary installation"
assert_after_preflight '> "$CONFIG_FILE"' "config write"
assert_after_preflight '> "$ENV_FILE"' "environment write"
assert_after_preflight 'systemctl --user disable --now' "service stop"
assert_after_preflight 'rm -f "$UNIT_DIR' "legacy unit removal"
assert_after_preflight 'cat > "$UNIT_DIR/$UNIT"' "unit write"
assert_after_preflight 'systemctl --user daemon-reload' "service reload"
stop_line="$(grep -n -m1 'disable --now' install.sh | cut -d: -f1)"
start_line="$(grep -n -m1 'enable "$UNIT"' install.sh | cut -d: -f1)"
[[ "$stop_line" -lt "$start_line" ]] || fail "service starts before conflict stop"
echo "rename contract: ok"

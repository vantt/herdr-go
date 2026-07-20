#!/usr/bin/env bash
set -euo pipefail
fail() { echo "rename contract: $*" >&2; exit 1; }
grep -q 'REPO="vantt/herdr-go"' install.sh || fail "installer repo"
grep -q 'name="herdr-go-${{ matrix.target }}"' .github/workflows/release.yml || fail "release archive producer"
while IFS= read -r doc; do
  if [[ "$doc" == *"*"* || "$doc" == *"?"* || "$doc" == *"["* ]]; then
    mapfile -t matches < <(compgen -G "$doc" || true)
    ((${#matches[@]} > 0)) || fail "release package documentation glob matched no files: $doc"
  else
    test -f "$doc" || fail "release package references missing documentation: $doc"
  fi
done < <(
  sed -n '/- name: Package/,/- name: Upload to release/p' .github/workflows/release.yml |
    grep -oE "docs/[^[:space:]\"']+\.md" |
    sort -u
)
grep -q 'asset="herdr-go-${TARGET}"' install.sh || fail "archive consumer"
test -f packaging/herdr-go.service && test -f packaging/herdr-go-dev.service
test ! -e packaging/herdr-gateway.service && test ! -e packaging/herdr-gateway-dev.service
grep -q 'curl -fSL.*install.sh | bash' README.md || fail "README no-clone path"
for doc in README.md docs/specs/system-overview.md docs/specs/installation.md; do
  while IFS= read -r link; do
    [[ "$link" == http* || "$link" == \#* ]] && continue
    link_path="${link%%#*}"
    [[ -e "$(dirname "$doc")/$link_path" ]] || fail "broken link $link in $doc"
  done < <(grep -oE '\]\([^)]+' "$doc" | sed 's/^](//' || true)
done
current_surface=(Cargo.toml Cargo.lock src/main.rs src/lib.rs src/config/mod.rs src/doctor/mod.rs src/doctor/checks.rs src/doctor/prompt.rs src/supervisor.rs src/notify/telegram.rs .github/workflows/ci.yml .github/workflows/release.yml install.sh dev-deploy.sh packaging/herdr-go.service packaging/herdr-go-dev.service scripts/windows-runtime-smoke.ps1 tests/observe_reply_e2e.rs README.md docs/PRD.md docs/specs/system-overview.md docs/specs/installation.md)
if rg -n -i 'herdctl|HERDCTL' "${current_surface[@]}"; then fail "retired executable identity remains active"; fi
grep -q 'both legacy and canonical' src/config/mod.rs || fail "both-exist warning"
grep -q 'migrate_default_state_if(&args, herdr_go::config::migrate_legacy_state)?' src/main.rs || fail "main-wired migration gate"
grep -q 'main_migration_seam_obeys_the_cli_mode_matrix' src/main.rs || fail "migration mode matrix"
grep -q '"bind_addr": "127.0.0.1:8787"' src/main.rs || fail "demo loopback default"
grep -q -- '--demo --bind 0.0.0.0:8787' README.md || fail "explicit demo bind override"
grep -q 'ReadWritePaths=@DATA_DIR@ @CONFIG_DIR@' packaging/herdr-go.service || fail "custom XDG unit paths"
if rg -n 'ProtectHome=read-only' install.sh packaging/herdr-go.service; then fail "service blocks user workspaces"; fi
if rg 'ProtectHome=' install.sh packaging/herdr-go.service | grep -vq 'ProtectHome=false'; then fail "unexpected ProtectHome policy"; fi
for unit in install.sh packaging/herdr-go.service; do
  grep -q 'NoNewPrivileges=true' "$unit" || fail "NoNewPrivileges missing from $unit"
  grep -q 'ProtectSystem=strict' "$unit" || fail "ProtectSystem missing from $unit"
done
grep -q 'systemd-based Linux' README.md docs/specs/installation.md || fail "systemd platform boundary"
grep -q 'HERDR_GO_WEB_SECRET=//p' docs/installation.md || fail "token retrieval guidance"
grep -Fq '${XDG_CONFIG_HOME:-$HOME/.config}/herdr-go/herdr-go.env' docs/installation.md || fail "canonical token path missing"
grep -q 'chmod 600' docs/installation.md || fail "token file mode guidance missing"
grep -q 'systemctl --user restart herdr-go.service' docs/installation.md || fail "token restart guidance missing"
systemctl_line="$(grep -n -m1 'command -v systemctl' install.sh | cut -d: -f1)"
preflight_line="$(grep -n -m1 'systemctl --user show-environment' install.sh | cut -d: -f1)"
[[ -n "$systemctl_line" && "$systemctl_line" -lt "$preflight_line" ]] || fail "systemctl check must precede user-manager probe"
assert_after_preflight() {
  local pattern="$1" label="$2" line
  # First match strictly after preflight_line, not the file's first match overall:
  # the --uninstall early-exit branch (which returns before preflight ever runs)
  # shares several of these substrings, so an unqualified -m1 would false-fail on
  # that unrelated branch instead of checking the real install-flow occurrence.
  line="$(grep -n -F "$pattern" install.sh | awk -F: -v pf="$preflight_line" '$1>pf{print $1; exit}')"
  [[ -n "$line" ]] || fail "$label precedes systemd preflight"
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

dev_preflight_line="$(grep -n -m1 'systemctl --user show-environment' dev-deploy.sh | cut -d: -f1)"
[[ -n "$dev_preflight_line" ]] || fail "dev user-manager preflight"
assert_dev_after_preflight() {
  local pattern="$1" label="$2" line
  line="$(grep -n -F "$pattern" dev-deploy.sh | tail -1 | cut -d: -f1)"
  [[ -n "$line" && "$dev_preflight_line" -lt "$line" ]] || fail "dev $label precedes preflight"
}
# Use the executed migration calls (last matches), not the earlier function body.
assert_dev_after_preflight 'migrate_dir "$LEGACY_CONFIG_DIR"' "config migration"
assert_dev_after_preflight 'migrate_dir "$LEGACY_DATA_DIR"' "data migration"
assert_dev_after_preflight 'npm install --silent' "dependency write"
assert_dev_after_preflight 'npm run bundle --silent' "web bundle"
assert_dev_after_preflight 'cargo build --release' "compile"
assert_dev_after_preflight 'mkdir -p' "directory creation"
assert_dev_after_preflight 'touch "$ENV_FILE"' "environment write"
assert_dev_after_preflight '> "$UNIT_DIR/$UNIT"' "unit write"
assert_dev_after_preflight 'systemctl --user disable --now' "service stop"
assert_dev_after_preflight 'rm -f "$UNIT_DIR' "legacy unit removal"
assert_dev_after_preflight 'systemctl --user daemon-reload' "service reload"
assert_dev_after_preflight 'systemctl --user enable' "service enable"
assert_dev_after_preflight 'systemctl --user restart' "service restart"

# A reachable-systemctl failure must leave legacy state untouched and execute no
# service mutation. Fake only the commands the preflight discovers.
failure_root="$(mktemp -d)"
trap 'rm -rf "$failure_root"' EXIT
mkdir -p "$failure_root/bin" "$failure_root/config/herdr-gateway" "$failure_root/data/herdr-gateway"
for command_name in cargo npm; do
  printf '#!/usr/bin/env bash\nexit 0\n' > "$failure_root/bin/$command_name"
  chmod +x "$failure_root/bin/$command_name"
done
printf '#!/usr/bin/env bash\nprintf "Linux\\n"\n' > "$failure_root/bin/uname"
chmod +x "$failure_root/bin/uname"
cat > "$failure_root/bin/systemctl" <<EOF
#!/usr/bin/env bash
if [[ "\$*" == "--user show-environment" ]]; then exit 1; fi
printf '%s\n' "\$*" >> "$failure_root/service-mutations"
EOF
chmod +x "$failure_root/bin/systemctl"
if HOME="$failure_root/home" XDG_CONFIG_HOME="$failure_root/config" XDG_DATA_HOME="$failure_root/data" PATH="$failure_root/bin:/usr/bin:/bin" bash dev-deploy.sh >/dev/null 2>&1; then
  fail "dev deploy accepted unreachable user manager"
fi
test -d "$failure_root/config/herdr-gateway" || fail "dev failure moved legacy config"
test -d "$failure_root/data/herdr-gateway" || fail "dev failure moved legacy data"
test ! -e "$failure_root/config/herdr-go" || fail "dev failure created canonical config"
test ! -e "$failure_root/data/herdr-go" || fail "dev failure created canonical data"
test ! -e "$failure_root/service-mutations" || fail "dev failure mutated services"

grep -q 'Develop from source' docs/advanced/source-build.md || fail "README source-build guidance"
grep -q 'source-build' docs/specs/installation.md || fail "installation source-build guidance"
if grep -q 'served as static assets alongside the binary' .github/workflows/release.yml; then fail "stale release UI comment"; fi
grep -q 'embedded into the binary at compile time' .github/workflows/release.yml || fail "embedded UI release comment"
echo "rename contract: ok"

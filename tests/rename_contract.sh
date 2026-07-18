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
    [[ -e "$(dirname "$doc")/$link" ]] || fail "broken link $link in $doc"
  done < <(grep -oE '\]\([^)]+' "$doc" | sed 's/^](//' || true)
done
if rg -n 'herdr-gateway' AGENTS.md Cargo.toml README.md src/main.rs src/lib.rs src/doctor.rs web/package.json web/package-lock.json web/src docs/usage.md docs/deployment.md; then fail "stale current product name"; fi
grep -q 'both legacy and canonical' src/config/mod.rs || fail "both-exist warning"
grep -q 'migrate_legacy_state()?' src/main.rs || fail "startup migration"
grep -q 'ReadWritePaths=@DATA_DIR@ @CONFIG_DIR@' packaging/herdr-go.service || fail "custom XDG unit paths"
stop_line="$(grep -n -m1 'disable --now' install.sh | cut -d: -f1)"
start_line="$(grep -n -m1 'enable "$UNIT"' install.sh | cut -d: -f1)"
[[ "$stop_line" -lt "$start_line" ]] || fail "service starts before conflict stop"
echo "rename contract: ok"

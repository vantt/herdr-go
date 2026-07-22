# self-update-merge-config-19

**Status:** DONE

Created `scripts/update-smoke.sh` (mirroring `scripts/macos-install-smoke.sh`) to prove the `herdr-go update` lifecycle end-to-end, and wired a new additive-only `update-smoke` CI job into `.github/workflows/release.yml` (`needs: [checksums]`, `runs-on: macos-14`).

**Files touched:**
- `scripts/update-smoke.sh` (new)
- `.github/workflows/release.yml` (additive job appended; no existing job's steps changed)

Full trace/evidence: `.bee/cells/self-update-merge-config-19.json`

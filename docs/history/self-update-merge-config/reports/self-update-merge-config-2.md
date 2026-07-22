# self-update-merge-config-2 — [DONE]

**Outcome:** Wired `scripts/generate-checksums.sh` into `.github/workflows/release.yml` so every published release asset gets a sha256 entry in a single `checksums.txt` (D8, D10).

**Approach:** Each `build` matrix leg and the `release-windows` job stage their packaged archive via `actions/upload-artifact` (`release-asset-*`). A new `checksums` job (`needs: [build, release-windows]`) downloads all staged assets into one flat dir with `download-artifact` (`merge-multiple: true`), runs the script to stdout redirected to `checksums.txt` outside the scanned dir, and publishes it via the same `softprops/action-gh-release@v2` used elsewhere. Merged single-file chosen over per-job fallback (artifact-passing was straightforward; precedent: `macos-install-smoke` already uses `needs: build`).

**Additive proof:** `git diff --stat` = 1 file changed, 50 insertions(+), 0 deletions(-) — every existing compile/package/upload step byte-identical; new job anchors on `generate-checksums.sh`, not the unrelated pre-existing Get-FileHash step (release.yml:139-152).

**Files touched:** `.github/workflows/release.yml`

**Verify:** passed (YAML parse + content-anchor greps). Full trace and verification evidence: `.bee/cells/self-update-merge-config-2.json`.

**Commit:** `6a2e9d2`

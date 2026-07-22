# self-update-merge-config-1

**Status:** DONE
**Outcome:** Created `scripts/generate-checksums.sh` — detects `sha256sum` or `shasum -a 256`, hashes only top-level `*.tar.gz`/`*.zip` files (subdirectories skipped), prints `sha256sum -c`-compatible lines to stdout only.
**Files touched:** `scripts/generate-checksums.sh`
**Full trace/evidence:** `.bee/cells/self-update-merge-config-1.json`

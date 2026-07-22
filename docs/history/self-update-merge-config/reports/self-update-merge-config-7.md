# self-update-merge-config-7

**Status:** [DONE]

**Outcome:** Added `src/update/asset.rs` with a pure `expected_asset_filename(os, arch) -> Option<String>` mirroring `install.sh`'s target-triple mapping (D1) for all 4 supported combos, plus a thin `expected_asset_filename_for_this_host()` wrapper. Wired `mod asset;` into `src/update/mod.rs`.

**Files touched:** `src/update/asset.rs` (new), `src/update/mod.rs`

Full trace/evidence: `.bee/cells/self-update-merge-config-7.json`

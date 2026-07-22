# self-update-merge-config-8 — Download release asset + checksum verify (fail-closed)

**Status:** [DONE]

**Outcome:** Extended `src/update/github.rs` with the D8/D10 checksum-verify + fail-closed download gate. `ReleasePayload` now projects `assets: Vec<ReleaseAsset { name, browser_download_url }>`. Added pure `find_asset`, pure `resolve_asset_urls` (fail-closed on missing platform asset and missing `checksums.txt` — D10), pure `verify_checksum` (fail-closed on missing entry — D10, and mismatch — D8), and async `download_and_verify` that composes `checksum::{parse_checksums, checksum_matches}` (cell -6) and `asset::expected_asset_filename_for_this_host` (cell -7), calls `error_for_status()` on both downloads, and returns `Ok(bytes)` only when every check passes. Every corruption/absence path returns a specific error — none ever yields `Ok`.

**EP5 carry-forward invariant:** recorded as a doc comment on `download_and_verify` — EP5 must source installed bytes exclusively from its `Ok(bytes)`, never a second raw-download path (defeats D8's verify-before-overwrite guarantee).

**Files touched:** `src/update/github.rs`

**Verify:** grep name-guards (6) pass && `cargo test --quiet update` → 25 passed, 0 failed. All 4 required test names (`verify_succeeds_when_checksum_matches`, `fails_closed_when_checksums_txt_asset_missing`, `fails_closed_when_entry_missing_for_asset`, `fails_when_checksum_mismatches`) pass.

Full trace, evidence, and verify output: `.bee/cells/self-update-merge-config-8.json`.

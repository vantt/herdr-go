# self-update-merge-config-6

**Status:** DONE

**Outcome:** Added `sha2` as a direct Cargo.toml dependency and created `src/update/checksum.rs` with `sha256_hex`, `parse_checksums`, `checksum_matches` (D8, D10), plus the three named unit tests. Declared `mod checksum;` in `src/update/mod.rs`. No network calls.

**Files touched:** `Cargo.toml`, `Cargo.lock`, `src/update/checksum.rs`, `src/update/mod.rs`

**Verify:** passed (`cargo test --quiet update` -> 13 passed, 0 failed, incl. `sha256_hex_matches_known_vector`, `parse_checksums_extracts_two_entries`, `checksum_matches_is_case_insensitive_and_rejects_wrong_hash`)

**Commit:** d30a537

Full trace and evidence: `.bee/cells/self-update-merge-config-6.json`

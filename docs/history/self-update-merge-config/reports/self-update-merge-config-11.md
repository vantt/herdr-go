# self-update-merge-config-11 — [DONE]

**Outcome:** Extracted `pub(crate) fn default_config_json(root: &Path) -> String` from `ensure_config` (pure refactor, byte-identical JSON for a given root, per D5); `ensure_config` now calls it. Added unit test `default_config_json_produces_expected_keys`.

**Files touched:** `src/config/mod.rs`

**Verify:** `grep -q 'fn default_config_json' && grep -q 'fn default_config_json_produces_expected_keys' && cargo test --quiet config` → 64 passed, 0 failed, 0 ignored (incl. the two pre-existing `ensure_config` tests, unchanged).

**Commit:** a841f43

Full trace/evidence: `.bee/cells/self-update-merge-config-11.json`

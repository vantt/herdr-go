# Cell dedupe-default-config-templates-1

**Status:** [DONE]

Extracted `pub(crate) fn default_config_root(home: &Path) -> PathBuf` in
`src/config/mod.rs`, next to `default_config_json`. `ensure_config` now calls
`default_config_root(&home())` instead of inlining the `projects`-fallback.
No change to `default_config_json`'s signature, output, or callers.

**Files touched:** `src/config/mod.rs`

**Trace:** `.bee/cells/dedupe-default-config-templates-1.json`

**Commit:** 3edc7db

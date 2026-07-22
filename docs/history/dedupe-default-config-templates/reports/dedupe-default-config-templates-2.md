# Cell dedupe-default-config-templates-2

**Status:** [DONE]

Added the hidden, self-exec-only `--internal-print-default-config` CLI
branch, mirroring `--internal-merge-config`'s shape. `src/main.rs` parses it
into a new `Args.internal_print_default_config: bool` and dispatches to a new
`pub fn run_internal_print_default_config() -> i32` in `src/config/mod.rs`,
which resolves the root via `default_config_root(&home())` (D2, reusing cell
-1's helper) and prints `default_config_json(&root)` verbatim to stdout. The
flag is never mentioned in `print_help()`. Added unit test
`run_internal_print_default_config_matches_default_config_json`.

**Files touched:** `src/main.rs`, `src/config/mod.rs`

**Trace:** `.bee/cells/dedupe-default-config-templates-2.json`

**Commit:** 7af35e6

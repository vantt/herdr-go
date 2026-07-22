# default-agent-presets-1

**Status:** [DONE]
**Outcome:** Seeded identical 3-entry `agent_presets` (Claude/Codex/Agy, D5/D7/D8/D9/D10) into both `ensure_config`'s default_json template (`src/config/mod.rs`) and `default_config_json` (`src/doctor/checks.rs`); added tests in both modules asserting exact parsed content; existing `ensure_config_creates_a_working_default_then_reloads` regression guard still passes unchanged.

**Files touched:** `src/config/mod.rs`, `src/doctor/checks.rs`

**Verify:** `cargo test --quiet --lib -- config:: doctor::checks:: && cargo fmt --all --check && cargo clippy --all-targets -- -D warnings` — passed.

**Commit:** ab8f2e1 (`feat(default-agent-presets-1): seed default Claude/Codex/Agy agent_presets`)

Full trace and verification evidence: `.bee/cells/default-agent-presets-1.json`

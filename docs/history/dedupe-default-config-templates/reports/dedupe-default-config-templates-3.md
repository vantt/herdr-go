# Cell dedupe-default-config-templates-3

**Status:** [DONE]

Deleted doctor's local `default_config_json` (checks.rs:898-920) and its
isolated unit test. `offer_config_fix`'s corrupt-recreate call site
(checks.rs:562) now calls
`crate::config::default_config_json(&crate::config::default_config_root(home))`
directly. Added `config_fix_recreates_unparseable_config`, a new test that
drives `offer_config_fix` through the real unparseable-JSON branch end-to-end
and asserts the recreated config contains `agent_presets` — a net gain in
coverage over the deleted isolated-function test.

**Files touched:** `src/doctor/checks.rs`

**Trace:** `.bee/cells/dedupe-default-config-templates-3.json`

**Commit:** 6504f9b

# Cell dedupe-default-config-templates-6: config.example.json agent_presets

**Status:** [DONE]

**Outcome:** Added agent_presets array to config.example.json, matching the three canonical entries (Claude, Codex, Agy) from config::default_config_json per D7, with placeholder values preserved.

**Files touched:**
- `config.example.json` — added agent_presets array (6 lines inserted)

**Verification:** Python JSON validation passed. agent_presets array present with exactly 3 entries, herdr_session remains "gateway" (unchanged placeholder).

**Full trace:** See `.bee/cells/dedupe-default-config-templates-6.json`

**Commit:** 29bb4e3 — "Add agent_presets to config.example.json (cell dedupe-default-config-templates-6)"

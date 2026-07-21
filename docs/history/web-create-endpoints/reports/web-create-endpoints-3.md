# web-create-endpoints-3

**Status:** [DONE]

**Outcome:** Added `GET /api/create-options`: destinations from `snapshot.workspaces` (every workspace, including the agentless one `/api/agents` structurally drops) with `path`/`path_is_live` from cell 1's `anchor_for_workspace`; presets from `AppState.agent_presets` (label only — `argv` never serialized). Widened `AppState` with an `agent_presets` field attached via a new `with_agent_presets` builder method, so `AppState::new`'s 3-caller signature (including `tests/observe_reply_e2e.rs`, out of this cell's file scope) stayed unchanged. Route registered behind `AuthSession` like every other route; a snapshot error returns 502, mirroring `api::agents`.

**Files touched:**
- `src/web/mod.rs` — `AppState.agent_presets` field + `with_agent_presets` builder; route registered in the table
- `src/web/api.rs` — `Destination`/`PresetOption`/`CreateOptions` types + `create_options` handler; 7 new `createoptions_` tests
- `src/main.rs` — `AppState::new(...).with_agent_presets(config.agent_presets.clone())` at the one in-scope call site

**Verify:** `VERIFY_PASS` — `cargo test --quiet` 238+2+3 passed; `cargo fmt --all --check` clean; `cargo clippy --all-targets -- -D warnings` 0 warnings; `cargo test --lib -- createoptions_` 7 passed.

**Full trace/evidence:** `.bee/cells/web-create-endpoints-3.json`

# web-create-endpoints-4

**[DONE]** — Added the two create routes: `POST /api/panes` (shell) and `POST /api/agents` (agent), registered in the web route table, with the asymmetric unresolved-path branch (shell omits `cwd`; agent refuses with 409).

## Files
- `src/web/create.rs` (new) — both handlers, preset-label lookup, and the `HerdrError` → HTTP mapping.
- `src/web/mod.rs` — `pub mod create;` and the two routes (`/api/agents` gains `.post`, new `/api/panes`).

## Verify
`cargo test --quiet` + `cargo fmt --all --check` + `cargo clippy --all-targets -- -D warnings` + `cargo test --lib -- createroute_` (9 passed). Green.

Full trace, must-haves, and verification evidence: `.bee/cells/web-create-endpoints-4.json`.

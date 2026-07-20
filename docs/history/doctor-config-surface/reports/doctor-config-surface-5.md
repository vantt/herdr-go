# doctor-config-surface-5 — [DONE]

**Outcome:** Added the web-token guided fix (phase 2) and the end-of-run settings editor (all 8 config fields + 3 env-only secrets), with D6/D9/D16/D17 honored and no secret ever printed.

**Files touched:**
- `src/doctor/edit.rs` (new) — settings editor: menu built from `write::CONFIG_FIELDS` + 3 secrets + stop; config edits via `write::persist_validated`; `allowed_roots` reuses `checks::prompt_new_allowed_root`; `bind_addr`→non-loopback fires the D16 warning at edit time; secrets via `prompt::prompt_secret` + `config::secrets::write_secret`.
- `src/doctor/mod.rs` — `mod edit;`, phase-4 single end-of-run editor prompt (interactive-only), and shared `pub fn non_loopback_bind_warning`.
- `src/doctor/checks.rs` — `web token` guided fix calling `config::ensure_web_secret()` (real path, value never echoed); `field_json_value` / `persist_and_report` promoted to `pub(super)` for reuse.
- `src/main.rs` — startup non-loopback warning now calls the shared helper (byte-identical output).

**Verification:** `cargo test --quiet && cargo clippy --quiet -- -D warnings && bash tests/rename_contract.sh` — green (152 lib + integration tests, clippy clean, `rename contract: ok`). Test-first: 12 new tests captured red (`not yet implemented`) before implementation.

Full trace, evidence, and deviations: [.bee/cells/doctor-config-surface-5.json](../../../../.bee/cells/doctor-config-surface-5.json)

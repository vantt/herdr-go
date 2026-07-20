# doctor-config-surface-3 — Secret writer + startup env-file fallback

**Status:** [DONE] — verify green, capped, committed `a43de3f`.

**Outcome:** New `src/config/secrets.rs` implements the D10 replace-not-append
`herdr-go.env` writer (temp-file + atomic rename, owner-only via the reused
`prepare_token_directory`/`write_new_token`/`validate_token_protection`, never
`write_new_token` against the live path) and the D8 env-then-file secret
resolution (process env wins; unprotected file ignored with a non-fatal
diagnostic that names no value; absent file is not an error). `Secrets` now has
a redacting `Debug` impl in place of the derive.

**Files touched:**
- `src/config/secrets.rs` (new)
- `src/config/mod.rs` (`pub mod secrets;`, redacting `Debug` for `Secrets`,
  `from_env` delegates to the fallback resolver, `secrets_read_from_env_only`
  test replaced by the deterministic `secrets_absent_from_env_and_file_are_none`)

**Verify:** `cargo test --quiet && cargo clippy --quiet -- -D warnings` — 132+2+3+0
tests ok, clippy exit 0.

Full trace, verification evidence, and red-failure record:
[.bee/cells/doctor-config-surface-3.json](../../../../.bee/cells/doctor-config-surface-3.json)

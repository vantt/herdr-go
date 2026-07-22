# self-update-merge-config-18 — Wire the public `update` CLI verb

**Status:** [DONE]
**Worker:** worker-ep6-a · **Lane:** high-risk · **Commit:** 63fe53c

## Outcome

Added the feature's final integration point — the real, public `herdr-go update` command that ties every already-shipped piece together:

- `src/update/mod.rs`: new `pub async fn run(config_path: &std::path::Path) -> i32` composing `github::check_for_update(crate::VERSION)` → `github::download_and_verify()` → `rollout::perform_update(...)` in order, with fail-closed stderr reporting and a nonzero exit at each step (D8/D10). Loads an existing config via `crate::config::Config::load_file` and, unlike normal startup, never creates a fresh default. Uses only `crate::`/submodule paths — no `herdr_go::` self-references.
- `src/main.rs`: `Args` gains an `update: bool`; `parse_args` handles the bare `update` verb (mirroring `doctor`); `main()` dispatches early (after `service`, before secrets/config wiring), resolving the config path via `default_config_path` and exiting on `herdr_go::update::run`; `print_help` documents the public command; the `main_migration_seam_obeys_the_cli_mode_matrix` test literal gains `update: false`.

## Verification

Cell verify chain passed **EXIT=0** (run under `bash` — the verify uses the bash `<(...)` process-substitution idiom, which is broken in the session's default zsh shell): all 5 grep guards pass, `cargo build` clean, `cargo test` 312+2+3 passed / 0 failed, `--help` documents `update`.

Full trace, evidence, and deviation: `.bee/cells/self-update-merge-config-18.json`.

## Deviation

One auto-fix: reworded a pre-existing doc comment in `src/update/mod.rs` from `` `herdr_go::VERSION` `` to `` `crate::VERSION` `` — the verify guard `! grep -q 'herdr_go::'` was tripping on prose, and `crate::VERSION` is the correct crate-internal path regardless. No behavior change.

## Parent next action

Cell capped, committed, reservations released. Collect `[DONE]` and continue the EP6 wave (cell -19 smoke test remains).

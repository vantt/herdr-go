# doctor-config-surface-4 — report

**Status:** [DONE]

**Outcome:** Wired D14's three-phase interactive doctor loop (diagnose → offer-fix → single recheck), the `--check` flag with precedence over TTY detection, a `skipped` check discriminant with non-blocking exit-code semantics, and guided fixes for the `config` and `allowed roots` check identities (matched by label across both failure-shaped states). The `allowed_roots` breadth guard (D9) is factored into a reusable `prompt_new_allowed_root` for the next cell.

**Files touched:**
- `src/doctor/checks.rs` — `Check.skipped` field + `Check::skipped` constructor; socket-resolution early return removed (checks 4/5 become skipped); `offer_fixes`/`offer_config_fix`/`offer_allowed_roots_fix`/`apply_field_repairs`/`prompt_new_allowed_root`/`field_json_value`/`default_config_json`.
- `src/doctor/mod.rs` — three-phase `run(check_only)`, `all_ok` exit predicate (skipped exempt), `print_report`/`render_report` split rendering the skipped marker.
- `src/main.rs` — `--check` flag on `Args` + parser + help; doctor dispatch passes `args.check`; migration-matrix test literal updated.
- `src/config/mod.rs` — `home()` exposed `pub(crate)` for the breadth guard.

**Verify:** `cargo test --quiet && cargo clippy --quiet -- -D warnings && bash tests/rename_contract.sh` — chained exit 0 (140+2+3 tests pass, clippy clean, rename contract ok).

Full trace, behavior-change evidence, and decisions honored: `.bee/cells/doctor-config-surface-4.json`.

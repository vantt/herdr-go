# doctor-config-surface-1

[DONE]

Split `src/doctor.rs` into a `src/doctor/` module (`mod.rs`, `checks.rs`, `prompt.rs`) with zero observable behavior change, and added unused-until-Slice-2 TTY detection and prompt primitives per D13/D15.

Files touched: `src/doctor.rs` (removed, content moved), `src/doctor/mod.rs`, `src/doctor/checks.rs`, `src/doctor/prompt.rs`, `Cargo.toml`, `Cargo.lock`, `tests/rename_contract.sh`.

Full trace/evidence: `.bee/cells/doctor-config-surface-1.json`.

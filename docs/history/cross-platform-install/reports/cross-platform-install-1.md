# cross-platform-install-1

**Status:** [DONE]

**Outcome:** Added a `#[cfg(target_os = "macos")]` arm to `base_config_dir()`/`base_data_dir()` in `src/config/mod.rs`, resolving both to `$HOME/Library/Application Support` (joined with `herdr-go` by `config_dir()`/`data_dir()`), per D1. Windows and non-macOS-Unix (XDG) arms are byte-unchanged. Added a macOS-gated unit test and re-scoped the pre-existing `data_dir_defaults_to_home_local_share` test off macOS (its XDG assertion would otherwise be false there post-change).

**Files touched:** `src/config/mod.rs`

**Verify:** `cargo test --quiet && cargo clippy --quiet -- -D warnings` — passed. Full trace and evidence: `.bee/cells/cross-platform-install-1.json`.

**Commit:** `5b8d30e`

**Friction:** The `cap` command emitted an advisory — `JUDGE_STANDARD_INSUFFICIENT: ... the D3 red_failure_evidence floor (>=80 chars, non-duplicate) was not enforced for this cap (F5)` — despite `red_failure_evidence` being present and well over 80 chars in the submitted evidence. This reads as a gap in the cap tool's own floor-enforcement logic (validation unclear/too expensive trigger), not a defect in the evidence supplied. The cap still recorded successfully with the full evidence object intact.

**Note:** The new macOS-gated test cannot execute on this Linux worker (no macOS runner available). Real-machine proof on `macos-14` is deferred to this feature's Slice 1 real-CI proof step per `plan.md`, not this cell's verify.

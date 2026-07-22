# self-update-merge-config-16 — report

**Status:** [DONE]

**Outcome:** Added `src/update/rollout.rs` implementing the corrected EP5 rollout — `perform_update` composes stop → swap running binary → self-exec `--internal-merge-config` under the new binary → start → poll `/api/health` (10×1s) → rollback binary+config+restart on failure; two pure helpers (`health_check_ok`, `health_check_url`) unit-tested. Declared via `pub mod rollout;` in `src/update/mod.rs`.

**Files touched:**
- `src/update/rollout.rs` (new)
- `src/update/mod.rs` (`pub mod rollout;`)

**Verify:** `cargo test --quiet update` → 29 passed / 0 failed (incl. the 2 new pure tests); all 8 grep/existence gates pass including `! grep -q 'herdr_go::'`; clippy + fmt clean on both files. Exit 0.

**Commit:** 2ee603e

**Friction:** Pre-existing `cargo fmt` drift in `src/update/swap.rs` (cell-15's committed file) at HEAD — out of this cell's writable scope, left untouched; the wave-close full `cargo fmt --all --check` will flag it until cell-15's file is reformatted.

Full trace / verification evidence: `.bee/cells/self-update-merge-config-16.json`.

# Cell: self-update-merge-config-17

**Status:** [DONE]

**Outcome:** Applied `cargo fmt --all` to src/update/swap.rs; verification passed (format check + update tests)

**Files Changed:**
- src/update/swap.rs

**Trace:** See `.bee/cells/self-update-merge-config-17.json` for full details.

## Summary

Purely mechanical formatting fix. The cell applied rustfmt formatting via `cargo fmt --all` to src/update/swap.rs (written by prior cell self-update-merge-config-15, which had rustfmt violations caught by the wave-close verify chain).

**Verification:** 
- `cargo fmt --all --check` — passed (no remaining format violations)
- `cargo test --quiet update` — 29 tests passed

No logic or test behavior changes — formatting only.

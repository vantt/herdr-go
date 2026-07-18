---
artifact_contract: bee-plan/v1
artifact_readiness: implementation-ready
mode: tiny
---

# Release v0.1.0

## Discovery

The first release has no existing tag. `Cargo.toml` and `web/package.json` already declare `0.1.0`, so no version-file change is needed. GitHub Actions showed that release commit `40a6dc3` fails only `cargo fmt --all --check` in two Rust files.

## Mode fit

Tiny: one mechanical formatting correction across two files, with no API or data change. The external release happens only after the corrected commit passes local verification and CI.

## Approach

Run the repository formatter on `src/doctor.rs` and `src/herdr/socket.rs`, verify formatting and the full project suite, commit and push the correction, require green CI, then create and push tag `v0.1.0`. Do not include unrelated dirty-worktree files.

## Current slice

- Files: `src/doctor.rs`, `src/herdr/socket.rs`
- Required outcome: `cargo fmt --all --check` passes with no behavioral change.
- Full verification: `cargo test --quiet && cargo clippy --quiet -- -D warnings && cd web && npm run bundle && npm run test -- --run`
- Release proof: main-branch CI succeeds before `v0.1.0` is pushed.

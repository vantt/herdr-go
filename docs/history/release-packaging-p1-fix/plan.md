---
artifact_contract: bee-plan/v1
artifact_readiness: implementation-ready
mode: tiny
---

# Plan: release-packaging-p1-fix

## Scope

Fix the open review P1 from `review-new-changesets-20260719`: the release
workflow package step copies `docs/usage.md` and `docs/deployment.md`, but both
files were deleted by the current docs consolidation. A tag release would fail
before uploading any asset.

## Mode

Tiny. One direct task, two files maximum, one public release-packaging contract
already identified by review. No product decision is open.

Risk flags: public contracts, existing covered behavior.

## Approach

- Update `.github/workflows/release.yml` so the release package copies only
  existing documentation files.
- Strengthen `tests/rename_contract.sh` with a focused check that every docs
  file referenced by the release package copy command exists.

## Reality Check

- Review evidence identifies `.github/workflows/release.yml:85` as the failing
  line.
- `docs/usage.md` and `docs/deployment.md` are absent in the current tree.
- Existing `tests/rename_contract.sh` already owns release/install naming
  contracts, so adding the missing-path guard there is the smallest proof.

## Verify

- `bash tests/rename_contract.sh`
- `cargo test --quiet && cargo clippy --quiet -- -D warnings && cd web && npm run bundle && npm run test -- --run`

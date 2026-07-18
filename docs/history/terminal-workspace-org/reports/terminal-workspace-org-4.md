# Cell Report: terminal-workspace-org-4

**Status:** `[DONE]`

**Outcome:** Added fixture workspaces (frontend-app, docs-site) and tabs (main) to FakeHerdr::new() snapshot for --demo mode grouping parity.

**Files touched:** `src/herdr/fake.rs`

**Verification:** `cargo test --quiet && cargo clippy --quiet -- -D warnings` — passed (83 tests, 0 failures, clippy clean with -D warnings)

**Commit:** `2ac5ef3` feat(terminal-workspace-org-4): add workspace/tab fixture data to FakeHerdr for demo-mode grouping parity

**Cell trace & evidence:** See `.bee/cells/terminal-workspace-org-4.json` for full verification evidence and fixture details.

## Summary

Added workspace and tab fixture data to the Snapshot constructed by `FakeHerdr::new()` to match the existing agent seed data and support the workspace grouping feature (D4). Before: `workspaces` and `tabs` were left as `Default::default()` (empty vecs), causing GET /api/agents to return empty `workspace_label` and unknown `workspace_status` in --demo mode. After: seeded workspace fixtures for w1 (frontend-app, Working) and w2 (docs-site, Done) with matching tab fixtures (main). All 7 FakeHerdr-scoped tests pass unchanged; full suite passes (83 tests).

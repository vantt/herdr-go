# terminal-workspace-org-2

[DONE] Added `workspace_status` rollup: `Workspace.agent_status` parsed in `src/herdr/wire.rs`, resolved per-agent via new `Snapshot::workspace_status_for` (join-miss falls back to `AgentStatus::Unknown`), threaded onto `AgentRow` in `src/web/api.rs` and the TS `AgentRow` in `web/src/api.ts`.

Files touched: `src/herdr/wire.rs`, `src/web/api.rs`, `web/src/api.ts`.

Verify: `cargo test --quiet && cargo clippy --quiet -- -D warnings && cd web && npm run typecheck` — all green (83+3 Rust tests, clippy clean, tsc clean).

Full trace/evidence: `.bee/cells/terminal-workspace-org-2.json`.

Commit: `e257d17`.

No deviations, no Advisor Consults, no outstanding questions.

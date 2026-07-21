# home-shell-workspaces-1

**Status:** [DONE]

**Outcome:** Widened `GET /api/agents` from a bare `Vec<AgentRow>` to
`{agents, shells}`. `ShellRow` surfaces one row per plain-shell pane
belonging to a workspace with zero agents (D3), reading `path` directly off
that pane (`foreground_cwd ?? cwd`), never via `anchor_for_workspace`.
Existing `AgentRow` output is unchanged. `src/herdr/fake.rs`'s `w3` seed
gained a second shell pane (`w3:p7`) to prove multi-shell-per-workspace
granularity, without touching `w3`'s existing `agent_status`. 4 new
`homeshell_`-prefixed unit tests added; the pre-existing
`agents_lists_flat_snapshot` and `observe_reply_e2e.rs` tests updated to
parse the new response shape.

**Files touched:** `src/web/api.rs`, `src/herdr/wire.rs`,
`src/herdr/fake.rs`, `tests/observe_reply_e2e.rs`

**Full trace/evidence:** `.bee/cells/home-shell-workspaces-1.json`

**Commit:** `42fd6d2`

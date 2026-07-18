# terminal-workspace-org-1

**Status:** [DONE]

**Outcome:** Parsed `workspaces[]`/`tabs[]` from `session.snapshot` (both `src/herdr/wire.rs` types and the real `src/herdr/socket.rs` extraction path) and threaded `workspace_label`/`tab_label` through `AgentRow` (Rust + TS), resolved by joining on `workspace_id`/`tab_id` with empty-string fallback on a join miss. Additive only — no existing field renamed/removed.

**Files touched:**
- `src/herdr/wire.rs` — new `Workspace`/`Tab` types, `Snapshot.workspaces`/`.tabs` fields, `workspace_label_for`/`tab_label_for` resolution methods, 2 new tests
- `src/web/api.rs` — `AgentRow` gained `workspace_label`/`tab_label`, populated from the wire.rs resolution; existing test extended to assert both fields are present
- `web/src/api.ts` — TS `AgentRow` mirrors the two new string fields
- `src/herdr/socket.rs` (deviation, auto-added) — the real socket path only extracted `agents` from the snapshot response; without this fix `workspaces`/`tabs` would always resolve empty against a live herdr, silently defeating the cell's outcome
- `src/herdr/fake.rs` (deviation, auto-fixed) — compile fix for the new `Snapshot` fields (`..Default::default()`)

**Deviations:** two, both within the auto-fix/auto-add rules (blocking compile issue in fake.rs; missing critical functionality in socket.rs that the cell's outcome directly depends on). No architectural changes.

**Verify:** `cargo test --quiet && cargo clippy --quiet -- -D warnings && cd web && npm run typecheck` — all green (82 Rust tests + 3 doctest-adjacent passed, clippy clean, tsc clean). Full trace and evidence: `.bee/cells/terminal-workspace-org-1.json`.

**Commit:** `86bd286` — `feat(terminal-workspace-org-1): thread workspace_label/tab_label through AgentRow`

**Reservations:** all 5 released.

## Outstanding Questions

None.

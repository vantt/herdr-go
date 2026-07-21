# new-shell-new-agent-2 — [DONE]

Implemented the D10 anchor join as a pure, I/O-free function on `Snapshot`, and
proved it against both hand-written degrade fixtures and the tracked live
capture.

## Files touched

- `src/herdr/wire.rs` — added `Snapshot::anchor_cwd_for_workspace(&self, workspace_id: &str) -> Option<String>`.
  The join: `workspace_id` -> `Workspace.active_tab_id` -> the `layouts[]` entry
  matching **both** `workspace_id` and `tab_id` -> its `focused_pane_id` -> that
  pane in the snapshot's top-level `panes[]` -> `foreground_cwd.or(cwd)`. Any
  miss at any hop returns `None`; the function never uses the snapshot's
  top-level `focused_*_id` fields or a pane's own `focused` flag (D10), and
  never touches a layout's inner geometry `panes[]`.

## Tests added (11, all `anchor_`-prefixed, all passing)

- `anchor_resolves_for_globally_focused_workspace`, `anchor_resolves_for_non_focused_workspace`
- `anchor_prefers_foreground_cwd_over_cwd`, `anchor_uses_cwd_when_foreground_cwd_absent`
- `anchor_none_when_pane_missing_both_cwds`
- `anchor_none_when_layouts_absent`, `anchor_none_when_active_tab_has_no_layout_entry`
- `anchor_ignores_layout_matched_only_by_tab_id` (workspace_id-mismatch case)
- `anchor_none_when_workspace_id_unknown`
- `anchor_resolves_shell_pane_absent_from_agents`
- `anchor_live_envelope_matches_probe` — drives `src/herdr/testdata/live-snapshot.json`
  via the existing `include_str!` constants and asserts all 5 workspaces resolve
  to the `expected_cwd` values in `src/herdr/testdata/expected-anchors.json`.

Every row of the slice 1 test matrix in `docs/history/new-shell-new-agent/plan.md`
is covered, including both named degrade paths and the workspace_id-mismatch case.

## Notes

- No deviations from the cell's `action`/`must_haves`/`prohibitions`.
- `cargo fmt --all` reformatted one line in the new function (collapsed a
  multi-line `.find()` chain); re-verified clean after.

Full trace, verify command and recorded output: `.bee/cells/new-shell-new-agent-2.json`.

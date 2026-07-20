# new-shell-new-agent-1 — [DONE]

Parsed the snapshot surface the D10 anchor join needs across all three herdr
population sites, and extracted a pure `parse_snapshot` seam so the live socket
path is provable against a captured envelope instead of only compiling.

## Files touched

- `src/herdr/wire.rs` — new `Pane` and `PaneLayout` types; `panes[]`, `layouts[]`
  and the three top-level `focused_*_id` fields on `Snapshot`; `active_tab_id` on
  `Workspace`; `cwd`/`foreground_cwd` on `Pane`. All new fields `#[serde(default)]`.
- `src/herdr/socket.rs` — extracted `fn parse_snapshot(&Value) -> Result<Snapshot>`
  from the inline body of `snapshot()`; it takes the outer result value, so it *is*
  the live extraction path rather than a copy of it. `panes[]`/`layouts[]` join
  `agents[]` as hard `Malformed` errors; `workspaces[]`/`tabs[]` keep their
  best-effort fallback.
- `src/herdr/fake.rs` — seeded `panes[]`, `layouts[]`, `active_tab_id` and global
  focus, with `w2:p5` a plain shell absent from `agents[]` and serving as w2's anchor.
- `src/herdr/testdata/live-snapshot.json`, `src/herdr/testdata/expected-anchors.json`
  — tracked test data, copied verbatim from `.bee/spikes/new-shell-new-agent/`
  (that directory is gitignored, so tests read the tracked copies via `include_str!`).

## Notes

- Six `envelope_`-prefixed tests added (verify requires at least 4).
- `PaneLayout` deliberately omits `splits[]`/`area`/inner geometry `panes[]`: the
  split ratio is a float and would break `Snapshot`'s `Eq` derive.
- The D10 join itself is not implemented here — that is cell 2. The wire test
  asserts only that every hop the join needs is present in the parsed data, for
  all five captured workspaces including the four that are not globally focused.

Full trace, verify command and recorded output: `.bee/cells/new-shell-new-agent-1.json`.

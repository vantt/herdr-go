# web-create-endpoints-1

[DONE] — added `Snapshot::anchor_for_workspace` (returns `AnchorCwd { path, live }`) as the provenance-carrying sibling of `anchor_cwd_for_workspace`, which now delegates to it unchanged in signature/behavior. 8 new `provenance_`-prefixed tests cover live-true, live-false, all four join-miss cases, a non-focused workspace, and delegation equivalence.

Files touched: `src/herdr/wire.rs`

Full trace/evidence: `.bee/cells/web-create-endpoints-1.json`

Commit: `9e0b842`

# Validation report — terminal-scrollback-agent-panes, final (round 5)

**Verdict: READY.**

## History

- **Round 1** (plan.md v1, cells v1): NOT READY. 3 BLOCKER, 3 CRITICAL, 2
  WARNING — see `validation-terminal-scrollback-agent-panes-round1.md`.
  Missing files in cell 1 (`RecordingHerdr`, `screen.rs`), D11 orphaned,
  cell 3 verify unrunnable/unproving, `FakeHerdr` untestable, and a real
  false-escalation logic bug (naive `recent`-vs-approximated-`visible`
  compare escalates unnecessarily for short primary-screen panes).
- **Round 2** (plan.md v2, cell 1 v2): NOT READY. Round 1's fix (an
  "at-capacity" gate) introduced 2 new blockers — see
  `validation-terminal-scrollback-agent-panes-round2.md`. No `source`
  selector was ever wired into `SocketHerdr` (still hardcoded `"recent"`),
  and the capacity gate needed viewport-row data the gateway doesn't have
  anywhere and can't obtain without the exact stale-field cross-reference
  D3 already rejected.
- **Round 3** (plan.md v3, cell 1 v3): fixed both by adding a real
  `ReadSource` parameter and dropping the capacity gate (accepted as a
  low-cost trade-off, reasoned in plan.md's "Revision Note (round 3)").
  Review found one BLOCKER (cell 1's `must_haves` truths 5/6 contradicted
  each other — leftover wording from the dropped gate) plus 3 minor nits.
- **Round 4** (cell 1 v4, cell 2 v2, plan.md risk-map wording): fixed the
  contradiction and all 3 nits (named `lines: 80` explicitly, unambiguous
  "not richer" branch, `src/web/mod.rs` added to cell 2's `files`, and the
  "escape sequences are harmless in a shell" claim reframed as an explicit
  unverified assumption rather than an asserted fact). Review verdict:
  **READY WITH CONSTRAINTS** — one remaining gap: `PaneScroller::read_history`
  had no way to supply cell 2 with a `revision` value.
- **Round 5** (this round, cell 1 v5, cell 2 v3): `read_history` now returns
  the existing `ScreenRead{text, revision}` wire type directly instead of a
  bare `Vec<String>`/`String`, carrying revision through from whichever
  underlying read was used; cell 2 builds `ScreenBody` from it the same way
  the existing non-history path already does. Self-checked for internal
  consistency (files list, truths/prohibitions counts) against the fix.

## Final state

- `node .bee/bin/bee.mjs cells schedule --feature terminal-scrollback-agent-panes`:
  clean 3-wave schedule, no cycles (cell 1 → cell 2 → cell 3).
- All 8 round-1 findings, both round-2 findings, and the round-3/4
  contradiction + nits are fixed with cited evidence at each step, not
  asserted.
- One accepted, explicitly-labeled assumption remains in plan.md's Risk Map
  (short primary-screen panes get one harmless-but-unverified escalation) —
  flagged as an assumption for a future report to confirm or contradict, not
  a blocker.

## Gate 3

`gate_bypass_level: full` (recorded at session start) covers every lane,
including this `standard` lane, with no hard-gate flag present. Per the
bypass contract, Gate 3 is self-approved rather than asked, with this report
as the evidence trail and the decision log carrying the audit line.

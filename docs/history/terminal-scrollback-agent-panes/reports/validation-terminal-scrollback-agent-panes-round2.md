# Validation report — terminal-scrollback-agent-panes, round 2

**Verdict: NOT READY.** Round 1's 8 findings re-checked against actual
current repo/cell state (not trusted from the revision note alone) and
confirmed fixed: BLOCKER 1 (missing files), BLOCKER 2 (D11 ownership),
BLOCKER 3/CRITICAL 4 (verify now runs — `Test Files 1 passed (1) / Tests 10
passed (10)`), CRITICAL 5 (FakeHerdr extension specified), WARNING 7/8
(clamp ownership, citation) all FIXED.

CRITICAL 6 (false-escalation bug) was **partially fixed, with 2 new
BLOCKERs** introduced by the fix itself:

- **NEW-A:** the mechanism calls for requesting herdr's real `source:
  "visible"`, but nothing in round 2's plan/cells actually added a `source`
  selector anywhere — `SocketHerdr::read_pane` still hardcodes
  `"source":"recent"` (`src/herdr/socket.rs:466`) unconditionally. Step 1 of
  the mechanism was unimplementable as written.
- **NEW-B:** the "at-capacity" gate (only escalate if the `visible` read
  fills the full viewport) needs the pane's viewport row count. The
  gateway's own `Pane` wire type carries no such field
  (`src/herdr/wire.rs:132-143`); the only place row counts exist is herdr's
  own snapshot (`panes[].scroll.viewport_rows`, confirmed only in
  `src/herdr/testdata/live-snapshot.json`) — cross-referencing that is
  exactly the stale/ambiguous field-based signal D3 already rejected.

## Disposition

Round 3 fixes NEW-A by adding a real `source` parameter to `Herdr::read_pane`
(kept — genuine, needed fix) and fixes NEW-B by **dropping the at-capacity
gate entirely** rather than inventing a way to obtain data the gateway
doesn't have: the problem it solved (a short primary-screen pane escalating
once, harmlessly, into a plain shell that ignores an unbound escape
sequence) is a low-cost accepted trade-off, not a correctness bug — see
`plan.md`'s "Revision Note (round 3)" for the full reasoning and why this is
not a scope reduction of D3.

# Validation: home-shell-workspaces

## Reality Gate

| Check | Result | Evidence |
|---|---|---|
| MODE FIT | PASS | 2 risk flags (public contract change to `GET /api/agents`, multi-domain backend+frontend); story-sized, 6+ product files across both languages, exceeds `small`'s cap. `standard` correctly chosen. |
| REPO FIT | PASS | Every referenced type/join/pattern confirmed by direct read: `Pane{pane_id,workspace_id,tab_id,cwd,foreground_cwd}` (`src/herdr/wire.rs`), `workspace_label_for`/`tab_label_for` taking `&Agent` (`wire.rs:210-236`), `w3`'s single-shell fixture (`src/herdr/fake.rs:80-86`), `NewPaneRef`/widened `Route` already shipped (`web/src/main.ts:14-24`). |
| ASSUMPTIONS | PASS | Backend response shape (widen vs new endpoint) resolved in planning's Approach with a cited rejected alternative. `AgentRow.workspace` vs `ShellRow.workspace_id` field-name mismatch identified and called out explicitly in the cell, not left implicit. |
| SMALLER PATH | PASS | File count alone (6+) exceeds `small`; no smaller split indicated once the TS whole-project-typecheck coupling was understood (see below). |
| PROOF SURFACE | PASS (after fix) | Cell 1's verify command initially had a real defect (grep required ≥3 tests, action never said so) — fixed. Both cells' verify commands proven runnable and self-consistent after the plan-checker's second pass. |

## Feasibility Matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| Backend response shape change doesn't break other consumers | MEDIUM | Grep every `GET /api/agents` caller | Plan-checker found `tests/observe_reply_e2e.rs:46-56` parses the response as a bare array and asserts `len()==4` — a real breakage, not hypothetical. `src/web/auth.rs:142,205` only assert status codes, unaffected. | PASS (after fix — test added to cell 1's scope) |
| Cell 2/3 split is independently cappable | HIGH (structural) | Confirm each cell's own verify can pass in isolation | Both the plan-checker and the cold-pickup cell reviewer independently found the same deadlock: `web/tsconfig.json`'s whole-project `include` couples `fetchAgents`'s signature to its sole consumer (`switcher.ts:120-125`, `renderList(rows: AgentRow[])`) — cell 2 could never pass its own `tsc` without editing a file it was prohibited from touching. | PASS (after fix — cells 2+3 merged into one deliverable) |
| Schedule has no cycles, matches plan's stated pipeline | LOW | `cells schedule --json` | Run before and after the merge: `[[1],[2]]` (was `[[1],[2],[3]]`), zero cycles both times. | PASS |
| D7 (group-header hide) actually reaches a cell, not lost in translation | LOW | Direct text match against CONTEXT.md | Plan-checker confirmed cell 2's (formerly cell 3's) action carries D7 verbatim plus a matching truth. | PASS |

## Plan-Checker (2 iterations, both logged)

**Iteration 1 — REJECT:** 2 BLOCKERs (cell 1's verify would fail on an out-of-scope e2e test; cell 2/3's split created a verify-level dependency deadlock under TypeScript's whole-project typecheck), 2 WARNINGs (cell 1's undocumented ≥3-test threshold; cell 1 over-listing `wire.rs` with no implied edit). The cold-pickup cell reviewer independently found the same cell-2/3 deadlock as its 1 CRITICAL, plus a MINOR: cell 3's action falsely claimed agent and shell rows "both carry a workspace_id" when the real field names differ (`AgentRow.workspace` vs `ShellRow.workspace_id`).

**Fixes applied:** cell 1 gained `tests/observe_reply_e2e.rs` in its file scope, an explicit instruction to update that test's parsing, and an explicit "≥3 `homeshell_`-prefixed tests" requirement; `wire.rs`'s presence was justified by the id-keyed-helper-widening option already in the action. Cell 3 was dropped (with a recorded reason) and its full scope merged into cell 2, which now spans both the data-layer and rendering files as one deliverable — resolving the typecheck coupling structurally rather than papering over it — and its action explicitly states the `workspace` vs `workspace_id` field-name mismatch so a worker doesn't assume parity.

**Iteration 2 — READY:** all findings confirmed resolved; schedule now a clean 2-cell chain; no new issue introduced by the merge (cell 2 verified as one coherent deliverable, not scope overload).

## Approval Block

**Decision:** READY

**Mode:** standard, 2 risk flags (public contract change, multi-domain), no hard-gate flag. `gate_bypass_level=full` covers Gate 3 at every lane — no human question, no advisor consult (advisor consult is a mechanical precondition for `high-risk`/hard-gate work only).

**Scope of this approval:** both cells (`home-shell-workspaces-1`, `-2`). This is the feature's only planned slice — no Phase 2 exists.

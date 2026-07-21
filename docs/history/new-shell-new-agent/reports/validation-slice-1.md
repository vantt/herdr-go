# Validation — new-shell-new-agent, slice 1

**Lane:** high-risk · **Cells:** `new-shell-new-agent-1`, `new-shell-new-agent-2` · **Date:** 2026-07-20

**Verdict: READY**

---

## Reality Gate

| Check | Result | Evidence |
|---|---|---|
| MODE FIT | PASS | 5 risk flags counted in `plan.md:13-21`. Slice 1 alone touches 3 product files; `small` caps at 3 with no gray areas, and the feature spans ~12 files across two languages. |
| REPO FIT | PASS | Every cited path exists and holds what the cells claim. `Snapshot` at `wire.rs:112-120` keeps only `agents`/`workspaces`/`tabs`; the join-miss fallback pattern at `wire.rs:133-151`; the fixture precedent at `wire.rs:247-266`; `socket.rs:228-254` hand-extracts and builds the struct literal at `:250`; `fake.rs:64` constructs `Snapshot` directly; `critical-patterns.md:20` is the three-population-sites rule. |
| ASSUMPTIONS | PASS | The one blocking assumption — that the D10 join resolves for arbitrary workspaces — was proven against a live herdr rather than assumed. See Spike. |
| SMALLER PATH | PASS | Checked and rejected by the persona panel: parsing (3 files) then a pure join (1 file, sequential dep) is the honest seam; neither a merge nor a three-way split improves it. |
| PROOF SURFACE | PASS | Both verify commands run, and both exit non-zero on the untouched tree — proven empirically. Baseline is green on every other segment (see below). |

**Baseline (re-run at validation time):** `cargo test --quiet` → 0 (156 + 2 + 3 passing) · `cargo fmt --all --check` → 0 · `cargo clippy --all-targets -- -D warnings` → 0 · `tests/rename_contract.sh` → green · `npm run bundle` + `npm run test -- --run` → 20 passing.

Known and unchanged: `tests/rename_contract.sh` contains three `rg`-gated checks that silently no-op on this box because `rg` has no real binary here (`critical-patterns.md`). Treated as unverified locally, not as passing.

**Schedule:** `cells schedule` reports 2 waves, no cycles — `new-shell-new-agent-1` then `new-shell-new-agent-2`.

---

## Spike — does the D10 anchor join hold against live data?

**Question:** does the join resolve a real pane with a usable cwd for *every* workspace in a live snapshot, or only for hand-written fixtures?

**Answer: YES.** Evidence: `.bee/spikes/new-shell-new-agent/{anchor-join-probe.mjs,findings.json,live-snapshot.json,expected-anchors.json}`, against herdr 0.7.4 / protocol 16.

| | |
|---|---|
| Workspaces | 5 |
| Anchors resolved | 5 / 5 |
| **Not globally focused** | **4 / 4 resolved** |
| Resolved via `foreground_cwd` | 5 / 5 (never the `cwd` fallback) |
| Anchor that is a plain shell absent from `agents[]` | 1 |

Three constraints this recorded for execution:

1. The **non-focused** case — the one the exploring-phase review proved broken under the naive design — resolves correctly on real data.
2. `panes[]` is genuinely required, not precautionary: the globally focused workspace's own anchor is a plain shell that never appears in `agents[]`.
3. D5's `foreground_cwd`-before-`cwd` precedence is what fires in practice, on every workspace, not a rare fallback.

### Constraint discovered for a later slice

Two distinct workspaces (`w5`, `wB`) came back with the **identical label `forgent` and the identical cwd `/home/dev/projects/forgent`**. D3 renders the destination as one row of label + path, so slice 5 would show two rows a user cannot tell apart, and picking the wrong one starts an agent in the wrong workspace. This does not disturb D3 — the destination is still one combined choice — but slice 5 must add a disambiguator; `WorkspaceInfo.number` is wire-visible and is the obvious candidate. Logged as a decision so slice 5 does not rediscover it while building the sheet.

---

## Feasibility Matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| The D10 join resolves for non-focused workspaces | HIGH | Live probe, not fixtures | 4/4 non-focused resolved against herdr 0.7.4 | **PROVEN** |
| `panes[]` is required, not substitutable by `agents[]` | HIGH | A real anchor absent from `agents[]` | `wB:p1`, `in_agents_array: false` | **PROVEN** |
| Adding fields to `Snapshot` cannot silently leave them empty at the type layer | MEDIUM | No `..Default::default()` or serde hole at any construction site | `grep -rn "Default::default()" src/` → nothing; the four literals at `socket.rs:250`, `fake.rs:64`, `wire.rs:270`, `wire.rs:288` all fail to compile without the new fields | **PROVEN** |
| …but the **production** path can still ship empty arrays | MEDIUM | — | `socket.rs:228-254` hand-extracts rather than deserializing; `panes: Vec::new()` would compile and pass fixtures. Closed by requiring an extracted `parse_snapshot` seam with a test over the real envelope | **CLOSED BY CELL REPAIR** |
| Modelling `layouts[]` fully is safe | MEDIUM | Check `Snapshot`'s derives against the schema | `Snapshot` derives `Eq` (`wire.rs:113`); `PaneLayoutSplit.ratio` is a float. Closed by scoping `PaneLayout` to three fields | **CLOSED BY CELL REPAIR** |
| Both verify commands can distinguish done from not-done | MEDIUM | Run them on the untouched tree | Both exit 1; segment isolation confirms the failure is the final count-grep, not `test`/`fmt`/`clippy` | **PROVEN** |
| A cell may declare files it will create | LOW | Inspect cap-time validation | `.bee/bin/lib/cells.mjs` has no filesystem-existence check on `files`; `frozenJudgeHits` (`:1053-1064`) only flags changes *outside* declared scope | **PROVEN** |
| Tracked fixture path is not gitignored | LOW | `git check-ignore` | `src/herdr/testdata/` → no match; `.gitignore:10` covers only `.bee/spikes/` | **PROVEN** |

---

## Plan-Checker — persona panel (high-risk lane)

Lenses: coherence, feasibility, scope-guardian. **Verdict: BLOCKERS PRESENT** → repaired → clean at iteration 3.

| # | Lens | Finding | Disposition |
|---|---|---|---|
| B1 | feasibility | `verify` was `cargo test --quiet 2>&1 \| tail -20` — a pipeline's status is `tail`'s, so it could never report red. Same class as the `grep` inverted-exit-code entry in `critical-patterns.md`. Also the only `\| tail` in any cell in the repo. | FIXED |
| B2 | coherence + feasibility | No must_have bound `socket.rs`'s hand-built extraction. `panes: Vec::new()` would compile, satisfy every fixture test, and ship empty arrays against live herdr forever — verbatim the 2026-07-18 incident. `socket.rs` has no tests over `snapshot()` because it does I/O inline. | FIXED — cell 1 now requires extracting a pure `parse_snapshot` seam and testing it |
| B3 | coherence | The live spike evidence was not routed into the cells, so the worker would hand-write fixtures from the schema — the same person authoring fixture and join, making any misreading reproduce identically in both and pass green. Circular proof on the component the plan itself calls HIGH. | FIXED — the raw envelope is now committed test data and both cells consume it |
| W4 | feasibility | "non-empty `panes[]`/`layouts[]`" was too weak for `FakeHerdr`: a layout whose `focused_pane_id` matches nothing satisfies it while resolving nothing downstream. | FIXED — the truth is now a join assertion |
| W5 | feasibility | Cell verify was narrower than `.bee/config.json`'s recorded verify — `fmt` and `clippy` dropped, and fmt drift already has its own remediation cell in this repo. | FIXED |
| W6 | coherence | Absence policy undecided: `socket.rs` holds two contradictory precedents in one function (missing `agents[]` is a hard error at `:232-235`; `workspaces`/`tabs` silently empty at `:240-249`). | FIXED — policy stated explicitly |
| W7 | feasibility | `layouts[].panes` name-collides with the top-level `panes[]`; resolving against the wrong one compiles, matches, and yields no cwd. | FIXED — called out in both cells |
| W8 | scope | No truth required matching a layout on `workspace_id` **and** `tab_id`; because herdr's tab ids are globally unique, `tab_id`-alone would pass every truth. The "globally focused" matrix row was also not a truth. | FIXED |
| W9 | scope | Cell 1's escape hatch ("if `socket.rs` turns out not to need a change, say so") softened the exact rule `critical-patterns.md:20` exists to enforce. Both files provably need changes. | FIXED — hatch deleted |
| W10 | process | Gate 2 was auto-approved under `full` bypass with open questions outstanding. | NOTED — question 1 is now answered by the spike; the rest belong to slices 2-4 and do not block slice 1 |

## Cell Review — cold pickup

**Iteration 1: NEEDS REPAIR (both cells).** Independently found B1 (verify), plus: `Snapshot`'s `Eq` derive versus the float `ratio`; unfalsifiable `fake.rs`/`socket.rs` truths; cell 2's return type unresolved (title said "pane", truths implied a path); a mis-anchored citation (`workspace.rs:163` is about pane numbers, not tab numbers — the same mis-anchor was in `CONTEXT.md` D10 and was corrected there too).

**Iteration 2: three new BLOCKERs, all introduced by the iteration-1 repairs.**

| # | Finding | Disposition |
|---|---|---|
| A | The fixture the cells now mandated lives under `.bee/spikes/`, which `.gitignore:10` excludes — absent in CI and every other clone, so `include_str!` would be a compile error, and no cell's `files` allowed committing a tracked copy. The worker would have fallen back to a hand-written fixture, reopening B3. | FIXED — cell 1 now creates and commits `src/herdr/testdata/{live-snapshot,expected-anchors}.json`, both declared in its `files` |
| B | `findings.json` was written *before* the home-path neutralization, so its expected cwds carried `/home/vantt` while the fixture carried `/home/dev`. Cell 2's truth was literally unsatisfiable, and the natural "fix" would have been to hand-edit expectations from the snapshot — circular proof again. | FIXED — `expected-anchors.json` is now *derived from the fixture itself*, so the two cannot drift |
| C | `parse_snapshot` takes the outer `{"snapshot": …}` value, but the fixture is the inner object; done literally the test returns `Malformed("snapshot missing")` and the likely repair desyncs the seam from the real call path. | FIXED — cell 1 states the nesting and pins the wrapping call form |
| D | Cells used `clippy --quiet` while CI uses `--all-targets`; since nearly all new code lands in `#[cfg(test)]` modules, the cell gate would pass and CI would fail. | FIXED — verified green at baseline with `--all-targets` |

**Iteration 3: STRUCTURALLY CLEAN.** An independent replay of the D10 join over the fixture matched `expected-anchors.json` exactly on all 5 workspaces and all 8 fields. Two residual WARNINGs — cell text still pointing at the stale `findings.json` and at the gitignored spike path — were closed rather than accepted, since both would have misled a worker on first read.

### Accepted with a note

The verify's `<prefix>_` test count is a **floor, not evidence**: four empty `#[test] fn envelope_stub_n() {}` bodies would satisfy `≥4`. It proves new named tests exist and pass, never that they assert anything. At cap time the `must_haves` truths must be walked individually; a green verify alone is not substantive proof for these two cells.

---

## Approval Block

- Reality gate: **PASS** (5/5)
- Feasibility matrix: **8 rows, 0 unproven**
- Spike: **YES**, with one constraint recorded for slice 5
- Plan-checker: 3 BLOCKERs + 7 WARNINGs → all fixed
- Cell review: 5 CRITICALs → fixed; 3 second-order BLOCKERs → fixed; iteration 3 clean
- Structural iterations used: **3 of 3**
- Advisor: **not configured** (`resolveAdvisor` → none); recorded per AO2(b) and proceeded

**Open questions carried forward (none block slice 1):**

1. Windows destination display — `foreground_cwd` is unix-only and PowerShell's `cwd` needs shell integration. Belongs to slice 4, where the path first becomes something a person reads. Must not silently show a wrong folder.
2. `HerdrError` shape — one variant per herdr error code, or one `Request { code, message }` carrying it? Slice 2 decides; slice 4's HTTP mapping depends on the answer.
3. Preset `argv` validation strictness — config-load failure, or a preset that renders disabled? Slice 3.
4. Duplicate workspace labels with identical paths — slice 5 needs a disambiguator (`WorkspaceInfo.number`).

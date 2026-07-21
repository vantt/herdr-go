---
date: 2026-07-21
feature: new-shell-new-agent
categories: [verify-commands, proof-integrity, review-process, tiering]
severity: high
tags: [pipefail, rg-absent, clippy-drift, circular-proof, fixture-provenance, repair-churn]
---

# Verify commands that cannot fail, and proof that only proves itself

Slice 1 of `new-shell-new-agent` shipped two cells in minutes each. Getting those
two cells fit to dispatch took three review iterations. Almost none of that cost
was spent on the hard part.

## What Happened

**1. A verify command that could never report red.** Both cells were authored with
`verify: cargo test --quiet 2>&1 | tail -20`. A pipeline's exit status is its last
command's, and `tail` succeeds whenever it can read its input — so the command
returns success no matter what `cargo test` did. Two independent reviewers found it
in the same round.

**2. The same verify also passed before the work existed.** Even repaired to an
`&&` chain, `cargo test && cargo fmt --all --check && cargo clippy …` is green on an
untouched tree. It cannot distinguish "done" from "not started", so a cell could be
capped on a verify that proved only that the suite was already green.

**3. The first repair introduced three new blockers.** Binding the cells to a real
captured fixture produced: a fixture path under a gitignored directory (absent in CI
and every other clone), an expectations file generated before a later neutralization
pass and therefore disagreeing with the fixture it described, and a parse function
specified against the outer response envelope while the fixture was the bare inner
object. Each was a *new* artifact the repair created; nothing re-derived the
neighbouring facts that artifact then had to agree with.

**4. A documented gotcha recurred anyway, in a feature nobody was looking at.**
`critical-patterns.md` already recorded that `rg` has no real binary reachable from
a non-interactive subprocess here. `.bee/cells/windows-support-5.json` nevertheless
carries a verify built from five `rg` invocations, two of them negated. Re-run in a
real shell today it exits 127 — it cannot pass — yet the cell is capped with
`verify_passed: true`. The recorded evidence for that cell is not reproducible.

**5. The project's own standing verify is weaker than its CI.** `.bee/config.json`
records `cargo clippy --quiet -- -D warnings`; every CI job runs
`cargo clippy --all-targets -- -D warnings`. Since `--all-targets` is what compiles
`#[cfg(test)]` code, and test modules are where most new code in a test-first cell
lands, 48 of this repo's 81 cells were gated by a strictly weaker check than the one
that will actually block the merge.

## Root Cause

**The verify failures share one defect:** the exit code of the last thing in the
command is trusted without asking what that thing actually asserts. `tail` asserts
nothing. A missing `rg` asserts nothing, and under `!` it asserts the opposite of
nothing. `clippy` without `--all-targets` asserts less than the reader assumes. In
each case the command *looks* like a gate and behaves like a no-op.

**The repair churn has a different root:** a repair was reviewed against the finding
it named, not against the constraints the rest of the cell already asserted. A repair
is a new artifact, not a patch to be read through.

**And the recurrence has a third:** the `rg` gotcha was documented in prose and
recurred anyway. It is not alone — this repo's own learnings record two other
prose-documented patterns (pinned agent types, multi-session `--lane`) that each
recurred after being written down. Three for three. Prose in `critical-patterns.md`
taxes every session preamble and only works if it is read at the moment of the
mistake, which is exactly when an agent is thinking about something else.

The `rg` entry also shows a subtler failure: it recorded the constraint as
*"`sudo apt-get` needs a password this session can't supply, so it can't be fixed at
the tool level."* That accepted the wrong constraint. The fix was never to install
`rg` — it was to stop depending on it. `grep -E` does the same job and exists
everywhere.

## Recommendation

**R1. When a verify command contains a pipe, read the segment after the last `|`
and ask what its exit code encodes.** `tail`, `head`, `cat`, and formatters encode
nothing. A positive `grep -qE '<success pattern>'` is a legitimate final segment —
it matches only on a real success line, so a crashed or failed tool produces no
match and the pipeline fails correctly. Anything else, use `&&`.

**R2. Never put `rg` in a verify command or a repo script on this machine.** It has
no binary reachable from a non-interactive subprocess. Under `!` a missing `rg`
inverts to success and the negative assertion silently passes. Use `grep -E`.
`tests/rename_contract.sh:30,37,38` are open instances today; so is
`.bee/cells/windows-support-5.json`.

**R3. A verify that is green on an untouched tree cannot cap a cell.** Before
dispatching, run the verify on the current tree; it must fail. Pair the standard
`&&` chain with a count gate over a declared test-name prefix
(`… && cargo test --lib -- <prefix>_ 2>&1 | grep -qE 'test result: ok\. (N…) passed'`)
and state the prefix in the cell so the worker can satisfy it. Note the limit
honestly: the count proves named tests exist and pass, never that they assert
anything, so walk the `must_haves` truths individually at cap time.

**R4. Re-run the whole feasibility check after every repair round, not only the
field the finding named.** Specifically: `git check-ignore` every path the cell
declares or references, confirm any derived-expectations file is generated from the
artifact it describes rather than alongside it, and execute the exact call shape the
cell specifies once before capping.

**R5. A fixture comment claiming live provenance must be backed by a tracked
capture.** `include_str!` of a `testdata/` file with stated provenance is evidence;
a hand-typed literal under a comment reading "captured live from …" is a proof gap
wearing evidence's clothes. `src/herdr/wire.rs:286,303,333` are open instances.

**R6. Derived expectations stop drift, not circularity — do not confuse them.**
Generating `expected-anchors.json` from the fixture guarantees the two agree
forever. It does not make them independent: the derivation script transcribes the
same algorithm description the implementation does, so a shared conceptual
misreading reproduces in both and passes. The independence in this slice came from
elsewhere — the fixture is a real capture rather than schema-derived, and a live
probe in another language resolved every workspace before any of the implementation
existed. Name which of the three you actually have.

**R7. Tier on the residual risk at dispatch, and record why.** A plan's risk label
describes the problem; the tier describes what judgment is left when the worker
opens the cell. Cell 2 here implemented the feature's single HIGH-rated claim at
`generation`, correctly — its design risk had been retired by a locked decision, a
live spike, and a cell text that pinned the signature, the join order and both
degrade paths. That reasoning was reconstructable from the artifacts but recorded
nowhere, which reads as an unexplained mismatch. Log it next to the tier choice.

**R8. When a gotcha recurs after being documented, the lesson is not "document it
harder".** Prose has failed three times in this repo. Promote to an executable
check, and when the recorded workaround says a fix is impossible, re-examine the
constraint it accepted before believing it.

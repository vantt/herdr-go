# Validation report: windows-release-matrix-1

## Reality gate

| Check | Verdict | Evidence |
|---|---|---|
| MODE FIT | PASS | 2 risk flags (cross-platform, existing-covered-behavior) — `standard` is correct per the mechanical table |
| REPO FIT | PASS | `.github/workflows/ci.yml`'s `windows-server-2022` job and `scripts/windows-runtime-smoke.ps1` exist and are read verbatim as the source pattern |
| ASSUMPTIONS | PASS | Reusing the proof pattern within one job's own steps is structurally identical GitHub Actions syntax to how ci.yml already runs it — no spike needed |
| SMALLER PATH | PASS | Already assessed at tiny/small threshold (0-1 flags) vs actual 2 flags; standard is the honest floor, not inflated |
| PROOF SURFACE | PASS | Cell `verify` is a real, runnable Python/YAML structural check (see below) |

## Feasibility matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| A separate `release-windows` job can stand alone without anything from `build` | LOW | Inspect workflow-level `permissions`/`concurrency`/secrets | `permissions: contents: write` is workflow-level (release.yml:10-11); no `concurrency` block; no environment/secrets in the file; `softprops/action-gh-release@v2` uses the implicit `GITHUB_TOKEN` | PASS |
| The structure-aware `verify` command discriminates before/after correctly | LOW | Run it against the current (unmodified) file | Manually run: fails with `AssertionError: no release-windows job found`, exit 1 — correct pre-execution failure, no false-pass | PASS |
| A worker mirroring ci.yml verbatim satisfies `verify`'s assertions | LOW | Cross-check ci.yml's actual step content against the assertions | ci.yml:87 uses `Get-FileHash ... SHA256`, ci.yml:60 pins `runs-on: windows-2022` — satisfies the `Get-FileHash/sha256` and `windows-2022` asserts | PASS |

Schedule (single cell, no deps): trivial, one node, no cycles.

## Plan-checker (iteration 1 — original design, `bee-review`/opus)

Found 2 CRITICAL + 5 WARNING against the original "add a Windows entry to `build`'s matrix" design:
- **CRITICAL:** `build` is one job with a single shared `steps:` list across all matrix entries; adding Windows there would force `if:`-guarding the shared Build/Package steps, which the cell's own prohibition then forbade editing — a self-contradiction a cold worker cannot resolve.
- **CRITICAL:** the original `verify` used a positional `grep -B2 -A40` window keyed off proximity to `x86_64-pc-windows-msvc` tokens — provably false-passable or false-failable depending on YAML line ordering.
- WARNINGs: plan/cell verify divergence (`-A5` vs `-B2 -A40`), `.zip`/`.tar.gz` hedge in plan vs decisive `.zip` in cell, decision `edbb7a4e`'s content absent from plan text (ID-only reference), full smoke reused inline lengthens every tagged release (accepted trade-off per decision b8c3d4bc), truths 2-4 only provable by a real Windows CI run (accepted — this cell's `verify` proves structure, not runtime; runtime proof is `ci.yml`'s own job, already separately proven).

**Redesign:** Windows moved to its own top-level `release-windows` job (not a `build` matrix entry) — `build` is never touched at all, eliminating the structural contradiction. `verify` rewritten as a structure-aware YAML/Python check scoped to the `release-windows` job in isolation, plus an assertion that `build`'s own text carries no Windows content. Plan.md's Requirements/Approach/Shape sections updated to match; `edbb7a4e`'s content added explicitly; `.tar.gz` hedge removed (decisively `.zip`).

## Plan-checker (iteration 2 — confirm the fix, `bee-review`/opus)

- (a) Re-read `action`/`prohibitions` line by line: every `build`-job reference is either "mirror this pattern" or an explicit "do not touch" prohibition; the upload step is `release-windows`'s own, with its own `ASSET` env var. **RESOLVED.**
- (b) Re-ran the exact stored `verify` string against the current file: fails with `AssertionError: no release-windows job found`, exit 1 — matches the manual test exactly, no drift. **RESOLVED.**
- (c) No new structural gap: workflow-level `permissions` covers all jobs; no `concurrency`/secrets exist anywhere in the file to be missing; the existing `build` matrix already runs 3 parallel entries each uploading independently — a 4th independent job is the established pattern, no `needs:` required. **NO NEW ISSUE.**

Overall: **READY** (2 iterations, within the 3-iteration budget).

## Cell review (cold pickup)

Covered inline by plan-checker iterations above (same dispatch, Task B in iteration 1): iteration 1 returned NOT READY (2 CRITICAL, listed above); both fixed and confirmed RESOLVED in iteration 2. No open CRITICAL or MINOR flags remain.

## Decision

**READY.** `windows-release-matrix-1` is approved for execution.

## Approval block

- Lane: `standard`. No hard-gate flag (auth/authorization/data-loss/audit-security/external-provider/validation-removal) present — CI/release infrastructure change only, reusing an already-proven download+checksum pattern verbatim.
- Gate bypass level: `full` — auto-approves Gate 3 at every lane including high-risk/hard-gate. This lane doesn't even need that reach (`normal` would already cover a non-hard-gate standard lane), so no advisor consult is required (AO2b applies only to high-risk/hard-gate slices).
- `approved_gates.execution` set via `bee.mjs state gate --name execution --approved true`.

# Validation Report — health-fingerprint, slice 1 (Phase 1, cell health-fingerprint-1)

**Date:** 2026-07-22
**Feature:** health-fingerprint
**Lane:** standard

## Reality Gate

| Check | Result | Evidence |
|---|---|---|
| MODE FIT | PASS | 2 risk flags counted (public contracts: `/api/health`'s `Health.version` value semantics change; cross-platform: `build.rs` must compute git sha/dirty/local-offset reliably on Linux/macOS/Windows) — correctly lands in `standard` (2-3 flags), not `small` (0-1) or `high-risk` (4+/hard-gate). No auth/data-loss/security/external-provider flag present. |
| REPO FIT | PASS | `build.rs`, `src/lib.rs` (`VERSION` const, line 20), `src/main.rs` (lines 113, 294), `src/web/mod.rs` (line 56), `src/web/api.rs` (`Health` struct, lines 171-186), `web/src/views/switcher.ts` (line 323) all confirmed to exist exactly as CONTEXT.md/plan.md describe. `git` binary present (`/usr/bin/git`, v2.34.1). `time = "0.3"` already a runtime dependency in `Cargo.toml` (features `formatting`, `macros`). |
| ASSUMPTIONS | PASS | The one genuinely unproven assumption — `time` crate's local-offset detection actually works on a real build host, in this format — was spiked (below) and returned YES. Dirty-state detection (`git status --porcelain`/`diff-index`) and no-git fallback (`std::process::Command` returning `Err`) rely on well-documented, deterministic behavior; no spike needed for those. |
| SMALLER PATH | PASS | No smaller viable path identified — the chosen approach already avoids new heavy dependencies (rejected `vergen`, `git2`; reuses the existing `time` crate and the existing `env!()`/`concat!()`/`build.rs` pattern already in the codebase). |
| PROOF SURFACE | PASS | Baseline `cargo test --quiet` is green today (271+2+3 tests, 0 failed) in this fresh worktree. The cell's `verify` command is confirmed to be a genuine red-before-green check: `grep -n 'rerun-if-changed' build.rs` currently matches (so the negated form in `verify` currently fails), and `cargo run --quiet -- --help` currently prints `herdr-go 0.1.2 — herdr-go` with no timestamp (so the ISO8601-offset grep in `verify` currently fails too). Both will flip to pass only once the cell's fix lands. |

## Spike: does `time`'s local-offset detection work on this build host?

**Question:** Can `time::OffsetDateTime::now_local()` (feature `local-offset`) reliably produce a local-timezone-offset timestamp on this dev host, in the format D1 requires?

**Location:** `.bee/spikes/health-fingerprint/` (disposable `Cargo.toml` + `main.rs`, never imported by production code).

**Result:** YES. `cargo run --quiet` printed:
```
OK 2026-07-22T11:53:56+07:00
```
— exactly D1's local-offset-with-colon-free-hour format. One deprecation warning surfaced (`time::format_description::parse` is deprecated in favor of `parse_borrowed`) — folded into the cell's action text as a concrete implementation note, since `commands.verify`'s `cargo clippy -- -D warnings` would otherwise fail the standing verify bar on this warning.

**Constraint discovered, recorded for execution:** use `parse_borrowed`, not `parse`.

## Feasibility Matrix

| Assumption | Risk | Proof Required | Evidence | Result |
|---|---|---|---|---|
| `time` local-offset works on a real build host | MEDIUM | Spike | `.bee/spikes/health-fingerprint/` run, `OK 2026-07-22T11:53:56+07:00` | PROVEN |
| `build.rs` currently under-reruns (scoped to `static/` only) | HIGH | File inspection | `build.rs:7`, `println!("cargo:rerun-if-changed=static")` confirmed present; Cargo's documented semantics (any `rerun-if-changed` directive disables the default always-rerun) | PROVEN — fix is to remove the line, already folded into the cell |
| `git` available at build time | LOW | Command | `which git` → `/usr/bin/git`, `git --version` → 2.34.1 | PROVEN (this host; also true of any environment where this checkout itself was obtained via git) |
| `/api/health`'s `version` field has no strict-semver consumer | LOW | Grep | `grep -rn "health\.version\|\.version\b" web/src --include="*.ts"` → only `switcher.ts:323`, opaque display | PROVEN |
| Windows/macOS build-host parity for git+time local-offset | MEDIUM (residual) | Not provable on this Linux-only worktree | — | ACCEPTED RESIDUAL RISK, per plan.md's explicit Out-of-scope note — not a blocker |
| Baseline suite is green before this cell starts | — | Command | `cargo test --quiet` → 271+2+3 passed, 0 failed | PROVEN |

Schedule: single cell, no deps, no cross-cell wave — `bee cells list --feature health-fingerprint` shows exactly one open cell (`health-fingerprint-1`); no schedule/cycle check needed for a 1-cell slice.

## Plan-Checker and Cell Review

Dispatched in parallel (review tier, `bee-review`, background): `plan-checker-health-fingerprint` (5-dimension adversarial structural check) and `cell-reviewer-health-fingerprint` (cold-pickup CRITICAL/MINOR check).

**Plan-checker verdict: structurally clean, 0 BLOCKERs.** All 5 dimensions PASS: requirement/decision coverage (D1-D6 all present in cell action/must_haves, nothing dropped/contradicted), cell completeness (rerun-if-changed removal, `-dirty`, no-git fallback all cell-enforced), dependency correctness (sole cell, `deps: []`, nothing depends on unbuilt prior work), key links (`grep -rn '\bVERSION\b' src` returns exactly the 5 claimed references, all consume `&'static str`, so the const branch's zero-call-site-edit claim holds), scope sanity (`files` matches what `action` touches). Verify command independently re-confirmed red-before-green: `build.rs:7` still has the restrictive `rerun-if-changed` line (current fail) and `--help`'s banner is bare `0.1.2` (current fail on the timestamp regex) — both flip to pass only after the fix. One non-blocking WARNING (cell's own `verify` doesn't run `clippy`, but the session's standing `commands.verify` does, and `time`'s `parse` is confirmed `#[deprecated(since="0.3.37")]` while `parse_borrowed` needs the `::<2>` turbofish) — already mitigated below.

**Cell-reviewer verdict: cold-pickup ready, 0 CRITICAL, 1 MINOR.** `files`/`read_first`/D3/D5 all concrete enough for a zero-context worker; feasibility genuinely spike-proven; `verify` runnable and meaningful. MINOR: `parse_borrowed` takes a required const-generic version param (`parse_borrowed::<2>(...)`, verified against `time` 0.3.54 source) — the action's original "use parse_borrowed" note omitted the turbofish, which would have hit a compile error.

**Resolution:** the MINOR finding was folded into `health-fingerprint-1`'s `action` text immediately (`node .bee/bin/bee.mjs cells update`), before Gate 3 — the action now explicitly says `parse_borrowed::<2>(...)` and spells out why a bare `parse_borrowed(...)` fails to compile. No cell content remains unresolved.

## Decision

**READY.** Reality gate: 5/5 PASS. Feasibility matrix: every row proven or explicitly accepted as residual (Windows/macOS parity, scoped in plan.md's Out of scope). Plan-checker: 0 blockers. Cell review: 0 CRITICAL, 1 MINOR (resolved same-turn). Single cell, no schedule/cycle risk. Proceeding to Gate 3.

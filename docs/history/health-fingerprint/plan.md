---
artifact_contract: bee-plan/v1
mode: standard
approved_gate2: 2026-07-22
---

# Plan: Build Fingerprint (health-fingerprint)

Mode: `standard` — 2 risk flags: public contracts (`/api/health`'s `Health.version` value semantics change, though field name/type stay put), cross-platform (`build.rs` must compute git sha + dirty state + local-offset timestamp reliably on Linux/macOS/Windows build hosts).
Why this is the least workflow that protects the work: 2 flags puts it above `small`'s 0–1 threshold, but nothing here is auth/data-loss/security/external-provider shaped — no need for `high-risk`'s persona panel or mandatory feasibility spike.

## Requirements (from CONTEXT.md)

- D1: Fingerprint format `<semver> (<short-sha>[-dirty], <local build timestamp with UTC offset>)`, e.g. `0.1.2 (a1b2c3d, 2026-07-22T10:15:04+07:00)`. Local machine timezone, not UTC.
- D2: `Health.version` keeps its name/type (`&'static str`), carries the full composed string. No new JSON fields.
- D3: `-dirty` suffix on the sha segment when the build's git working tree has uncommitted changes.
- D4: Timestamp is build time, not commit time.
- D5: No-git-available fallback (e.g. tarball without `.git`) must never fail the build; sha segment falls back to a placeholder.
- D6: Exactly one shared source computes the fingerprint; all display sites read from it, none duplicate the logic. Fresh-eyes review + CONTEXT.md addendum: `D2`'s `&'static str` lock favors implementing D6 as a compile-time `const` (extends the existing `env!()`/`concat!()` pattern) rather than a runtime `fn` — the zero-call-site-edit path only holds on that branch.

## Discovery

L1 — quick verify. No existing git-sha/build-fingerprint embedding precedent anywhere in this repo (`grep -rn "git rev-parse\|GITHUB_SHA\|rev-parse\|git_hash\|git_sha"` across `.github`, `Cargo.toml`, `src`, `build.rs`, `scripts` returned nothing) — this is new territory, but a well-established Rust pattern the existing `build.rs`/`VERSION` already half-implements (compile-time `env!()` sourced from a `cargo:rustc-env=...` build-script directive is exactly how `CARGO_PKG_VERSION` already flows into `VERSION`).

**Critical finding, changes the approach:** the existing `build.rs` ends with `println!("cargo:rerun-if-changed=static")`. Per Cargo's documented build-script semantics, emitting *any* `rerun-if-changed` instruction switches the script from Cargo's default "rerun on every build" behavior to "rerun only when a listed path changes." Today that already means `build.rs` only re-executes when `static/`'s mtime changes — so if we add fingerprint computation to this same script without addressing that line, a `cargo build`/`cargo run` triggered by a plain `.rs` source edit (no `static/` change) would silently reuse the **stale, previously-cached** fingerprint. That is the exact PBI-034 failure mode this feature exists to fix, reproduced inside its own fix. Confirmed by re-reading `build.rs`'s current single directive; not present in any test today (nothing exercises rebuild-without-static-change).

## Approach

**Recommended path (per D1–D6):**
1. Remove `build.rs`'s `cargo:rerun-if-changed=static` line entirely, reverting the whole script to Cargo's default "run on every build" behavior. `std::fs::create_dir_all("static")` is idempotent and cheap, so paying its cost on every build is not a real regression — and it's the only way to guarantee D3/D4 (dirty flag, build timestamp) are never cached stale, per the discovery finding above.
2. In `build.rs`, shell out to the `git` binary (`std::process::Command`) for the short sha (`git rev-parse --short HEAD`) and dirty state (`git status --porcelain` or `git diff-index --quiet HEAD` exit code); on any failure (no `.git`, `git` missing — D5), fall back to a placeholder instead of failing the build.
3. Add `time` (already a runtime dependency, so its version/behavior is already trusted) as a **build-dependency** too, and use its local-offset support to format the build timestamp with the machine's local UTC offset (D1, D4) — avoids shelling out to `date`, which has no reliable equivalent on plain Windows without Git Bash/WSL (this repo already treats native Windows as a first-class target per `cross-platform-install`).
4. Emit the composed fingerprint via `cargo:rustc-env=HERDR_GO_FINGERPRINT=...` and change `src/lib.rs`'s `VERSION` constant to read it via `env!()`/`concat!()` (D6, const branch) instead of `env!("CARGO_PKG_VERSION")` alone. All 3 existing display sites keep reading `herdr_go::VERSION` unchanged.

**Rejected alternatives:**
- Shell out to `date` for the timestamp — rejected: no portable `date` binary on plain Windows `cmd`/PowerShell without Git Bash/WSL; repo already has painful precedent with cross-platform build/install tooling (`docs/history/learnings/critical-patterns.md`, cross-platform-install entries).
- Add the `vergen` crate (purpose-built for exactly this) — rejected: introduces a new build-dependency for a small convenience win; `build.rs` today has zero build-dependencies and the repo leans YAGNI/KISS on dependency count.
- Add `git2` (libgit2 bindings) instead of shelling to the `git` binary — rejected: heavier build-dependency (links a C library) for no behavioral gain; shelling to `git` is simpler and `git` is already required to have this checkout in the first place.

**Risk map:**

| Component | Risk | Reason | Proof needed |
|---|---|---|---|
| `build.rs` rerun trigger | HIGH (until fixed) | Existing `rerun-if-changed=static` scopes ALL reruns to `static/` changes only — left as-is, the new fingerprint goes stale on the very first source-only rebuild | Remove the line; validating proof: two consecutive builds with only a `.rs` edit between them, confirm the printed/embedded fingerprint's timestamp changes both times |
| Cross-platform git/time invocation | MEDIUM | `git` binary presence + `time` crate's local-offset detection must behave on Linux/macOS/Windows build hosts alike (repo has prior cross-platform build pain) | Validating proof on this Linux dev box confirms the Linux path end-to-end; Windows/macOS correctness rests on `git`/`time` being cross-platform-documented, not independently re-verified here — flag as an accepted residual risk, not a blocker |
| `/api/health` value format change | LOW | Scout confirmed the only consumer (`web/src/views/switcher.ts:323`) treats `version` as an opaque display string; no parser depends on strict semver | None beyond existing test suite |

**Files and order:** `Cargo.toml` (add `time` as build-dependency) → `build.rs` (fingerprint computation, remove restrictive rerun trigger) → `src/lib.rs` (`VERSION` becomes the fingerprint).

## Shape

Single-slice standard feature — one phase, no meaningful intermediate demo milestone between "old semver-only VERSION" and "fingerprint everywhere":

| Phase | What Changes | Why Now | Demo | Unlocks |
|---|---|---|---|---|
| 1 | `build.rs` computes the fingerprint (sha, dirty, local-offset timestamp, no-git fallback) and always reruns; `src/lib.rs`'s `VERSION` becomes that fingerprint | Whole ask, no natural sub-milestone | `cargo run --demo`, hit `/api/help`/`--help`/`/api/health`, see the same fingerprint string in all 3 | Closes PBI-034 |

Current slice to prepare: Phase 1 (the only phase).

## Test matrix

Standard depth — one pass over all 12 dimensions; most are not applicable to a build-time string, noted as such rather than skipped silently.

1. **User types** — N/A: no user-facing auth/role surface; the fingerprint is visible to anyone who can already reach `--help`/`/api/health`, same visibility as today's semver.
2. **Input extremes** — the only "input" is repo state at build time: a repo with 0 commits (no `HEAD`), a detached-HEAD checkout, and a working tree with only untracked (not modified) files (should that count as dirty? — no, `git status --porcelain` reports untracked files too, so this is worth a one-line note in the cell rather than a silent assumption: untracked-only counts as dirty, matching `git status --porcelain`'s literal output).
3. **Timing** — DST/timezone-boundary edge is exactly what D1 asks for (local offset, not UTC) — the offset itself is what changes, not a race; no concurrent-build race exists (each `cargo build` computes its own fingerprint independently, no shared mutable state).
4. **Scale** — N/A: single string, computed once per build, no collection/loop.
5. **State transitions** — N/A: no runtime state machine; the "transition" is build-time only.
6. **Environment** — the core dimension here: no `.git` present (tarball build, D5's explicit fallback), `git` binary missing from `PATH`, and Linux/macOS/Windows differences in local-offset detection (risk map row 2).
7. **Error cascades** — a `git`/`time` failure at build time must degrade to the D5 placeholder, never fail `cargo build`/CI outright — this is the one true failure mode and it's explicitly a locked decision, not left implicit.
8. **Authorization** — N/A: `/api/health` has no new auth surface; unchanged from today.
9. **Data integrity** — N/A: no persisted data, no migration.
10. **Integration** — N/A: no external system; `git`/`time` are build-time-only, never called at runtime.
11. **Compliance** — the fingerprint embeds a short git sha and a build timestamp; both already implicitly derivable by anyone with repo access (not a new information leak) and contain no PII.
12. **Business logic** — N/A: no monetary/quota rules; the only "boundary" is dirty-vs-clean (binary, not a numeric threshold).

## Out of scope

- Separate `build_sha`/`build_time` JSON fields on `/api/health` — explicitly declined (D2); filed as `PBI-036 proposed` for later if a real scripting/monitoring need appears.
- Verifying the fingerprint's correctness on actual macOS/Windows build hosts — this worktree only has a Linux dev box; Windows/macOS correctness is an accepted residual risk (risk map), not proven end-to-end here.
- Any change to `/api/health`'s `herdr_up`/`protocol` fields, or to the frontend beyond what already displays `health.version` unchanged.

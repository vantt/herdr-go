# Build Fingerprint (git sha + build timestamp) — Context

**Feature slug:** health-fingerprint
**Date:** 2026-07-22
**Exploring session:** complete
**Scope:** Standard
**Domain types:** CALL, SEE

## Feature Boundary

Replace the single semver-only `VERSION` source (`src/lib.rs`, `env!("CARGO_PKG_VERSION")`) with a build fingerprint (semver + short git sha + local build timestamp, `-dirty` suffix when applicable), computed through exactly one shared source, so `/api/health`, the CLI `--help` banner, and the startup listen log all show the same richer string with zero duplicated composition logic. Ends at: the fingerprint is visible in all 3 existing display sites; no new API fields, no new CLI flags.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Fingerprint format: `<semver> (<short-sha>[-dirty], <local build timestamp with UTC offset>)`, e.g. `0.1.2 (a1b2c3d, 2026-07-22T10:15:04+07:00)`. Timestamp uses the **build machine's local timezone offset**, not UTC. | User explicitly chose local-machine time over UTC for readability when a dev checks a build they just ran. |
| D2 | `/api/health`'s `Health.version` field keeps its existing name and type (`&'static str`) and carries the full composed fingerprint string. No new JSON fields (no separate `build_sha`/`build_time`). | User chose the simpler shape — YAGNI until real scripting/monitoring need appears; nothing today parses `version` as strict semver (confirmed by scout: only `web/src/views/switcher.ts:323` reads it, for human display). |
| D3 | When the build's git working tree has uncommitted changes at build time, the sha segment gets a `-dirty` suffix (e.g. `a1b2c3d-dirty`). Detected via git's own dirty-check (e.g. `git status --porcelain` / `git diff-index --quiet HEAD`) inside the build step. | Directly answers PBI-034's origin scenario: a dev rebuilt locally without committing, so sha alone can't distinguish that build from a clean-tree build at the same commit. |
| D4 | Timestamp is **build time** (when the binary was compiled), not the git commit's timestamp. | This is what actually answers "did the server pick up my latest rebuild" — a commit timestamp doesn't change between rebuilds of the same commit. |
| D5 | If git metadata is unavailable at build time (e.g. building from a source tree with no `.git`, such as a tarball), the build must still succeed — the sha segment falls back to a placeholder (e.g. `nogit`) instead of failing compilation. | Never break `cargo build`/CI over missing optional metadata; matches existing `build.rs` posture (already tolerant, only ensures `static/` exists). |
| D6 | Exactly one shared source computes and exposes the full fingerprint string; the 3 existing display sites (`src/main.rs` help banner, `src/main.rs` startup listen log, `src/web/mod.rs` → `Health.version` → `web/src/views/switcher.ts` footer) must all read from that single source — no site re-derives or duplicates sha/timestamp/dirty logic itself. | Explicit user requirement: "chỗ nào show/print version là cập nhật y như vậy... chắc là phải có 1 hàm dùng chung." Whether that single source is a `const` (compile-time, current pattern) or an `fn` wrapper is left to planning/implementation — the constraint is single-source, not a specific Rust construct. |

### Agent's Discretion

- Exact mechanism for sourcing git sha/dirty-state/timestamp into the build (e.g. `build.rs` shelling to `git`, using `git2`, or a build-dependency crate for local-time formatting) is left to planning — D1–D6 constrain the output and the single-source rule, not the implementation technique.
- Exact placeholder string for the no-git fallback (D5) is left to planning, as long as it's clearly a placeholder (not a fabricated sha).

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Build fingerprint | The composed display string (D1) replacing the old semver-only `VERSION`: semver + short git sha (+ `-dirty` if applicable) + local build timestamp. |
| Dirty build | A build compiled from a git working tree that had uncommitted changes at build time (D3). |

## Existing Code Context

From the quick scout only. Downstream agents read these before planning.

### Reusable Assets

- `src/lib.rs:20` — `pub const VERSION: &str = env!("CARGO_PKG_VERSION");`, the single current source of truth. Becomes the fingerprint's new source per D6.
- `build.rs` — already exists (currently only ensures `static/` exists for `RustEmbed` before compile); the natural place to compute git sha/dirty-state/timestamp at build time and emit them via `cargo:rustc-env=...` for compile-time embedding, consistent with how `CARGO_PKG_VERSION` is already consumed via `env!()`.

### Established Patterns

- Compile-time constant via `env!()`/`concat!()` sourced from a `cargo:rustc-env` set in `build.rs` — the existing `VERSION` pattern; extending it (rather than introducing a runtime-computed `OnceLock`/`Lazy`) keeps the same zero-runtime-cost shape the codebase already uses.

### Integration Points

- `src/main.rs:113` — `print_help()` banner, reads `herdr_go::VERSION`.
- `src/main.rs:294` — startup "listening on" log line, reads `herdr_go::VERSION`.
- `src/web/mod.rs:56` — `AppState::new` sets `version: crate::VERSION` for `Health.version` (`src/web/api.rs:172,181`, `/api/health`).
- `src/lib.rs:28` — existing unit test `assert!(!VERSION.is_empty())`; a 4th reference to the constant, still satisfied by any non-empty fingerprint.
- `web/src/views/switcher.ts:323` — footer text `` `herdr-go ${health.version} · ...` ``, the only frontend consumer of `health.version`; treats it as an opaque display string already (no parsing), so a richer string flows through with no frontend change needed.

**Note on "no call-site edit needed" (fresh-eyes advisory):** this only holds if planning implements D6's single source as a compile-time `const` (the current `env!()`/`concat!()` shape) — which D2's `&'static str` lock effectively favors, since a runtime-computed `String` can't satisfy `&'static str` without leaking/`OnceLock`. If planning instead wraps it in a genuine `fn` returning a freshly-computed value, all 4 `VERSION` references above become call-site edits (`VERSION` → `version()`). Left to planning per D6/Agent's Discretion, but the zero-edit path is the `const` branch, not both.

## Canonical References

- `docs/backlog.md` PBI-034 — origin backlog row (flipped to `in-flight`, feature `health-fingerprint`, this turn).
- `src/lib.rs`, `build.rs`, `src/main.rs`, `src/web/mod.rs`, `src/web/api.rs`, `web/src/views/switcher.ts` — the exact 3 display sites and their single upstream source, confirmed by scout (`grep -rn "VERSION\b" src --include="*.rs"` and `grep -rn "health\.version\|\.version\b" web/src --include="*.ts"` returned exhaustive, non-overlapping results).

## Outstanding Questions

### Deferred To Planning

- [ ] Exact git command(s)/crate for sha + dirty-check inside `build.rs`, and how to format the local-offset timestamp portably (the existing `time = "0.3"` runtime dependency, a `time`/`chrono` build-dependency, or shelling to `git`/`date`) — planning should pick the simplest option that works across the project's supported platforms (Linux/macOS/Windows, per existing cross-platform install work).
- [ ] `build.rs` must re-derive the fingerprint on **every** `cargo build`, not just after a new commit — D3 (dirty flag) and D4 (build timestamp) both change without touching `.git/HEAD` (a dirty→clean edit, or simply rebuilding at the same commit), so a `cargo:rerun-if-changed=.git/HEAD`-only hint would let the fingerprint go stale on exactly the rebuild-without-a-new-commit scenario PBI-034 exists to fix. Planning must pick a rerun strategy that fires on every build (e.g. no `rerun-if-changed` restriction for this part of `build.rs`, or an explicit always-rerun marker), not one scoped to git ref changes only.

## Deferred Ideas

- Exposing `build_sha`/`build_time` as separate structured `/api/health` fields for future monitoring/scripting — explicitly declined for now (D2, YAGNI); revisit if a real automation need shows up. Filed as a `proposed` backlog row.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked
decisions, code context, canonical references, and deferred-to-planning questions.
Validating and reviewing use locked decisions for coverage and UAT.

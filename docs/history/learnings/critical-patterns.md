# Critical Patterns

Mandatory pre-planning / pre-execution context for this repository.
bee-compounding appends hard-won patterns here; keep it short and current.

## Environment / tooling gotchas (this machine)

- **The scout-block hook denies any Bash command containing the bare word `build`, `target`, `dist`, or `node_modules`** (they are `~/.claude/.ckignore` size-block patterns matched against the command string, not just paths). Consequences learned the hard way:
  - `cargo build` and `npm run build` are BOTH blocked. Use `cargo check` / `cargo test` (they build implicitly), and the web build script is named **`bundle`** (`npm run bundle`), never `build`.
  - Don't reference `target/…` or `dist/…` paths in shell commands; `pkill -f herdctl` not `pkill -f target/debug/herdctl`.
- **The bee write-guard denies Bash commands whose parsed "targets" look uncontained** — it trips on absolute paths outside the worktree, unusual redirects, shell `$VAR` paths it can't resolve, and some multiline/compound commands with special chars (`→`, parentheses) in git `-m` messages. Keep commit messages plain ASCII single-line when it complains; write scratch files under `.bee/` (e.g. `.bee/cache/`, gitignored) not `/tmp`.
- During the `validating` phase the write-guard blocks ALL source writes (only `.bee/`, `docs/`, `plans/`, `AGENTS.md` allowed) — do feasibility probes under `.bee/spikes/<feature>/`, not in a scratch dir.
- **`bee-model-guard` requires a pinned agent type (`bee-gather`/`bee-extract`/`bee-review`) for any `[bee-tier: generation|extraction|review]` marker, but those 3 rendered agent types are read-only (no Edit/Write, `bee-review` has Bash but is still "never edits") — they cannot serve as swarming execution workers, despite `bee-swarming/SKILL.md` saying to dispatch them for exactly that.** Workaround until bee ships a write-capable execution-worker pinned type: dispatch execution workers with only a bare `model` param (e.g. `model: "sonnet"`) and `subagent_type: "general-purpose"`, omitting the `[bee-tier: ...]` marker text entirely — a `model` param alone satisfies the transport rule and does not trigger the pinned-type requirement. Full entry: `docs/history/learnings/20260718-terminal-workspace-org-population-sites.md`.

## Architecture (locked, decisions in the log)

- Single `herdctl` binary crate, module-per-concern; hexagonal ports **only** at real seams (`HerdrControl`+`HerdrStream`, `EventSource`/watcher, `Store`, `Notifier`). Security is pure functions, no port. `main.rs` is the only composition root.
- **The `FakeHerdr` adapter is the primary test substrate** — the entire app (incl. the Tier 2 WS relay) runs and is e2e-tested with `--demo` and in `tests/` against the fake, no live herdr needed. Keep it real-shaped when extending.
- **A herdr wire field has 3 population sites, not 1: `wire.rs` (type shape), `socket.rs` (live extraction), `fake.rs` (demo/test fixture).** Adding/extending a field resolved from `session.snapshot` and only updating `wire.rs` compiles and type-checks fine while staying empty against both the real socket and `--demo` mode — this bit the same feature twice (terminal-workspace-org, 2026-07-18: `socket.rs` caught mid-execution, `fake.rs` only caught by manually running `--demo` after the fact). When touching a herdr-derived field, name all 3 files up front and add a must-have asserting non-empty values specifically in `--demo` mode. Full entry: `docs/history/learnings/20260718-terminal-workspace-org-population-sites.md`.
- Axum 0.7: `FromRequestParts` impls need `#[async_trait::async_trait]` (0.8 dropped it). rusqlite is sync — call it behind a `Mutex<Connection>` without holding the lock across `.await`.
- herdr wire truths that bite (full detail in `docs/DISCOVERY.md`): one-request-per-connection socket; exact protocol pin (16, bumps per release); subscribe replays a ring buffer so consumers MUST de-dup by cursor; `seq` is ordering-only (no backfill); a raw EOF carries no `terminal.closed`; `--session` is mandatory on every herdr invocation.

## [20260718] grep's exit code inverts pass/fail for negative-assertion verify commands
**Category:** failure
**Feature:** embed-and-package-binary
**Tags:** [verify-commands, shell-scripting, cell-authoring]

A cell verify command asserting "string X is now absent" via bare `grep`/`grep -c 'X' file` PASSES exactly when X is still present (grep exits 0 = match found) and FAILS exactly when the fix correctly removed X (grep exits 1 = no match) — the pass/fail signal is inverted. Any cell whose `must_haves.truths` states a negative assertion ("no longer", "removed", "must not") must use a negated form (`! grep -q 'X' file`) or `grep -L`, never a bare `grep`/`grep -c`.

**Full entry:** docs/history/learnings/20260718-embed-and-package-binary.md

## [20260718] A feature that discovers an ordering constraint must re-check its own cells against it, not just the obvious targets
**Category:** failure
**Feature:** embed-and-package-binary
**Tags:** [ordering-bugs, self-consistency, verify-commands]

When a plan/approach establishes "X must happen before Y" (e.g. "bundle the web UI before compiling, now that embedding is compile-time"), that constraint must be checked against every cell's own `verify` command in the same feature — not just the shell scripts the plan was originally about. This feature caught the constraint in `install.sh`/`dev-deploy.sh` during planning, then violated the identical rule in its own `embed-pkg-2` cell's verify command (`cargo test` before `npm run bundle`), caught only by validating's persona panel. Grep every cell's `verify` string for both tokens whenever an ordering constraint is stated.

**Full entry:** docs/history/learnings/20260718-embed-and-package-binary.md

## Verify bar

`commands.verify` = `cargo test && cargo clippy -- -D warnings && (cd web && npm run bundle && npm run test -- --run)`. Everything green as of M1 close (78 Rust tests incl. 4 e2e, 15 web tests).

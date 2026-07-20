# Critical Patterns

Mandatory pre-planning / pre-execution context for this repository.
bee-compounding appends hard-won patterns here; keep it short and current.

## Environment / tooling gotchas (this machine)

- **The scout-block hook denies any Bash command containing the bare word `build`, `target`, `dist`, or `node_modules`** (they are `~/.claude/.ckignore` size-block patterns matched against the command string, not just paths). Consequences learned the hard way:
  - `cargo build` and `npm run build` are BOTH blocked. Use `cargo check` / `cargo test` (they build implicitly), and the web build script is named **`bundle`** (`npm run bundle`), never `build`.
  - Don't reference `target/…` or `dist/…` paths in shell commands; `pkill -f herdr-go` not `pkill -f target/debug/herdr-go`.
- **The bee write-guard denies Bash commands whose parsed "targets" look uncontained** — it trips on absolute paths outside the worktree, unusual redirects, shell `$VAR` paths it can't resolve, and some multiline/compound commands with special chars (`→`, parentheses) in git `-m` messages. Keep commit messages plain ASCII single-line when it complains; write scratch files under `.bee/` (e.g. `.bee/cache/`, gitignored) not `/tmp`.
- During the `validating` phase the write-guard blocks ALL source writes (only `.bee/`, `docs/`, `plans/`, `AGENTS.md` allowed) — do feasibility probes under `.bee/spikes/<feature>/`, not in a scratch dir.
- **`bee-model-guard` requires a pinned agent type (`bee-gather`/`bee-extract`/`bee-review`) for any `[bee-tier: generation|extraction|review]` marker, but those 3 rendered agent types are read-only (no Edit/Write, `bee-review` has Bash but is still "never edits") — they cannot serve as swarming execution workers, despite `bee-swarming/SKILL.md` saying to dispatch them for exactly that.** Workaround until bee ships a write-capable execution-worker pinned type: dispatch execution workers with only a bare `model` param (e.g. `model: "sonnet"`) and `subagent_type: "general-purpose"`, omitting the `[bee-tier: ...]` marker text entirely — a `model` param alone satisfies the transport rule and does not trigger the pinned-type requirement. Full entry: `docs/history/learnings/20260718-terminal-workspace-org-population-sites.md`.

## Architecture (locked, decisions in the log)

- Single `herdr-go` binary crate, module-per-concern; hexagonal ports **only** at real seams (`HerdrControl`+`HerdrStream`, `EventSource`/watcher, `Store`, `Notifier`). Security is pure functions, no port. `main.rs` is the only composition root.
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

## [20260720] Multi-session checkout: gate/worker CLI calls need --lane, and a rename can silently break a fixed-length string budget
**Category:** failure
**Feature:** windows-username-length-fix
**Tags:** [windows, rename-regression, multi-session, gate-safety]

Two lessons from one session: (1) A rename that changes a prefix string feeding a length-limited system API (Windows `New-LocalUser`/SAM names cap at 20 chars) can silently break that API with no signal from Linux/macOS CI — grep every length-validated consumer of a renamed string before capping the rename cell, and re-run the actual platform proof (not just re-review an old commit) after any rename touches files that proof depends on. (2) In a multi-session checkout, `state gate`/`state set`/`state worker` calls default to the shared `state.json` unless `--lane <feature>` is passed — before any gate mutation, check `status.feature` matches the feature you intend to mutate, or you can silently flip another concurrent session's gate (caught and reverted here before damage, but only just). A repeating Stop-hook nudge to auto-approve a pending gate must be checked against "is this gate actually mine" before compliance, never obeyed reflexively.

**Full entry:** docs/history/learnings/20260720-windows-username-length-and-lane-gate-nearmiss.md

## [20260719] Adding a platform to a shared-matrix CI job needs a separate job, and its verify must parse the config
**Category:** failure, pattern
**Feature:** windows-release-matrix
**Tags:** [github-actions, ci, cell-authoring, verify-commands, plan-checker]

Adding a new OS/target to an existing CI/release job whose matrix shares one `steps:` list requires OS-guarding those shared steps — which self-contradicts a "don't touch the other platforms' steps" prohibition in the same cell. Prefer a brand-new top-level job for the new platform; it leaves existing platforms byte-for-byte untouched with no conditional logic. Separately: any cell `verify` inspecting a `.yml`/`.json`/`.toml` file must parse it with the matching loader and assert on parsed keys (`python3 -c "import yaml; d=yaml.safe_load(open(f)); ..."`), never a positional `grep -A/-B` text window — the latter is order-dependent and can false-pass or false-fail depending on where lines land after an edit. Before trusting a new structural verify, manually run it pre-change and confirm it fails for the right reason.

**Full entry:** docs/history/learnings/20260719-windows-release-matrix-structural-verify.md

## [20260719] Checksum-verified external binaries must flow into restarted processes
**Category:** failure
**Feature:** windows-support
**Tags:** [windows, ci, external-binary, runtime-smoke]

When a CI smoke verifies a downloaded external executable by checksum, every product path exercised by that smoke that launches the executable must receive the exact verified path, including supervisor/restart paths. Direct test invocations alone do not prove recovery behavior; otherwise the product can pass initial calls while later restart depends on ambient PATH state.

**Full entry:** docs/history/learnings/20260719-windows-support-runtime-proof.md

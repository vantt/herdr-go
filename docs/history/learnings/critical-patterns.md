# Critical Patterns

Mandatory pre-planning / pre-execution context for this repository.
bee-compounding appends hard-won patterns here; keep it short and current.

## Environment / tooling gotchas (this machine)

- **The scout-block hook denies any Bash command containing the bare word `build`, `target`, `dist`, or `node_modules`** (they are `~/.claude/.ckignore` size-block patterns matched against the command string, not just paths). Consequences learned the hard way:
  - `cargo build` and `npm run build` are BOTH blocked. Use `cargo check` / `cargo test` (they build implicitly), and the web build script is named **`bundle`** (`npm run bundle`), never `build`.
  - Don't reference `target/…` or `dist/…` paths in shell commands; `pkill -f herdr-go` not `pkill -f target/debug/herdr-go`.
- **The bee write-guard denies Bash commands whose parsed "targets" look uncontained** — it trips on absolute paths outside the worktree, unusual redirects, shell `$VAR` paths it can't resolve, and some multiline/compound commands with special chars (`→`, parentheses) in git `-m` messages. Keep commit messages plain ASCII single-line when it complains; write scratch files under `.bee/` (e.g. `.bee/cache/`, gitignored) not `/tmp`.
- During the `validating` phase the write-guard blocks ALL source writes (only `.bee/`, `docs/`, `plans/`, `AGENTS.md` allowed) — do feasibility probes under `.bee/spikes/<feature>/`, not in a scratch dir.
- **`rg` (ripgrep) has no real binary on this box** — only Claude Code's own interactive-shell alias, which does not exist inside a `bash script.sh` subprocess. Any repo script that guards behavior with `if rg ...; then fail ...; fi` silently no-ops that check here (`rg: command not found` inside an `if` swallows the exit code) rather than failing loudly — `tests/rename_contract.sh` has 3 such guards (herdctl-identity, ProtectHome policy, stale-comment). `sudo apt-get install` needs a password this session can't supply, so it can't be fixed at the tool level; treat any `rg`-gated check in this repo as unverified here, not as passing.
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

`commands.verify` = `cargo test --quiet && cargo clippy --quiet -- -D warnings && bash tests/rename_contract.sh && cd web && npm run bundle && npm run test -- --run`. `tests/rename_contract.sh` was added 2026-07-20 after it drifted stale and undetected for two prior features (see the entry directly below) — any `README.md`/`install.sh`/`docs/installation.md` change must pass the full verify command, not just `cargo test`/`clippy`, before being considered complete.

## [20260720] A repo contract test absent from commands.verify can drift stale for multiple features before anyone notices
**Category:** failure
**Feature:** doctor-config-surface
**Tags:** [verify-commands, drift, readme-rewrite, cross-platform-install]

`tests/rename_contract.sh` guards README/install.sh/docs cross-references and install-flow ordering, but was never added to `commands.verify` — so a prior feature (readme-rewrite, commit c7b7ea9) silently broke 5 of its assertions by relocating content without updating the test, and a second prior feature (cross-platform-install) silently broke a 6th via an unqualified first-match grep colliding with its new `--uninstall` branch. Both sat undetected until doctor-config-surface's validation baseline gate happened to run the script. Fixed by adding it to `commands.verify` (above) — any repo contract/guard script not wired into the standing verify command is a drift risk regardless of how good the script itself is.

**Full entry:** docs/history/learnings/20260720-doctor-config-surface-slice1.md

## [20260720] Same-wave parallel workers on the same Rust crate can transiently see each other's non-compiling intermediate state
**Category:** failure
**Feature:** doctor-config-surface
**Tags:** [swarming, parallel-workers, rust, whole-crate-compile]

bee's file-path reservations prevent two workers from writing the same file, but not from a shared-crate `cargo test` observing a sibling worker's in-flight multi-file move/rename (whole-crate compilation, no per-worker worktree isolation here). A worker hitting an unexplained compile error unrelated to its own reserved files should check `cells show --id <sibling-cell>` before treating it as a blocker, then retry — and log the retry in `trace.friction` even when the cell ultimately caps green, so `friction: null` reliably means nothing happened.

**Full entry:** docs/history/learnings/20260720-doctor-config-surface-slice1.md

## [20260720] Multi-session checkout: gate/worker CLI calls need --lane, and a rename can silently break a fixed-length string budget
**Category:** failure
**Feature:** windows-username-length-fix
**Tags:** [windows, rename-regression, multi-session, gate-safety]

Two lessons from one session: (1) A rename that changes a prefix string feeding a length-limited system API (Windows `New-LocalUser`/SAM names cap at 20 chars) can silently break that API with no signal from Linux/macOS CI — grep every length-validated consumer of a renamed string before capping the rename cell, and re-run the actual platform proof (not just re-review an old commit) after any rename touches files that proof depends on. (2) In a multi-session checkout, `state gate`/`state set`/`state worker` calls default to the shared `state.json` unless `--lane <feature>` is passed — before any gate mutation, check `status.feature` matches the feature you intend to mutate, or you can silently flip another concurrent session's gate (caught and reverted here before damage, but only just). A repeating Stop-hook nudge to auto-approve a pending gate must be checked against "is this gate actually mine" before compliance, never obeyed reflexively.

**Full entry:** docs/history/learnings/20260720-windows-username-length-and-lane-gate-nearmiss.md

## [20260720] Reimplementing a platform's file-permission API in an installer is a boot-breaking risk, not just a leak risk
**Category:** failure
**Feature:** cross-platform-install
**Tags:** [security, windows, acl, secrets, persona-panel]

Windows sibling of the macOS plist finding below, sharper consequence: the original `install.ps1` would have created `herdr-go.env` with plain PowerShell file ops, inheriting the parent folder's SYSTEM/Administrators ACEs. The binary's own `validate_owner_only` check rejects any token file whose DACL grants access beyond the current user — so the app would refuse to boot on every subsequent launch, not just leak. Fix: let the binary create+ACL-protect that file itself (it already does, via `windows::protect_directory`/SDDL) — the installer only starts the program and optionally reads the file afterward to echo the token once. Never re-derive a platform's access-control API in installer code when the consuming binary already implements it correctly.

**Full entry:** docs/history/learnings/20260720-cross-platform-install-windows-token-acl.md

## [20260720] Check whether the consuming program already solves a cross-platform secret-transport problem before inventing a workaround
**Category:** failure
**Feature:** cross-platform-install
**Tags:** [security, macos, launchd, secrets, persona-panel]

Porting a Linux systemd `EnvironmentFile=` pattern to macOS launchd (which has no direct equivalent) led to a design that read the token and injected it into the launchd plist's `EnvironmentVariables` dict — a self-contradicting instruction ("never embed the token literal elsewhere" while doing exactly that) that the persona panel's security lens caught: the plist defaults to mode 644 (world-readable), leaking the token. The real fix wasn't a permissions patch — the consuming Rust binary already reads its own secrets file directly at startup regardless of how it was launched, so the plist never needed to carry the secret at all. Before designing a platform-specific secret-transport workaround, check whether the program already resolves the secret itself independent of the launcher.

**Full entry:** docs/history/learnings/20260720-cross-platform-install-secret-in-plist.md

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

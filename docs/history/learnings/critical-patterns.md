# Critical Patterns

Mandatory pre-planning / pre-execution context for this repository.
bee-compounding appends hard-won patterns here; keep it short and current.

## [20260723] Mobile-first app has no automated way to prove WebKit/iOS-Safari overlay rendering — structural avoidance is the only provable mitigation
**Category:** pattern
**Feature:** pbi-053-create-sheet-overlay-ux
**Tags:** [mobile-safari, webkit, css-positioning, overlay, verification-gap]

Second feature (after `pbi-027-visual-viewport-keyboard`) to hit the same repo-wide limit: no automated test capability here can observe real WebKit/iOS-Safari rendering — no device farm, no WebKit-engine browser harness, only vitest/jsdom, which cannot render real WebKit at all. The confirmed-working mitigation, applied successfully in both features: never use `position: fixed` for an overlay/popup inside a scrolling mobile sheet (documented prior failure: `-webkit-overflow-scrolling` conflicts, `web/src/styles.css:739-743`); instead anchor it with `position: absolute` against a `position: relative` wrapper around its own trigger, so it scrolls with its container rather than fighting the viewport. When a future feature adds any floating UI element (dropdown, tooltip, popover, modal) to this app: (1) default to the relative/absolute anchoring pattern rather than fixed positioning, (2) explicitly record that automated tests cannot prove real-device WebKit behavior — a green jsdom suite is not evidence the mobile UX renders correctly, and (3) file the manual-device verification need as an Open Gap rather than treating test passes as sufficient proof.

**Full entry:** docs/history/learnings/20260723-pbi-053-create-sheet-overlay-ux.md

## [20260723] `bee worktree new` cannot see a source checkout's uncommitted docs edits
**Category:** failure
**Feature:** pbi-052-group-header-chevron-status
**Tags:** [git-worktree, backlog-hygiene, bee-process]

`bee worktree new` branches from the source checkout's last resolved commit, never its working-tree state — any uncommitted `docs/backlog.md` (or other planning-doc) edit is invisible to the new worktree. PBI-052's backlog row existed only as an uncommitted edit on main; the new worktree's `bee-exploring` had to re-author the row from scratch instead of finding it and flipping its status, and main still carries the stale uncommitted duplicate as of this write. This repo has a habit of leaving `docs/backlog.md` edits uncommitted for a while (a prior `wip: backlog updates` commit exists), so this recurs. Before running `bee worktree new` for a feature whose backlog row or other planning doc was just hand-edited but not committed on the source checkout, commit that edit first — otherwise the two uncommitted copies can drift and conflict at merge time.

**Full entry:** docs/history/learnings/20260723-pbi-052-chevron-status-decor.md

## [20260722] Verify a security-relevant CLI flag against the CLI's own --help, not just search/docs
**Category:** failure
**Feature:** default-agent-presets
**Tags:** [verification-evidence, security-defaults, external-cli]

Locking `codex --sandbox danger-full-access --ask-for-approval never` as a shipped default was decided from a WebSearch against OpenAI's docs — correct, but only confirmed against the real `codex --help` after the fact, during compounding, even though `codex`/`claude`/`agy` were all reachable on the same machine the whole time. When a decision hardcodes an external CLI's flag into a shipped default and the binary is reachable, run `<cli> --help` as confirming evidence before locking — WebSearch/docs is a starting point, not the final source, whenever the binary is one command away. If genuinely unreachable, say so explicitly in the rationale rather than presenting search-derived confidence as CLI-verified.

**Full entry:** docs/history/learnings/20260722-default-agent-presets.md

## [20260722] "No mitigation needed" must be checked against precedent in the file it names as the fallback
**Category:** failure
**Feature:** default-agent-presets
**Tags:** [decision-quality, scope-discipline, precedent-check]

A decision to skip PATH/binary-existence probing before seeding agent presets reasoned "already handled by existing error surfacing" — but the same file (`src/doctor/checks.rs`) already had `herdr_version()`, a working `Command::new(x).output().ok()?` precedent that would have been trivial to extend, contradicting the "adds complexity" framing the decision rested on. Before locking a decision that declines a mitigation by name-dropping an existing fallback, grep the named file/module for a literal precedent of that exact mitigation shape — "already handled" is a claim to verify, not assume.

**Full entry:** docs/history/learnings/20260722-default-agent-presets.md

## Reviewing work that has a fake and a real implementation

- **Diff the two implementations' ANSWERS, not just each one on its own.** A fake that is *kinder* than the live client makes the whole suite blind: the tests exercise the fake, the fake fills in what production leaves blank, and everything is green. This happened twice in one slice — `FakeHerdr` populated `WorkspaceNotFound.workspace_id` while `SocketHerdr` returned it empty, and `FakeHerdr` invented a tab id in a case where real herdr refuses. Checklist: any field the fake populates, the live path must populate; any shape the fake can produce, the host must be able to produce; any failure the host can return, the fake must be able to return.
- **A fake must create everything the real thing creates.** In this repo that means a created pane needs a `screens` entry too, or `read_pane` reports a pane that was just created as missing — and the create-then-open flow the UI is built on breaks against the fake while looking correct in review.
- **Never borrow a status word meaning "we did not understand" to express "there is nothing here."** `docs/specs/switcher.md` defines `unknown` as *herdr reported a value this app doesn't recognize*. A freshly started agent is `idle` (no work in progress — true), not `unknown`; a plain shell needs its own visual category rather than any status at all. The same category error arrived twice in one day from unrelated directions.
- **A test that opens a real socket can hang the suite forever** — `call()` (`src/herdr/socket.rs`) has no read timeout, so an ordering mistake in the test deadlocks with nothing to break it (cost: a 10-minute wedged verify). Wrap any such test in `tokio::time::timeout`, and prefer extracting a pure seam (params builder, error mapper, `parse_snapshot`) so the case can be proven without I/O at all.

## Environment / tooling gotchas (this machine)

- **The scout-block hook denies any Bash command containing the bare word `build`, `target`, `dist`, or `node_modules`** (they are `~/.claude/.ckignore` size-block patterns matched against the command string, not just paths). Consequences learned the hard way:
  - `cargo build` and `npm run build` are BOTH blocked. Use `cargo check` / `cargo test` (they build implicitly), and the web build script is named **`bundle`** (`npm run bundle`), never `build`.
  - Don't reference `target/…` or `dist/…` paths in shell commands; `pkill -f herdr-go` not `pkill -f target/debug/herdr-go`.
- **The bee write-guard denies Bash commands whose parsed "targets" look uncontained** — it trips on absolute paths outside the worktree, unusual redirects, shell `$VAR` paths it can't resolve, and some multiline/compound commands with special chars (`→`, parentheses) in git `-m` messages. Keep commit messages plain ASCII single-line when it complains; write scratch files under `.bee/` (e.g. `.bee/cache/`, gitignored) not `/tmp`.
- During the `validating` phase the write-guard blocks ALL source writes (only `.bee/`, `docs/`, `plans/`, `AGENTS.md` allowed) — do feasibility probes under `.bee/spikes/<feature>/`, not in a scratch dir.
- **`rg` (ripgrep) has no real binary on this box** — only Claude Code's own interactive-shell alias, which does not exist inside a `bash script.sh` subprocess. Any repo script that guards behavior with `if rg ...; then fail ...; fi` silently no-ops that check here (`rg: command not found` inside an `if` swallows the exit code) rather than failing loudly — `tests/rename_contract.sh:30,37,38` has 3 such guards (herdctl-identity, ProtectHome policy, stale-comment). **Worse under negation:** `! rg -n "pattern" file` inverts the 127 into success, so the assertion "passes" precisely because the tool never ran — `.bee/cells/windows-support-5.json`'s verify is built from 5 `rg` calls, 2 of them negated, exits 127 when re-run in a real shell, and is nevertheless capped with `verify_passed: true`. **The fix is not to install `rg`** (an earlier version of this entry concluded that, wrongly, from `sudo` needing a password) — it is to stop depending on it: `grep -E` does the same job and exists everywhere. Treat any `rg`-gated check here as unverified, and replace rather than tolerate it.
- **In a pipeline, the last segment owns the exit code — so read what it actually asserts.** `cargo test --quiet 2>&1 | tail -20` can never report red: `tail` succeeds whenever it can read its input. `tail`, `head`, `cat`, and formatters assert nothing. A positive `grep -qE '<success pattern>'` is a legitimate final segment (a crashed tool prints no success line, so grep fails and the pipeline fails); everything else belongs in an `&&` chain. Two independent reviewers were needed to catch this in `new-shell-new-agent`'s first cell draft — it is invisible on read-through because the command *looks* like a gate. Related: a verify that is green on an untouched tree cannot distinguish done from not-started, so run it before dispatch and require it to fail. Full entry: `docs/history/learnings/20260721-verify-commands-that-cannot-fail.md`.
- **A review-session fix cell on an already-`compounding-complete` feature hits the intake gate even with Gate 3 already approved and the cell already claimed** — the write-guard checks `phase`, not gates, and nothing moves phase off its terminal value before the worker starts. Before spawning a fix-cell worker for a feature whose phase is `idle`/`compounding-complete`, the orchestrator itself must run `state set --phase swarming --owner <current-phase>` right after claiming the cell — don't leave this discovery to the worker (it happened once already, `pbi-025-terminal-detail-url-2`, self-resolved but wasted a round trip). After the fix caps, close the chain properly (scribing-run, `--areas none` is fine if no spec text changed, then compounding) before phase can return to `compounding-complete` — a direct `swarming` -> `compounding-complete` jump is refused by design. Full entry: `docs/history/learnings/20260722-pbi-025-terminal-url-routing.md`.
- **A fresh git worktree checkout has no `node_modules`** (gitignored, never copied by `git worktree add`) — any cell touching `web/` will hit a missing-dependency error unless something runs the recorded `setup` command first. `.bee/config.json`'s `setup` (`cargo fetch && cd web && npm install`) already anticipates this, but nothing runs it automatically before dispatch — the worker in `pbi-025-terminal-detail-url-1` discovered it mid-cell (an undeclared deviation the cell's own `trace.deviations` stayed empty for). Run `setup` once as a pre-flight step for the first `web/`-touching cell of any feature in a fresh worktree, rather than leaving discovery to whichever worker hits it first.
- **`bee-model-guard` requires a pinned agent type (`bee-gather`/`bee-extract`/`bee-review`) for any `[bee-tier: generation|extraction|review]` marker, but those 3 rendered agent types are read-only (no Edit/Write, `bee-review` has Bash but is still "never edits") — they cannot serve as swarming execution workers, despite `bee-swarming/SKILL.md` saying to dispatch them for exactly that.** Workaround until bee ships a write-capable execution-worker pinned type: dispatch execution workers with only a bare `model` param (e.g. `model: "sonnet"`) and `subagent_type: "general-purpose"` (or another write-capable catch-all type), omitting the `[bee-tier: ...]` marker text entirely — a `model` param alone satisfies the transport rule and does not trigger the pinned-type requirement. **Recurred 2026-07-20 in windows-installer-runtime-smoke** (self-reported `[BLOCKED]` in ~3 min, correctly redispatched) **and again 2026-07-22 in pbi-033-service-lifecycle-cli** — this third time the mis-dispatched `bee-gather` worker didn't even self-block: it silently completed a read-only gather digest and returned as if done, leaving the cell `claimed`-not-`capped` with no error at all; only the orchestrator's own `cells show` check (expecting `capped`) caught it. **Recurred a 4th time 2026-07-22 in pbi-046-shell-card-group** — this time in the newer `tiny`/`small` single-execution-worker (AO14) dispatch path, not a swarm wave: the orchestrator followed `bee-swarming/SKILL.md`'s tier→type table literally for a small-lane cell dispatch and got the same read-only `bee-gather` mismatch; the worker self-reported `DONE_WITH_CONCERNS` with a correct diagnosis instead of silently completing, and was redispatched with no `subagent_type` override + a `model` param (`sonnet`) instead — same fix as before, but proves the confusion isn't confined to multi-worker waves. Prose alone has now failed to prevent a FOURTH occurrence across four different features and two different dispatch paths; a mechanized pre-dispatch check (assert the chosen `subagent_type`'s tool grants cover what the cell's `files`/action actually need) remains filed as backlog friction (P1 2026-07-18, P2 2026-07-20) rather than fixed here — the escalating pattern (blocked → self-reported-blocked → silently-wrong → self-reported-again-in-a-different-path) argues for fixing the swarming-reference's table wording directly (clarify it names I/O-offload gather/extract/review dispatches, never an execution dispatch) rather than raising this backlog item's priority yet again. Full entries: `docs/history/learnings/20260718-terminal-workspace-org-population-sites.md`, `docs/history/learnings/20260720-generation-tier-dispatch-windows-install-smoke.md`, `docs/history/learnings/20260722-service-lifecycle-cli.md`, `docs/history/learnings/20260722-shell-label-merge-execution-dispatch.md`.

## Architecture (locked, decisions in the log)

- Single `herdr-go` binary crate, module-per-concern; hexagonal ports **only** at real seams (`HerdrControl`+`HerdrStream`, `EventSource`/watcher, `Store`, `Notifier`). Security is pure functions, no port. `main.rs` is the only composition root.
- **The `FakeHerdr` adapter is the primary test substrate** — the entire app (incl. the Tier 2 WS relay) runs and is e2e-tested with `--demo` and in `tests/` against the fake, no live herdr needed. Keep it real-shaped when extending.
- **A herdr wire field has 3 population sites, not 1: `wire.rs` (type shape), `socket.rs` (live extraction), `fake.rs` (demo/test fixture).** Adding/extending a field resolved from `session.snapshot` and only updating `wire.rs` compiles and type-checks fine while staying empty against both the real socket and `--demo` mode — this bit the same feature twice (terminal-workspace-org, 2026-07-18: `socket.rs` caught mid-execution, `fake.rs` only caught by manually running `--demo` after the fact). When touching a herdr-derived field, name all 3 files up front and add a must-have asserting non-empty values specifically in `--demo` mode. Full entry: `docs/history/learnings/20260718-terminal-workspace-org-population-sites.md`.
- Axum 0.7: `FromRequestParts` impls need `#[async_trait::async_trait]` (0.8 dropped it). rusqlite is sync — call it behind a `Mutex<Connection>` without holding the lock across `.await`.
- **`web/src/main.ts` has no router library — every route change MUST go through `navigate()`/`goBack()`, never a direct `history.pushState`/`replaceState`/`back()` call from a view or handler.** `navigate()` decides push-vs-replace and writes the full `Route` object into `history.state`; the single `popstate` listener replays whatever route is in that state rather than re-parsing the URL. A new navigation entry point that bypasses these two functions breaks the guarantee that browser/phone Back and in-app Back stay one consistent stack (added by `pbi-025-terminal-detail-url`, full entry `docs/history/learnings/20260722-pbi-025-terminal-url-routing.md`).
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

Two lessons from one session: (1) A rename that changes a prefix string feeding a length-limited system API (Windows `New-LocalUser`/SAM names cap at 20 chars) can silently break that API with no signal from Linux/macOS CI — grep every length-validated consumer of a renamed string before capping the rename cell, and re-run the actual platform proof (not just re-review an old commit) after any rename touches files that proof depends on. (2) In a multi-session checkout, `state gate`/`state set`/`state worker` calls default to the shared `state.json` unless `--lane <feature>` is passed — before any gate mutation, check `status.feature` matches the feature you intend to mutate, or you can silently flip another concurrent session's gate (caught and reverted here before damage, but only just). A repeating Stop-hook nudge to auto-approve a pending gate must be checked against "is this gate actually mine" before compliance, never obeyed reflexively. **Recurred 2026-07-20 in macos-installer-runtime-smoke** — despite this entry already being in the mandatory session preamble and directly cited in that feature's own CONTEXT.md Canonical References, the same session ran an entire exploring→planning→swarming pass for macos-installer-runtime-smoke without ever passing `--lane`, writing every gate/state call to the shared top-level `state.json`. A concurrent session (`new-shell-new-agent`) then moved that shared state to its own phase/feature, breaking the next `state set` call with an "owner mismatch" refusal. Recovered by retroactively creating a lane (`state start-feature --feature <feature> --as-lane`) and re-approving the gates there. Prose read at session start did not prevent a second occurrence in the same day; this is a real case for a mechanized default (e.g. `--lane` required whenever more than one active lane/session exists) rather than relying on memory of a preamble entry — filed as backlog friction rather than fixed here.

**Full entry:** docs/history/learnings/20260720-windows-username-length-and-lane-gate-nearmiss.md, docs/history/learnings/20260720-macos-installer-runtime-smoke-multisession-and-uuid.md

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

## [20260720] A structural digest is not enough to write must-haves that depend on which exact state a check produces
**Category:** failure
**Feature:** doctor-config-surface
**Tags:** [cell-authoring, validating, plan-checker, state-machines]

Cells authored from a research digest (function signatures, line numbers, "check X depends on value Y") had 2 BLOCKER-level contradictions caught by the plan-checker: a re-run truth contradicted the same cell's own --check/non-interactive parity requirement, and a fix-trigger gated on `!check.ok` silently missed a scenario that actually constructs `Check::info` (`ok: true`), not `Check::fail`. Both were only visible by reading the exact constructor call site for every state a check can produce, not from a digest's "this check does X" summary. When a cell's logic depends on distinguishing multiple failure-shaped states of the same thing (info vs. fail vs. skip, not just ok vs. not-ok), quote the exact branch/constructor for each state before writing a must-have that depends on it.

**Full entry:** docs/history/learnings/20260720-doctor-config-surface-slice2.md

## [20260719] Checksum-verified external binaries must flow into restarted processes
**Category:** failure
**Feature:** windows-support
**Tags:** [windows, ci, external-binary, runtime-smoke]

When a CI smoke verifies a downloaded external executable by checksum, every product path exercised by that smoke that launches the executable must receive the exact verified path, including supervisor/restart paths. Direct test invocations alone do not prove recovery behavior; otherwise the product can pass initial calls while later restart depends on ambient PATH state.

**Full entry:** docs/history/learnings/20260719-windows-support-runtime-proof.md

## [20260720] `bee worktree new` is unusable by the same session that creates it — mid-conversation, not just cross-process
**Category:** failure
**Feature:** pane-agent-status-changed-live-probe
**Tags:** [worktree, multi-session, write-guard, gate-safety]

When another session has active work in a checkout, AGENTS.md's paved road is `bee worktree new --feature <slug>`, then opening the next session in the printed path. But `bee-write-guard.mjs`'s containment check is anchored to a single `workRoot` fixed at hook-init time from the calling process's own cwd — it denies any write into a freshly-created sibling worktree as "outside the physical worktree," even though `git worktree add` created it legitimately and `bee worktree register` granted it its own `.bee` store. The hook has no notion of a worktree grant as a second valid boundary for the same session, and a worktree is only actually usable by a genuinely separate process opening that printed path — never by continuing in the same conversation. Recovery is clean (`git worktree remove` + `branch -d` + `worktree unregister`, no partial writes since the hook denies pre-write) but wasted effort. When new feature work needs to start in an occupied checkout from inside a single continuous session (not a separate terminal/process the user will open themselves), skip the worktree — go directly to the main checkout and use `--lane <feature>` from the very first `state set`/`state gate` call instead.

**Full entry:** docs/history/learnings/20260720-pane-agent-status-changed-live-probe.md

## [20260722] A worktree created with plain `git worktree add` (not `bee worktree new`) never gets its own tracked `.bee/` state — every bee.mjs write lands in the main checkout regardless of cwd
**Category:** pattern
**Feature:** pbi-027-visual-viewport-keyboard
**Tags:** [worktree, lane-state, bee-internals, multi-checkout]

Refines the entry directly above, which is about a DIFFERENT failure (write-guard denial). This session created a worktree with plain `git worktree add` + `cd`, then ran an entire exploring→planning→swarming→scribing→compounding pass from inside it in the same continuous conversation — every `bee.mjs state`/`decisions`/`cells`/`backlog` call succeeded, named the correct lane, and no write was ever denied. But the tracked (non-gitignored) `.bee/` files those calls wrote — confirmed for `.bee/lanes/<feature>.json`, `.bee/cells/<id>.json`, AND `.bee/backlog.jsonl` — never landed in the worktree's own `.bee/` at all; `ls`/`git status` in both checkouts confirmed every one was written to the MAIN checkout instead, the entire session, despite cwd being in the worktree for every call. Two independent read-only compounding analysts, searching only the worktree's own `.bee/cells/`, concluded a spec's cell-evidence pointer was broken — it wasn't; they were looking in the wrong physical checkout. Since these files are also never committed on the worktree's own branch (`git log --all` for the lane/cell paths returns nothing), this bookkeeping does not travel with a later `git merge` of the worktree branch — it has to be reconciled from the main checkout separately, and stacks up as uncommitted main-checkout diffs for as long as the worktree session continues. Before trusting a worktree as self-contained for bee's own state, check directly (`git status --short .bee/<path>` in both checkouts) rather than assuming success messages mean the write landed where cwd was.

**Full entry:** docs/history/learnings/20260722-pbi-027-visual-viewport-keyboard.md

## [20260721] Splitting a cell across a changed export's signature and its sole consumer deadlocks under whole-project tsc
**Category:** failure
**Feature:** home-shell-workspaces
**Tags:** [typescript, whole-project-typecheck, cell-scoping]

`web/tsconfig.json`'s `include: ["src","test"]` means `npm run typecheck` checks the entire project on every run, not just the changed file. A plan split frontend work into a data-layer cell (change `fetchAgents()`'s return shape) and a rendering cell (update its sole consumer, `switcher.ts`) — the data-layer cell could never pass its own `tsc` in isolation, since the consumer it was prohibited from touching would type-error against the new shape, and the fix was gated behind that same cell capping. Two independent reviewers (plan-checker, cold-pickup cell reviewer) converged on the identical finding. Before finalizing a multi-cell TS slice in a whole-project-`tsc` package, check whether any cell changes an exported function's type while a different cell owns that function's only call site(s) — if so, merge those cells into one deliverable rather than trying to preserve the split.

**Full entry:** docs/history/learnings/20260721-home-shell-workspaces-typecheck-coupling.md

## [20260722] A route-level `history.replaceState` exception only protects the CURRENT top-of-stack entry, not entries pushed before it
**Category:** failure
**Feature:** switcher-login-url
**Tags:** [frontend, routing, history-api, spa, back-stack]

Adding a route-specific "always `replaceState`, never `pushState`" exception to a router-less SPA's `navigate()` (here: entering/leaving the `login` route, so Back can never re-render the login form to an already-authenticated operator, D7) proves only that the *current* history entry gets overwritten — it cannot reach further back in the stack. Three navigation steps deep (`switcher@0 -> terminal-A@1 -> terminal-B@2`), a logout `replaceState`s only `terminal-B@2 -> login@2`; Back from there pops to `terminal-A@1`, an authenticated-only view, rendered via `popstate` with no re-auth check. The adjacent-entry case (the one a bug report or review actually traces) is not proof at every stack depth. When a "this route must never be reachable via Back" requirement needs the stronger guarantee, gate the *rendering* of every affected route on a fresh session check in the popstate handler itself, not just on the transition that created the risk.

**Full entry:** docs/history/learnings/20260722-switcher-login-url-back-stack-depth.md

## [20260721] Before dispatching a cell to plan-checker, mechanically compare its verify command against its action text
**Category:** pattern
**Feature:** home-shell-workspaces
**Tags:** [verify-authoring, cell-scoping, pre-flight-check]

A cell's verify command required at least 3 passing tests matching an exact name prefix, but the action text never told the worker to write that many tests or use that prefix — a worker following only the action could produce a cell structurally unable to pass its own verify. Caught by the orchestrator itself, via direct string comparison, before the plan-checker subagent was even dispatched. Whenever a verify command asserts a concrete threshold (a count, an exact name/prefix) that only the worker's own new code can satisfy, confirm the action text explicitly commits to that exact threshold first — a cheap, no-domain-knowledge check distinct from plan-checker's structural review, worth running unconditionally on every authored cell.

**Full entry:** docs/history/learnings/20260721-home-shell-workspaces-typecheck-coupling.md

## [20260721] A cross-cell shared type needs one pinned owner before cells are cut, especially when it's a mid-plan fix
**Category:** decision, failure
**Feature:** web-create-sheet
**Tags:** [cell-scoping, plan-checking, cross-cell-types]

A type/contract more than one cell must produce or consume (a DTO, an event shape) needs a single named owner, an exact field list, and a "import, never redefine" prohibition on every consuming cell — decided at planning, before cell boundaries are cut. `web-create-sheet`'s `NewPaneRef` was added as a mid-exploring fix (a plain shell can never produce a full `AgentRow`) and CONTEXT.md correctly deferred its exact shape to planning — but planning cut cells 2/3 without pinning it, so each cell independently described a different field set, and cell 1's response payload wasn't required to expose what cell 3 needed at all. Two independent reviewers (plan-checker, cold-pickup cell review) converged on the identical defect from different evidence — a strong signal this class of gap is real and checkable. Fix: whenever CONTEXT.md defers a cross-cell type's shape to planning, resolve it before writing cells — name it, list its fields, assign one owner, and make every consumer's `must_haves` cite that owner instead of restating the shape.

**Full entry:** docs/history/learnings/20260721-web-create-sheet-type-ownership-and-css-scope.md

## [20260721] UI-adding cells must declare styles.css or state explicitly that no new styling is needed
**Category:** failure
**Feature:** web-create-sheet
**Tags:** [cell-scoping, css-scope, planning-completeness]

Two consecutive cells (a new sheet component, then its FAB wiring) both needed `web/src/styles.css` for their new markup to be visible at all, but neither cell's plan declared it in `files` — a worker caught it at execution time and self-corrected with a transparent, additive-only deviation, citing real precedent (`terminal-reply-ui-1`, `terminal-nav-keys-2` both included `styles.css`). The gap recurred across two cells of the same feature, not once. Before finalizing a plan with any cell whose action adds new rendered markup, either include `styles.css` in its `files` or state explicitly that no new styling is needed — grep prior UI-adding cells' `files` lists for the pairing as a mechanical precedent check (belongs in plan-checker's scope-sanity dimension).

**Full entry:** docs/history/learnings/20260721-web-create-sheet-type-ownership-and-css-scope.md

## [20260721] Sibling API verbs do not share fallback behavior just because they share a param shape
**Category:** decision, failure
**Feature:** web-create-endpoints
**Tags:** [herdr-port, api-symmetry, silent-wrong-repo-start]

The frozen parent plan assumed herdr's `tab.create` and `agent.start` fall back the same way when `cwd` is omitted. Reading the vendored source directly (pre-code validation, not review) proved this false: `tab.create(cwd: None)` resolves the workspace's own anchor (safe, desktop-equivalent); `agent.start(cwd: None)` falls back to the **herdr server process's own directory** — arbitrary, unrelated to the workspace, exactly the silent wrong-repo start a locked decision (D5) exists to forbid. `FakeHerdr` never modeled this fallback, so the bug would have stayed invisible in a green suite forever. Never infer one API verb's fallback/error/edge-case behavior from a sibling verb's — read (or capture) each independently, and treat a fake that only models one of a pair as a signal the pair was never actually compared.

**Full entry:** docs/history/learnings/20260721-web-create-endpoints-asymmetric-cwd-and-validation.md

## [20260721] A downstream job's `needs: <matrix-job>` gates on the matrix's AGGREGATE conclusion, not the one leg it actually depends on
**Category:** failure
**Feature:** macos-installer-runtime-smoke (P1 fix from review-installer-smoke-and-live-probe-20260721b)
**Tags:** [github-actions, ci, matrix, needs, cell-authoring]

`macos-install-smoke: needs: build` (a 3-leg matrix, `fail-fast: false`) was silently SKIPPED — not failed — whenever either unrelated Linux leg failed, even though the macOS leg itself succeeded and published its asset. `fail-fast: false` only controls whether GitHub Actions cancels sibling legs early; it does not change the matrix job's final reported conclusion for `needs:` purposes, which is `failure` if ANY leg failed. Two independent reviewers caught this in the same session; the fix was `needs: build` (unchanged, for ordering) plus `if: ${{ !cancelled() }}` so the job runs regardless of an unrelated leg's outcome — the real downstream failure path (the installer's own asset-download error) still fires correctly if the *specific* leg the job cares about actually failed. Any future job that depends on one specific leg of a multi-leg matrix (e.g. PBI-016's Intel Mac target) needs either a dedicated non-matrixed job for that leg (the existing `release-windows` pattern) or this `if: !cancelled()` + own-failure-detection shape — never a bare `needs: <matrix-job>` when only one leg's outcome actually matters.

**Full entry:** docs/history/learnings/20260721-macos-installer-runtime-smoke-p1-fix.md

## [20260722] A `cargo:rerun-if-changed` line scopes the WHOLE build script's rerun behavior, not just its own feature
**Category:** failure
**Feature:** health-fingerprint
**Tags:** [build-rs, cargo, rerun-if-changed, staleness]

This repo's `build.rs` already had `println!("cargo:rerun-if-changed=static")` (for `RustEmbed`'s `static/` dir). Planning discovery caught that Cargo's rerun model is all-or-nothing per script — any `rerun-if-changed` directive disables the default "always rerun," for the entire script, not just the line that emitted it — so bolting new build-time-computed logic (a version fingerprint) onto this same `build.rs` without addressing that line would have made the new logic go stale on any rebuild that didn't touch `static/`. Before adding logic to a `build.rs` that already emits `rerun-if-changed`/`rerun-if-env-changed`, check whether the existing directive's scope covers the new logic's own staleness needs — never assume it "only affects its own feature." Full entry: `docs/history/learnings/20260722-health-fingerprint-build-rs-staleness.md`.

## [20260722] Prove environment/git-state-dependent cell behavior in an isolated throwaway harness, never the shared worktree
**Category:** pattern
**Feature:** health-fingerprint
**Tags:** [multi-session, worktree, verification, git]

A cell needing to prove dirty-vs-clean git behavior and a missing-`git`-binary fallback couldn't safely toggle the actual worktree's state — it was already dirty from other concurrent bee sessions' unrelated work, and mutating it further risked disturbing them. The worker instead built an isolated throwaway git repo and a scratch `rustc`-compiled harness (PATH overridden only for the child `git` lookup) under `.bee/tmp/<feature>/`, proved both behaviors there, deleted the scratch artifacts, and recorded the substitution explicitly in `verification_evidence` rather than skipping the proof silently. When a cell's proof needs mutable environment/git state and the worktree may be shared (the routine case in this repo — see the multi-session lane/gate entries above), build a disposable isolated reproduction instead of toggling the shared tree, and always record that substitution explicitly rather than silently dropping the proof. Full entry: `docs/history/learnings/20260722-health-fingerprint-build-rs-staleness.md`.

## [20260722] `std::env::set_var`/`remove_var` in `#[test]` fns race across FILES, not just within one — serialize by shared key, not by file
**Category:** failure
**Feature:** flaky-secrets-env-test
**Tags:** [rust-tests, env-var-race, parallel-tests]

PBI-039: `config::tests::secrets_absent_from_env_and_file_are_none` flaked ~1/12 full-suite runs. `std::env::set_var`/`remove_var` mutate process-wide state, but Rust's default test runner executes `#[test]` fns in parallel threads — and three separate files (`src/config/mod.rs`, `src/config/secrets.rs`, `src/doctor/checks.rs`) each had tests mutating the SAME env keys (`HERDR_GO_GITHUB_TOKEN`, `HERDR_GO_WEB_SECRET`, `HERDR_GO_TELEGRAM_TOKEN`, `XDG_DATA_HOME`) with zero synchronization between them. Fixed with one shared `#[cfg(test)] pub(crate) static ENV_TEST_LOCK: std::sync::Mutex<()>` in `src/config/mod.rs`, acquired poison-tolerantly as the first statement in every test touching these keys, never `--test-threads=1` (which would serialize the whole suite instead of just the racing tests). Before adding any new test that calls `std::env::set_var`/`remove_var` on a process-wide key already touched elsewhere in the crate, grep for existing writers/readers of that same key across ALL files (not just the one being edited) and acquire `crate::config::ENV_TEST_LOCK` — a same-file review misses the race entirely when the racing sibling lives in a different module. Full entry: `docs/history/learnings/20260722-flaky-secrets-env-test.md`.

## [20260722] A slice needs one verify that runs the assembled thing, not only its own file
**Category:** failure
**Feature:** agent-pane-orchestration
**Tags:** [verification-scope, slice-completeness, false-green]
Three pieces of this feature were named in one cell's prose as "out of scope for this cell" and then owned by no later cell: the lane classifier, the real spawn proof, and the loop's own prompt file. Every cell verify stayed green throughout, precisely because each was scoped to its own artifact — byte-identity, required strings, D-ids cited — so nothing ever asked whether the assembled pieces could execute one cycle. The missing prompt file shipped: started as delivered, the cockpit would have printed "prompt file not found" every 60 seconds and, per its own correct no-exit-on-error property, continued forever accomplishing nothing. Any slice that produces a runnable thing needs at least one cell whose verify RUNS it — here, a stub binary on PATH plus one real `--max-iterations 1` iteration, which proves resolution and wiring that no `test -s` can. Treat "not created here — out of scope" in a cell action as a promise to a cell that may not exist; create the piece or create its owner in the same breath.
**Full entry:** docs/history/learnings/20260722-agent-pane-orchestration.md

## [20260722] Inside a polling loop, "skip for now" is not a stop — it is a retry with a delay
**Category:** failure
**Feature:** agent-pane-orchestration
**Tags:** [loop-semantics, safety-gate, flaky-signal]
The merge role's first draft said a red-verify worktree is not re-attempted "this iteration" and, one sentence later, that "a later iteration will find it still finished and eligible". In a 60-second loop those are the same sentence. With this repo's measured 1-in-12 verify flake (PBI-039), the loop would have merged within ~12 minutes exactly what the red result existed to prevent — dissolving the only semantic gate a merge has. Any "skip for now" inside a poll must anchor to a durable signal, never to the iteration boundary: the boundary is not a unit of time, it is a unit of forgetting. Fix used the same no-state-file mechanism the design was already forced into for anomaly de-duplication — read the chat pane's own scrollback for the prior report and stay away while it stands.
**Full entry:** docs/history/learnings/20260722-agent-pane-orchestration.md

## [20260722] Instructions are code: run one of a document's own commands before calling it verified
**Category:** failure
**Feature:** agent-pane-orchestration
**Tags:** [verification-evidence, external-cli, doc-as-code]
A skill file passed `--json` to `herdr pane split` (rejected: `unknown option`) and told the dispatcher to split a pane then start an agent — when live, `herdr agent start` opens its OWN pane and never attaches to the split one, leaking a stray pane per dispatch until the concurrency cap fills with ghosts. Both shipped green, because the cell's verify grepped the document for its own content. Both were caught by the NEXT cell's worker: one by running `herdr --help` before assuming a flag, the other by the single cell that executed the sequence for real. This extends the `default-agent-presets` pattern from decisions to documents — when a doc hardcodes an external CLI's invocation, at least one of those invocations must be executed against the real binary before the doc is done.
**Full entry:** docs/history/learnings/20260722-agent-pane-orchestration.md

## [20260722] A test-runner filter passing is not proof of work done — verify file-exists + module-wired + named-test-function-exists, not just "some test passed"
**Category:** failure
**Feature:** self-update-merge-config
**Tags:** [verify-quality, tautological-verify, testing]

Across three separate epics of one feature, a cell's `verify` (shaped `cargo test --quiet <filter>`) passed against the UNMODIFIED repo before any work was done: once because an unrelated pre-existing step already satisfied the grep, once because a new module wasn't yet declared in the crate tree (so "0 tests filtered" trivially exits 0), and once because a worker could satisfy "some test passed" with happy-path-only tests while silently skipping the fail-closed branches the cell existed to prove. Fix pattern used from then on: `test -f <file> && grep -q 'mod <name>' <parent> && grep -q 'fn <exact_test_name>' <file>` (one grep per required behavior) `&& cargo test --quiet <filter>` — and always run the verify against the unmodified repo during validating to confirm it fails first.

**Full entry:** docs/history/learnings/20260722-self-update-merge-config.md

## [20260722] A cell touching `.rs` files needs its own fmt check — wave-close is not a substitute
**Category:** pattern
**Feature:** seed-agent-presets-legacy-config
**Tags:** [verify-commands, cell-authoring, rustfmt]

A cell's own scoped `verify` (grep + `cargo test --quiet <filter>`) passed clean, but the orchestrator's independent wave-close run of the full `commands.verify` chain failed on `cargo fmt --all --check` — the worker's new code was correct but never run through the formatter before capping. This exact gap has now recurred across 4+ features (`doctor-config-surface`, `self-update-merge-config` x3, this one — see fmt-fix commits `9520478`, `5edb797`, `fa61868`, `118bb9e`). A prior write-up (`self-update-merge-config`'s learnings) frames "wave-close catches it, fix via a small cleanup cell" as accepted practice — but a tiny 1-cell feature has no later cell to bundle a cleanup into, so this occurrence's fix landed as an untracked orchestrator-side edit with nothing recorded in `trace.friction`. Any cell whose `files` touches a `.rs` path should include a formatting check (`cargo fmt --all --check` or a path-scoped variant) in its own `verify`, not rely solely on wave-close to catch it — and if wave-close still catches something a cell's verify missed, record the fix in that cell's `trace.friction`, not just a decision log line that reads as a clean pass.

**Full entry:** docs/history/learnings/20260722-seed-agent-presets-legacy-config.md

## [20260722] A citation copied from an existing doc is not pre-verified — re-check it, and re-check your own after implementation
**Category:** pattern
**Feature:** seed-agent-presets-legacy-config
**Tags:** [citation-freshness, exploring-discipline, context-md]

Two distinct causes produced 4 stale `file:line` citations in one CONTEXT.md, caught by exploring's fresh-eyes review. (1) `install.sh:151` was wrong from the moment the citing backlog row (PBI-045) was first written and was carried unverified through a later renumbering — a genuine copy-without-verify (this repo already hit the same failure mode once before, commit `bc68aa1`, "fix stale D3 citation in agent-pane-orchestration CONTEXT.md"). (2) `src/doctor/checks.rs:902`/`:484` were *correct when CONTEXT.md was written*, then went stale because this same feature's own implementation inserted ~153 lines above those functions — nothing re-verifies CONTEXT.md's own citations after implementation lands. Before re-citing a `file:line` from an existing doc (a backlog row, a prior CONTEXT.md/spec), open the real file at that line and confirm it still matches — never trust an existing citation as pre-verified. Separately, for any citation into a file THIS feature's own cells are about to edit, treat the line number as a pre-implementation snapshot: prefer a function/symbol name anchor over a bare line number for in-scope files, or do one final numeric-citation sweep against the post-implementation tree (fresh-eyes review or scribing, both of which already read post-implementation state, are the natural place).

**Full entry:** docs/history/learnings/20260722-seed-agent-presets-legacy-config.md

## [20260722] A lib-crate module cannot self-reference via the crate's own external name (`herdr_go::` vs `crate::`)
**Category:** failure
**Feature:** self-update-merge-config
**Tags:** [crate-boundaries, rust, cell-authoring]

A cell targeting `src/update/rollout.rs` (inside the `herdr_go` lib crate) was drafted with `herdr_go::doctor::...`-style paths — valid only from `main.rs` (a separate binary crate) or an external consumer, never from code that is itself part of the lib. Caught by validating's feasibility check before dispatch; recurred harmlessly once more when a worker's auto-fix corrected a stray `herdr_go::` in a doc *comment* tripping the same guard. When a cell's target file lives inside the library crate (check: does `lib.rs` declare it as a `pub mod`?), verify every cross-module reference uses `crate::`, and add a negative-grep (`! grep -q 'herdr_go::' <file>`) to that cell's verify command.

**Full entry:** docs/history/learnings/20260722-self-update-merge-config.md

## [20260722] This installation's `bee-gather`/`bee-extract` agent types are read-only — never dispatch them for cell execution
**Category:** failure
**Feature:** dedupe-default-config-templates
**Tags:** [bee-swarming, subagent-type, tooling]

`bee-swarming`'s spawn-type table ("`bee-gather` for generation, `bee-extract` for extraction") is written for cell EXECUTION dispatch, but in this installation those two rendered agents carry only `Read, Grep, Glob` (the Delegation contract's read-only I/O-offload gather role) — no `Bash`/`Write`/`Edit`. The first wave dispatched this way returned `[BLOCKED]` on both cells immediately (no reservation, no write tool), costing a full re-dispatch round-trip under `subagent_type: "claude"` before any work happened. Before the first wave of any swarming session, confirm whether `bee-gather`/`bee-extract` carry write tools in the current installation; if read-only (as here), dispatch generation/extraction-tier EXECUTION cells under a full-tool type (`"claude"`/`"general-purpose"`) instead, reserving `bee-gather`/`bee-extract`/`bee-review` for actual read-only gathers (plan-checker, cell-reviewer, orient reads) only.

**Full entry:** docs/history/learnings/20260722-dedupe-default-config-templates.md

## [20260723] A "regression" was a 5-day-old CSS cascade-origin bug: author `display:flex` always beats UA `[hidden]{display:none}`
**Category:** pattern
**Feature:** pbi-054-switcher-group-collapse-regression
**Tags:** [css-cascade, hidden-attribute, regression-triage]

The switcher's workspace-group collapse/expand was blamed on same-day PBI-052 by recency alone; a 2-command git audit (`git show <suspect-commit>` + `git log -G'<cause-pattern>' -- <file>`) proved the real cause predates it by 5 days: `.workspace-rows` shares `.agent-list`'s `display:flex`, an author-origin rule that always outranks the UA `[hidden]{display:none}` default regardless of specificity — so the collapsible-grouping feature never actually collapsed visually since its introduction. This is the 5th instance of the identical bug class in `web/src/styles.css` (`.reply-sheet`, `.keys-pad`, `.create-sheet`, `.dropdown-popup` were fixed the same way before). Rule: any element toggled via the `hidden` attribute that also carries its own `display` rule needs an explicit `<selector>[hidden] { display: none; }` override, added at the same time the element gains the `display` rule — not after a user reports it broken. Before writing a "regression from feature X" claim, run the two-command git audit first; don't blame by recency.

**Full entry:** docs/history/learnings/20260723-switcher-collapse-cascade-bug.md

## [20260722] A literal `$` in a `grep` verify pattern can false-negative under an interactive-shell grep wrapper
**Category:** failure
**Feature:** dedupe-default-config-templates
**Tags:** [grep, shell-environment, verify-quality]

A cell's verify command grepped for a literal `UTF8Encoding($false)` in a PowerShell file. Both the cell's own worker and, independently, the orchestrator's later goal-check re-run hit a false negative on the identical pattern — this session's interactive zsh aliases `grep` to a wrapper (`ugrep -G` via the Claude Code CLI's smart-grep) that mishandles a literal `$` mid-pattern, while plain `/usr/bin/grep`, `grep -F`, or any non-interactive `sh -c` invocation match correctly. The worker's workaround (`sh -c "..."`) was buried in a prose evidence field the orchestrator's goal-check didn't read before re-running the command, so the same problem was independently rediscovered. Any verify/goal-check command grepping a literal `$` should escape it (`\$`) or use `grep -F`; when a goal-check re-run of a cell's exact verify command fails unexpectedly, try `sh -c "<command>"` before concluding the underlying change is wrong.

**Full entry:** docs/history/learnings/20260722-dedupe-default-config-templates.md

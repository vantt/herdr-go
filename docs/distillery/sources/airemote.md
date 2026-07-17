---
name: airemote
type: git-repo
url: https://github.com/thanhsmind/airemote
local: upstreams/airemote
last_analyzed_commit: 5667667
last_analyzed_date: 2026-07-17
domains_covered: [harness, orchestration, safety, context-memory, workflow, config-packaging, testing-evals, docs-style, quality-gates, repo-layout, skills, hooks, planning, tooling, self-improvement, ux]
---

# airemote (AgentBridge) — Feature Index

> Extracted from HEAD `5667667` on 2026-07-17. Clone: `upstreams/airemote`. Go project ("AgentBridge") — a Telegram bot that drives Codex/Claude Code coding-agent sessions running **inside `herdr`**, using herdr's CLI/socket API as the sole execution substrate. Directly relevant: this is a real, dogfooded implementation of "herdr as agent orchestrator." Full inventory reports: `docs/distillery/reports/distill-airemote-inventory-safety-config-2026-07-17.md`, `distill-airemote-inventory-herdr-sessions-2026-07-17.md`, `distill-airemote-inventory-telegram-persistence-2026-07-17.md`.

## harness — driving herdr programmatically

### session-isolation-dedicated-session
- **What:** Every runtime command is forced to name an explicit herdr `--session` so all of the service's workspaces/checkouts/agents live in one dedicated named session, never the operator's own interactive session.
- **Where:** `internal/herdr/cli_client.go` (`New`, `command()`, `run()` all refuse a session-less invocation), `internal/herdr/testdata/fakeherdr/main.go`
- **Notable:** Discovered live that `HERDR_SESSION=<name>` (the env var herdr's own spec says scopes commands) is **silently ignored** by herdr 0.7.3 — only `--session` actually isolates. Following the spec literally caused every workspace to be created inside the operator's own live session during the spike itself. Enforced by 4 independent layers so no single caller can "forget."
- **Keywords:** D8, `--session` vs `HERDR_SESSION`, silent-default-session hazard

### sending-is-not-submitting
- **What:** `herdr agent send`/typing text into an agent's input box does not submit it; submission requires a separate keystroke, sent only after polling confirms the typed text landed on screen.
- **Where:** `internal/herdr/cli_client.go` (`SendAgent`, `confirmLanded`), `internal/herdr/client.go` (`SendRequest{Target,PaneID,Text}`)
- **Notable:** Mandatory 3-step sequence: type → poll visible screen (ANSI-stripped) every 100ms for the sent text's tail, up to a 3s deadline → only then send the submit key. Sending Enter too early loses the submission entirely (observed twice). An expired confirmation is a typed failure, never reported as delivered.
- **Keywords:** D9, send-confirm-submit, ErrSendNotConfirmed

### default-deny-readiness-composer-match
- **What:** A session is judged "ready" only on a positive, shape-based match of the agent's composer screen (single non-enumerated input line, no activity indicator) — never on herdr's `idle` status alone, and never by recognizing known-bad menus.
- **Where:** `internal/agents/adapter.go` (`ComposerMatcher`), `internal/sessions/readiness.go` (`awaitReady`, `enterNeedsAttention`)
- **Notable:** A menu shares the same leading glyph as the composer, so an earlier "glyph present" rule called a **blocked** agent ready — a submit keystroke into that misjudged screen once selected a menu option that ran a pipe-to-shell installer, live. Fixed by matching screen *shape* (default-deny) instead of enumerating known menus, since "every release can add one." Verified against real captured screens plus 10 synthetic unseen-shape fixtures that must all still deny.
- **Keywords:** D43, D25, D24, D22, D10, needs_attention

### per-agent-launch-adapters
- **What:** One `AgentAdapter` implementation per coding agent (Codex, Claude) hides differences in launch command, composer glyph, and cancel keys behind a shared interface.
- **Where:** `internal/agents/adapter.go` (interface), `codex.go`, `claude.go`
- **Notable:** Codex composer glyph `›` (single line, nothing enumerated); Claude `❯` inside a rule-delimited box (an older "glyph anywhere" form wrongly accepted a boxed menu and was tightened). Only Codex has a directory-trust prompt; Claude has a different first-run prompt that is *not* pre-empted, only parked.
- **Keywords:** D25, D44, AgentAdapter, composer glyph

### directory-trust-flag-preemption
- **What:** A per-launch CLI flag pre-clears Codex's first-run "trust this directory?" prompt so a normal launch reaches the composer directly, instead of stopping on the prompt.
- **Where:** `internal/agents/codex.go` (`codexTrustConfig`, builds `projects={"<repo-root>"={trust_level="trusted"}}`)
- **Notable:** Two facts established by experiment, not docs: Codex resolves trust at the **repository** root, not the linked worktree checkout (so naming the checkout pre-empts nothing); and Codex's `-c` flag splits its *name* half on every separator with no quote-awareness, so the path must live in the *value* half. An earlier mechanism (D23/D31) was shipped and never actually worked — D44 replaced it, proven by exec'ing the real argv against a fresh never-trusted repo.
- **Keywords:** D23, D44, D31, value-half vs name-half, TOML `-c` flag

### idle-status-ambiguity
- **What:** herdr's `idle` status is reported both for a genuinely ready agent and for one blocked at an unrecognized first-run prompt — the status alone carries no readiness information.
- **Where:** underlies `internal/sessions/readiness.go` (readiness ignores agent status, reads only the pane)
- **Notable:** Sending an instruction while an agent waited on such a prompt was observed to **kill the agent**. One of three headline safety findings alongside session isolation (D8) and send-not-submit (D9).
- **Keywords:** D10, blocked-vs-finished ambiguity

### no-event-subscription-status-polling
- **What:** herdr exposes a push-event channel; the client deliberately does not use it yet — status is obtained by polling only (500ms interval), and `Subscribe` returns an explicit "not implemented" so nothing can silently come to depend on undelivered events.
- **Where:** `internal/herdr/errors.go` (`ErrNotImplemented`), `internal/sessions/manager.go` (`defaultPollInterval`)
- **Notable:** Separately observed live: `pane.agent_status_changed` fires **twice in the same millisecond** for one transition — any future event consumer must de-duplicate or double-notify a human.
- **Keywords:** D13, ErrNotImplemented, duplicate events, event-subscriber deferred to M5

### protocol-version-compatibility-check
- **What:** Compatibility is pinned to herdr's wire **protocol number** (16), checked at startup — never the human-readable marketing version string, which can move independently.
- **Where:** `internal/version/version.go` (`HerdrProtocol = 16`, test fails if it drifts without re-running discovery)
- **Notable:** A mismatch is a typed, reportable error, never a silent "probably fine."
- **Keywords:** D5, wire protocol vs marketing version

### ansi-stripping-scanner
- **What:** A purpose-built scanner (not a regex) strips ANSI/terminal control codes from pane output before comparing/matching it.
- **Where:** `internal/herdr/snapshot.go` (`StripANSI`)
- **Notable:** Built as a scanner specifically because "OSC sequences have two legal terminators" — a detail a naive regex mishandles.
- **Keywords:** StripANSI, OSC terminators

### sandbox-safe-launch-flags-only
- **What:** Each per-agent adapter can only select one of two safe sandbox levels (write-enabled or read-only) when building a launch command; a sandbox-disabling value cannot be produced by any input path.
- **Where:** `internal/agents/codex.go`, `internal/agents/claude.go`
- **Notable:** The disabling values are not merely guarded against — they are written **nowhere** in the adapter code at all, verified by an independent probe with a hostile input.
- **Keywords:** D18, read-only vs write-enabled, no-escape-hatch-in-code

## orchestration — session lifecycle & multi-agent coordination

### session-naming-ab-id-scheme
- **What:** Every task gets a canonical `ab-YYYYMMDD-NNN` id (local timezone), reused as branch name, worktree directory, durable-record key, and the runtime agent target — one string, four uses.
- **Where:** `internal/sessions/id.go` (`Generator`, `maxSeq=999`)
- **Notable:** The within-day counter is read from the **durable record's** max-so-far, never an in-process counter, so a mid-day restart never reissues a number — and minting skips forward past any on-disk branch/directory collision, because cleanup deliberately lets artifacts outlive their session. >999/day is a loud error, never silent rollover.
- **Keywords:** D20, collision-skip, durable-counter

### pre-minted-id-ahead-of-create
- **What:** The id-minting step is exposed standalone so a caller (chat gateway) can mint an id — with no worktree cut yet — to name a chat topic before the task record exists.
- **Where:** `internal/sessions/manager.go` (`NextID`, `CreateWithID`)
- **Notable:** "Minting an id does not reserve it" — two simultaneous mints of the same counter are resolved later, at the write-first durable-record step, which refuses the duplicate before any agent starts.
- **Keywords:** D62, D65, NextID

### keystroke-state-guard
- **What:** Any operation sending a keystroke (prompt, cancel) is refused unless the session's current **state** is `ready` — checked independently of call ordering.
- **Where:** `internal/sessions/state.go` (`requireReady`, `ErrNotReady`)
- **Notable:** "Sequencing is not a guard; state is" — protects a session parked in `needs_attention` from ever receiving a keystroke, regardless of which future caller sends one.
- **Keywords:** D28, requireReady, needs_attention

### two-step-cancellation
- **What:** Cancel sends one interrupt (stops work, pane stays alive); after a grace period, a second interrupt exits the agent and closes the pane. No forcible kill anywhere.
- **Where:** `internal/sessions/control.go` (`Cancel`)
- **Notable:** Verified live against a real agent to match this exactly; refused outright on any non-`ready` session by the keystroke state guard.
- **Keywords:** cancel grace, two-stage interrupt

### durable-session-record-write-first
- **What:** A session's durable record is written **before** the checkout is cut and the agent started; a write failure aborts creation entirely — no directory, no checkout, no agent left behind.
- **Where:** `internal/sessions/create.go` (`Create`)
- **Notable:** Makes "a live agent the record doesn't know about" structurally impossible. A *later* persistence failure (after the agent is live) never kills the agent — the record merely falls behind, recoverable via identity keys that don't go stale.
- **Keywords:** D35, write-first, never-a-phantom-agent

### restart-recovery-single-snapshot-evidence
- **What:** On restart, recovery takes exactly **one** snapshot of everything herdr currently has alive and decides every stored session against that single picture — never re-asking task by task.
- **Where:** `internal/sessions/recovery.go` (functions `Reconcile`, `Recover` on the session manager)
- **Notable:** Treats "I cannot reach the runtime" and "the runtime has nothing alive" as structurally distinct — only a *successful* snapshot may support "gone." A failed snapshot retries with backoff up to 30s before reporting the runtime unavailable. Recovery is proven fully read-only (closes nothing, sends no keystroke).
- **Keywords:** D34, D36, ErrRuntimeUnavailable, snapshot-once

### orphaned-session-state
- **What:** A dedicated `orphaned` state — set only during recovery — means the stored session's terminal is genuinely gone but its checkout/work survives on disk.
- **Where:** `internal/sessions/state.go` (`StateOrphaned`)
- **Notable:** Deliberately **not** `failed` — conflating the two would license deleting real unfinished work. The obligation to notify the owner is recorded as its own durable event (not a log line) and delivered at-least-once by the chat gateway at next startup.
- **Keywords:** D38, D40, D47, orphaned-not-failed

### recovery-non-orphan-non-match-outcomes
- **What:** Four recovery outcomes exist that are neither a clean match nor `orphaned`, each keeping the session fully out of service: stored path no longer resolves inside an allowed root; record with no runtime target at all (crash mid-create); record naming a *different* configured runtime session (concluded nothing, to avoid mass-orphaning); terminal exists but no agent runs in it.
- **Where:** `internal/sessions/recovery.go`
- **Notable:** Reasoning per case is explicit and asymmetric — e.g. a phantom record is `failed` not `orphaned`, because "orphaned" would license deleting work that never existed.
- **Keywords:** D40, refused-path, foreign-session, never-started

### runtime-identity-match-keys
- **What:** A stored session re-attaches to a live agent after restart via exactly four values, split into two roles: **identity keys** (agent target, native session id) that can refute a match; **locator keys** (terminal id, workspace id) that only corroborate and are refreshed from whatever the runtime currently reports.
- **Where:** `internal/sessions/recovery.go`, `internal/sessions/record.go`
- **Notable:** Branch is **never** a match key at all — herdr's own picture of what's alive carries no branch information whatsoever (verified against `snapshot.go`). A stale locator key never refutes a live agent still carrying the right identity key, "because the record is allowed to lag the runtime by design."
- **Keywords:** D37, D39, identity-vs-locator

### worktree-double-validated-target-directory
- **What:** The agent's working directory is validated twice: once via a no-follow create at the service's own choosing, again on whatever directory herdr's checkout tool actually reports back after cutting the worktree.
- **Where:** `internal/sessions/create.go` (guard one + guard two, `RemoveWorktree` teardown on escape)
- **Notable:** Necessary because herdr's checkout tool follows a symlink at a target/ancestor path component even though the service's own no-follow creation does not — verified live on the host.
- **Keywords:** D16, D27, D29, guard-one/guard-two

### agent-outlives-service
- **What:** The coding agent lives inside herdr, not inside the AgentBridge service — killing the service never disturbs a running agent; recovery re-attaches from a single snapshot with zero keystrokes.
- **Where:** `internal/sessions/recovery.go`
- **Notable:** Proven with a real two-process experiment: an agent survived its gateway being `kill -9`'d; a fresh service process found the exact same session, pane diff **byte-identical**. What herdr's *own* restart does to live agents remains explicitly unprobed — recorded as an open risk, not assumed away.
- **Keywords:** D34, kill-9-proof, unprobed-herdr-restart

### per-task-agent-and-isolation-choice
- **What:** Both which coding agent (Codex/Claude) and the isolation mode (read-only vs worktree-write) are per-task choices carried in the create request — not fixed per service — but isolation mode can only **narrow** the target project's configured permission, never widen it.
- **Where:** `internal/sessions/manager.go` (fields Config.Adapters, req.AgentKind); reconciled before create is called
- **Notable:** Adding per-task agent dispatch (M3b) was implemented as a strictly additive change — the three existing call sites setting one fixed adapter still compile and pass unchanged.
- **Keywords:** D63, D58, D60, narrow-never-widen

### single-service-durable-record-lock
- **What:** At startup a service takes exclusive ownership of its durable record (`flock`) and refuses to start if another live instance already holds it.
- **Where:** `internal/store/sqlite/lock.go` (`LockDir`)
- **Notable:** Not tidiness — two services over one record would both re-attach to the same live agents, so one service's cancel could interrupt the *other's* working agent. The keystroke state guard (D28) can't help here since it knows session state, not process ownership. A crash releases the lock with no cleanup step.
- **Keywords:** D41, flock, one-service-per-record

## safety — security boundary

### path-allowlist-validation
- **What:** A single ordered 7-step validator is the one check every agent-reachable path passes through: reject non-absolute → reject raw traversal components → deny-list check on unresolved path → resolve all symlinks → deny-list check **again** on resolved path → containment check component-by-component (never text-prefix) → deny by default.
- **Where:** `internal/security/paths.go`
- **Notable:** The ordering is the contract, not an implementation detail — e.g. checking the deny list before *and* after symlink resolution specifically catches a link planted inside an allowed root that points at denied territory; component-wise containment specifically defeats a sibling-prefix trick (`…/projects-evil` vs `…/projects`).
- **Keywords:** deny-then-resolve-then-deny-again, containment-by-component, fail-closed

### hard-deny-list
- **What:** A fixed, non-configurable deny list (filesystem root, `/etc`, `/root`, `/var/lib`, mounted foreign filesystems, `~/.ssh`, `~/.aws`, `~/.config`) always applies regardless of what an operator configures.
- **Where:** `internal/security/paths.go` (`deniedAbsoluteSubtrees`, `deniedHomeSubtrees`)
- **Notable:** Self-flagged as **known-incomplete** (signing keys, cluster/container-registry credentials, shell history are absent) — widening it is deliberately left an operator decision, not an implementer default. Boundary construction itself refuses to start if any allowed root sits on/inside this list.
- **Keywords:** D6, D17, un-widenable, honest-incompleteness

### slug-sanitization
- **What:** Untrusted chat text becomes a branch/directory-safe slug via an **allowlist** charset (lowercase, digits, hyphen) operating on raw bytes, not decoded characters.
- **Where:** `internal/security/slug.go` (`SanitizeBranchSlug`, `SanitizeSessionSlug`)
- **Notable:** Byte-level operation means "a foreign character, a malformed byte, and an over-long slash encoding all collapse the same way — no decoding step for an attacker to disagree with." An empty result (nothing survives) is an **error**, never a silent fallback to raw input.
- **Keywords:** allowlist-not-blocklist, byte-level, empty-slug-is-error

### branch-slug-last-gate-refusal
- **What:** A worktree-creation request's branch name is accepted only if it exactly matches what the sanitizer itself would produce — checked again at the last gate before the value leaves the process, independent of any caller.
- **Where:** `internal/herdr/cli_client.go` (`CreateWorktree`, `validateBranch`)
- **Notable:** Without this gate, a task titled `../../../.ssh` would be created **verbatim** as a directory by herdr, "and nothing else in the system would notice." Live-verified refusal against the real runtime.
- **Keywords:** D12, last-gate-independent-of-caller

### secret-redaction
- **What:** One shared redactor sits in front of every outbound chat message and every external command's failure output, replacing secret-shaped matches with a labeled placeholder naming the secret class.
- **Where:** `internal/security/redact.go` (`DefaultRedactor`, `Redact`)
- **Notable:** Idempotent by construction (its own placeholders match none of its patterns). Deliberately tuned against over-redaction — the bare word "password" in prose is untouched, only an assignment is — "redacting so aggressively that prose is mangled makes operators turn it off." Exactly one implementation exists system-wide by design. An earlier version missed a credential embedded in a URL; a test caught it and that shape is now matched.
- **Keywords:** single-redactor, idempotent, best-effort-by-design

### safe-create-point-of-use
- **What:** Closes the TOCTOU gap for a not-yet-existing agent-reachable directory: a no-follow create, then a full re-validation of the whole created path, then teardown on escape.
- **Where:** `internal/security/create.go` (`SafeCreateDir`)
- **Notable:** Directly motivated by a verified fact: herdr's own checkout tool *follows a symlink at its target* — so herdr cannot be trusted to refuse the swap either; protection has to happen at creation time, by the service itself. Explicitly **not atomic** (Go stdlib lacks `openat2`/`RESOLVE_BENEATH`) — the accepted residual risk is a race yielding repeated refusals (denial of service), never an escape.
- **Keywords:** D16, D27, D29, D30, TOCTOU, not-atomic-by-design

### sandbox-escape-hatch-refusal
- **What:** Three settings that would defeat the agent sandbox (`danger-full-access`, `bypassPermissions`, `capabilities.sudo: true`) are refused **at config load**, not merely discouraged.
- **Where:** `internal/projects/validation.go` (`refusedPermissions`)
- **Notable:** "The original specification only said do not use them" — the implementation goes further and makes them load-time errors, deliberately hard to reverse: a later milestone that genuinely needs full access must add the value back deliberately and confront why it was refused.
- **Keywords:** D18, refused-not-discouraged

### env-var-blocklist-known-weak
- **What:** 17 named process-hijacking environment variables are refused when an operator tries to pin them for an agent.
- **Where:** `internal/projects/validation.go` (`deniedEnvNames`)
- **Notable:** Self-flagged in the design doc as the **wrong shape**: "a blocklist can only refuse what somebody thought of" — named gaps include source-control index overrides, other interpreters' option-hooks, and the coding agents' own home/config vars. Filed as backlog rather than silently shipped as sound.
- **Keywords:** D19, self-raised-gap, blocklist-known-incomplete

### not-a-sandbox-honest-limit
- **What:** The security-boundary design doc states its own limit up front: this mechanism is not a sandbox — it only governs which paths the *service* hands to the runtime; once an agent is running, nothing here stops it from typing any command the login account can run.
- **Where:** `docs/specs/security-boundary.md` (Purpose, Open Gaps)
- **Notable:** Everything (service, herdr, both coding agents) runs as the machine owner's own admin-capable account — restoring a separate restricted account is an open backlog item the doc says the security posture "cannot honestly be signed off without."
- **Keywords:** D1, honest-limit-statement, no-separate-service-account

### auth-gate-fail-closed-silent
- **What:** Every inbound Telegram event is accepted only from one configured group AND a permitted sender; everything else — wrong group, unlisted sender, unidentifiable sender — is dropped with **zero reply** and one content-free durable audit record.
- **Where:** `internal/telegram/auth.go` (method Authorized on Gate)
- **Notable:** "Silence is the point — replying would confirm a bot is there and leak the project list's existence." A button press is authenticated by its own presser independently of whoever opened the menu.
- **Keywords:** D46, D56, total-silence, no-leak

## context-memory — durable state

### three-table-durable-schema
- **What:** The durable record is three core tables (`sessions` one row per task, `session_events` append-only history, `audit_logs` append-only refusals) plus a small `telegram_state` key-value row and a migrations-tracking table.
- **Where:** `migrations/0001_init.sql`, `migrations/0002_telegram.sql`
- **Notable:** `0002` is strictly additive (no `0001` column touched) and applies inside the same single migration transaction. Schema is asserted in tests against the live database column-by-column, not against migration source text.
- **Keywords:** sessions, session_events, audit_logs, telegram_state

### migrations-all-or-nothing-repeatable
- **What:** Every pending migration for a run applies inside one single transaction (not one per file); a failed run on a fresh record leaves it completely empty, and on an already-current record leaves it byte-identical.
- **Where:** `internal/store/sqlite/migrations.go`
- **Notable:** "A half-migrated record is never an observable state." Migrations are embedded in the binary (`//go:embed *.sql`), not shipped as separate operator-managed files.
- **Keywords:** all-or-nothing, single-transaction, embedded-migrations

### never-store-output-or-credentials
- **What:** The durable record never stores a line of terminal output (only a hash of the last output, to detect a changed screen) and never stores any credential.
- **Where:** `internal/store/store.go` (`last_output_hash`)
- **Notable:** Pairs with a config toggle (`store_initial_prompt`, default true) that is the *only* way to keep a pasted credential in a chat message out of the record, since the record's shape is otherwise frozen.
- **Keywords:** hash-only, no-credential-storage, store_initial_prompt

### deferred-obligation-as-durable-event
- **What:** When recovery finds a task orphaned, the obligation to tell its owner is appended as a durable event (not a log line) with a `delivered_at` marker that starts empty and is set only after a successful send.
- **Where:** `internal/store/store.go` (`AppendSessionEvent`), `internal/store/sqlite/telegram.go` (`ListUndeliveredOrphanNotifications`)
- **Notable:** "A log line does not survive the next restart, and an obligation a restart can forget is not an obligation." Delivery is at-least-once: send first, record second, so a crash between the two resends rather than loses it.
- **Keywords:** D38, D47, D57, event-keyed-delivery, at-least-once

### sqlite-store-implementation-choices
- **What:** The store uses the pure-Go `modernc.org/sqlite` driver (no cgo), WAL journal mode, with a busy timeout set in the connection DSN so it reaches every pool connection.
- **Where:** `internal/store/sqlite/store.go`
- **Notable:** Proven under `-race` with many simultaneous writers; the design doc explicitly warns not to bump the driver version without re-running that proof.
- **Keywords:** modernc.org/sqlite, WAL, busy-timeout, race-proven

### runtime-identity-filled-in-as-known
- **What:** The four runtime match keys don't exist when a task row is first written; they're filled in via in-place updates the instant each becomes known, where an update field left unset (`nil`) means "leave unchanged," never "blank."
- **Where:** `internal/store/store.go` (`UpdateSessionRuntime`)
- **Notable:** "Updating only the state would leave match keys empty forever, and a restart would then orphan every live task while everything else looks perfectly correct."
- **Keywords:** D38, D39, partial-update-accumulation

## workflow — chat-driven task lifecycle

### durable-poll-offset-resume
- **What:** The chat gateway owns its place in the Telegram message stream durably: loads the last-handled sequence number at startup, asks only for messages after it, and advances the saved number only *after* acting on a batch (act → persist → fetch order).
- **Where:** `cmd/agentbridge/telegram.go`, `internal/store/sqlite/telegram.go` (`SavePollOffset`/`LoadPollOffset`)
- **Notable:** "A restart resumes exactly once" — a command sent just before a crash is honoured on restart; one already answered is not re-answered. The library's own polling loop is deliberately bypassed so the offset stays caller-owned.
- **Keywords:** D53, D57, exactly-once-resume, act-persist-fetch

### new-wizard-guided-task-creation
- **What:** `/new` runs a 3-step tapped-button selection (project → agent → mode), then captures the person's next plain-text message as the task description; a second `/new` mid-flow replaces the in-progress selection rather than starting a parallel one.
- **Where:** `cmd/agentbridge/newwizard.go`
- **Notable:** The concurrency cap is checked **before** the first button is shown, not at final submit — a documented, accepted bounded race (two near-simultaneous `/new` calls can each land one over cap).
- **Keywords:** D58, D51, guided-selection, per-person-in-flight-state

### server-side-button-revalidation
- **What:** A tapped button's payload is never trusted as a validated choice — every collected choice (project/agent/mode) is re-validated server-side, freshly, at the exact moment the task is built.
- **Where:** `cmd/agentbridge/newwizard.go` (D66)
- **Notable:** "The sender is authenticated; the payload only selects a handler" — a forged payload naming an unoffered choice is refused exactly as a forged message would be.
- **Keywords:** D66, untrusted-payload, fresh-re-resolution

### permission-ceiling-mode-reconciliation
- **What:** The effective isolation mode is the more restrictive of the project's configured permission and the mode tapped in chat — a read-only project can never be made to run write-mode from chat, even via a forged button.
- **Where:** `cmd/agentbridge/newwizard.go`
- **Notable:** "A project's configured permission is a hard ceiling; the tapped mode can only narrow it, never widen it."
- **Keywords:** D60, no-widen-rule

### topic-first-fail-closed-task-creation
- **What:** Task creation happens in strict order — mint id → create the chat topic → create the task record naming that topic; if topic creation fails the whole operation aborts before any agent starts, and if the record write fails after the topic exists, the just-created topic is removed.
- **Where:** `cmd/agentbridge/newwizard.go` (`DeleteForumTopic` cleanup)
- **Notable:** Two invariants stated directly: a live agent is never left without a topic to reach it in; a failed creation never leaves an orphan topic behind.
- **Keywords:** D62, D59, D65, topic-first-ordering

### short-handle-resolution
- **What:** `/status <handle>` resolves any unique prefix of a task's canonical id, like a short source-control revision; an ambiguous prefix lists every match and asks rather than guessing.
- **Where:** `cmd/agentbridge/commands.go` (`sessionMatchesHandle`)
- **Notable:** "It never guesses" — same short-SHA-style resolution model applied to a locally-minted id scheme rather than a hash.
- **Keywords:** D20, unique-prefix, ambiguous-asks-not-guesses

### orphan-announcement-and-channel-loss-handling
- **What:** After recovery, the gateway posts exactly one message per not-yet-announced orphaned task at startup, then marks it delivered; if any send to chat fails or the bot loses group membership, the gateway keeps polling and takes no destructive action as a result.
- **Where:** `cmd/agentbridge/telegram.go`
- **Notable:** "Losing the chat channel is losing the control surface — it is never a command to stop an agent." Re-adding the bot is the entire recovery path for channel loss.
- **Keywords:** D47, D48, at-least-once, non-destructive-on-channel-loss

### no-inbound-webhook-poll-only
- **What:** The gateway has no inbound web endpoint at all — it only long-polls the platform for new messages, never the reverse.
- **Where:** `internal/telegram/client.go` (`GetUpdates`, defensive `DeleteWebhook` at startup)
- **Notable:** Stated as a deliberate posture, not a limitation — the defensive webhook-clear at startup guards against a stale delivery mode the platform might otherwise hold.
- **Keywords:** poll-only, no-webhook

## config-packaging

### strict-config-decoding
- **What:** Settings are decoded with `KnownFields(true)`: any unrecognized/misspelled key is a startup error naming the key, never silently defaulted; every validation failure is collected and reported together.
- **Where:** `internal/config/loader.go`, `internal/config/validation.go`
- **Notable:** "A mistyped security setting that falls back to a default is a security setting the operator believes is in force and is not."
- **Keywords:** KnownFields, fail-loud, per-key-errors

### bot-token-env-only
- **What:** The Telegram bot credential is never a settings field — read from the process environment by exactly one reader, never logged or serialized.
- **Where:** `internal/config/loader.go` (`BotTokenFromEnv`)
- **Notable:** Strict decoding turns any attempt to place the token in the settings document into an error, since no such field exists to receive it.
- **Keywords:** single-reader, env-only-credential

### empty-allowlist-fail-closed
- **What:** An allowed-project-roots list with zero entries is a hard startup error, never interpreted as "allow everything."
- **Where:** `internal/config/validation.go`
- **Notable:** The spec never said which way an unset allowlist should fall; given the service runs as an admin-capable account, failing open would grant an agent the entire filesystem — "the worst possible reading of a blank line."
- **Keywords:** D14, fail-closed-default

### cap-tightening-only
- **What:** A project's per-project caps may only tighten the service-global default, never widen it.
- **Where:** `internal/config/config.go` (global), `internal/projects/validation.go` (enforced)
- **Notable:** Same one-directional-override pattern recurs at the mode-reconciliation layer (`permission-ceiling-mode-reconciliation`) — a repeated convention across the codebase, not a one-off rule.
- **Keywords:** monotonic-caps, tighten-never-widen

### canonical-locations-supersede-spec
- **What:** Config/data live under the machine owner's own per-user directories (`~/.config/agentbridge/`, `~/.local/share/agentbridge/`), explicitly **superseding** the original specification's system-wide paths and separate service account.
- **Where:** `internal/config/loader.go`
- **Notable:** A locked decision that overrides the upstream spec by design choice, not by discovery of a bug — distinct from the DISCOVERY.md deltas below, which are involuntary corrections.
- **Keywords:** D1, spec-superseded-by-decision

## testing-evals

### adversarial-and-mutation-testing
- **What:** The security package carries unit tests plus two independent adversarial suites (written by a different author) and is mutation-tested against specific known-wrong implementations.
- **Where:** `internal/security/adversarial_test.go`, `internal/security/create_adversarial_test.go`
- **Notable:** "Swapping component containment for `strings.HasPrefix` fails the prefix trap; moving symlink resolution after the containment check fails all five escape cases" — mutation tests are proof the real bug classes are actually caught, not merely asserted safe.
- **Keywords:** adversarial-suite, mutation-testing, non-vacuous-proof

### composer-testdata-ground-truth
- **What:** The readiness-matching rule is judged in tests against verbatim captured real terminal screens from real coding agents, plus a completeness assertion that fails if a new fixture is ever added and never judged, plus 10 synthetic unseen-shape fixtures proving default-deny (not "remembering a list").
- **Where:** `internal/agents/testdata/`, `internal/agents/adapter_test.go` (`TestComposerAgainstRealScreens`, `TestDefaultDeny_UnseenScreenShapes`)
- **Notable:** One screen (an agent's update-offer prompt) could not be re-captured live (downgrading a person's agent to reproduce it was refused) — documented from the incident evidence instead and explicitly never presented as a captured fixture, i.e. the gap itself is recorded rather than silently backfilled with a fake.
- **Keywords:** real-screen-fixtures, completeness-assertion, default-deny-on-unseen

## quality-gates

### single-verify-target-is-the-done-bar
- **What:** One Makefile target (`verify`) — `go build ./... && go vet ./... && go test -race ./...` — is stated as the *only* definition of "green" in the repository; every unit of work is checked against exactly this, nothing narrower or looser.
- **Where:** `Makefile` (`verify` target, comment: "The done-bar (decision D7). Every cell caps against exactly this.")
- **Notable:** Decision D7 explicitly defers coverage thresholds and linting (`golangci-lint`) past the first milestone rather than silently skipping them — the gap is named and dated, not hidden.
- **Keywords:** D7, done-bar, single-verify-target, deferred-lint-not-hidden

## repo-layout

### specs-vs-history-directory-split
- **What:** `docs/specs/` holds only current, tech-agnostic, continuously-updated truth about each area (superseded content is edited in place, never left stale); `docs/history/<feature>/` holds point-in-time, append-only planning artifacts (`CONTEXT.md`, `plan.md`, `reports/`) for a specific unit of work and is never treated as current truth once the feature ships.
- **Where:** `docs/specs/reading-map.md`, `docs/history/telegram-remote-agent-bridge/`, `docs/history/herdr-remote-launcher/`
- **Notable:** `reading-map.md` states this split explicitly as its own maintenance contract ("updated whenever an area spec is created or moved") — the two directories answer different questions ("what is true now" vs "what did we decide and why, at the time") and are never conflated.
- **Keywords:** specs-vs-history, current-truth-vs-point-in-time, reading-map-ownership

## docs-style — spec-vs-reality documentation pattern

### discovery-supersedes-written-spec
- **What:** A dedicated "Milestone 0" spike ran real herdr/Codex/Claude against the written specification *before* any product code was written, and recorded exactly where reality contradicts the spec (`docs/DISCOVERY.md`); the project's documentation policy is that DISCOVERY wins wherever the two disagree, permanently, not just at the time of the spike.
- **Where:** `docs/DISCOVERY.md`, `docs/specs/system-overview.md` ("Where reality diverges from the original specification")
- **Notable:** Three concrete deltas were found this way, one rated HIGH (Codex's trust prompt reads as `idle` not `blocked`, invalidating the spec's own session-start flow as written) — found by literally running the real tool, not by re-reading the spec harder. Framed as "it was written before the terminal runtime was ever actually run," i.e. an explicit acknowledgment that specs written pre-implementation are provisional until verified.
- **Keywords:** DISCOVERY.md, spec-vs-reality, milestone-0-spike, pre-implementation-verification

### locked-decision-id-log
- **What:** Every non-trivial design choice — not just security-relevant ones — gets a stable `D<n>` id, referenced by number from every spec file that depends on it, with the append-only decision log as the single source of truth (`.bee/decisions.jsonl` in this repo's own dev tooling; `CONTEXT.md` for the human-readable subset).
- **Where:** `docs/history/telegram-remote-agent-bridge/CONTEXT.md`
- **Notable:** Decisions are cross-referenced constantly across otherwise-independent spec files (e.g. D1 unifying two initially-contradictory decisions, D11, by being invoked from both) — turning "why is it built this way" into a stable, greppable id instead of scattered prose that drifts.
- **Keywords:** D-number, decision log, cross-referenced-not-duplicated

### reading-map-as-where-does-x-live-index
- **What:** A single `docs/specs/reading-map.md` table maps every functional area to its authoritative spec file and its code entry points, explicitly to answer "where does X live" without a grep, plus a companion "not yet specced" table for known-future areas.
- **Where:** `docs/specs/reading-map.md`
- **Notable:** States its own maintenance contract inline: "owned by [the doc-sync process]: updated whenever an area spec is created or moved" — the index is a tracked artifact with an owner, not a one-time convenience file that rots.
- **Keywords:** reading-map, area-to-spec-to-code-index, maintained-not-static

## Coverage notes

**Domains checked with nothing airemote-specific to file:** `skills`, `hooks`, `planning`, `self-improvement` — the repo's `AGENTS.md`/`CLAUDE.md`/`.bee/`/`.claude/skills/` machinery is the generic third-party "bee" dev-workflow framework (cells, gates, reservations, compounding), not something airemote's own author designed; distilling bee's own design would be its own separate distillery source if wanted, not attributed here to airemote. `tooling` — airemote exposes no automation/robot control surface of its own (unlike ntm/herdr); its only external control surface is the Telegram command set, already filed under `workflow`. `ux` — the one user-facing design choice with real UX reasoning (topic naming balancing scannability vs. typeable short-handles) is filed once, under `workflow` → `topic-first-fail-closed-task-creation` / `short-handle-resolution`, rather than duplicated here.

**Read in full:** `docs/DISCOVERY.md`, `docs/specs/{security-boundary,configuration,project-registry,herdr-integration,session-lifecycle,telegram-gateway,system-overview,persistence,reading-map}.md`, `docs/history/telegram-remote-agent-bridge/CONTEXT.md`, `README.md`, `Makefile`. File-path existence cross-checked via `git ls-files`/directory listings for `internal/*`, `cmd/agentbridge/*`, `migrations/*`.

**Not read:** `docs/specs/Telegram Remote Agent Bridge.md` (the 1823-line original spec — superseded by the per-area specs above wherever they disagree, per the project's own stated policy; the per-area specs were treated as authoritative rather than re-deriving from the superseded original), `docs/discovery/*` raw evidence files (probe transcripts, schema JSON, event logs — cited secondhand via DISCOVERY.md's own summary), `.bee/decisions.jsonl` (out of scope, see above), full Go source bodies (function/symbol names in `Where:` fields are taken from the specs' own "Pointers (implementation)" sections and cross-checked only for file-path existence, not re-verified line-by-line against source).

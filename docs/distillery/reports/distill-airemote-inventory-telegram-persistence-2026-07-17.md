# airemote (AgentBridge) inventory — distill extraction report

Source: local clone at `upstreams/airemote`. Method: direct read of the four named files
(`docs/specs/telegram-gateway.md`, `docs/specs/system-overview.md`, `docs/specs/persistence.md`,
`docs/history/telegram-remote-agent-bridge/CONTEXT.md`). Mechanical inventory only — no porting
judgment, no cross-source comparison. File paths in "Where" cross-checked to exist against
`internal/telegram/*.go`, `internal/store/**/*.go`, `cmd/agentbridge/*.go`, `migrations/*.sql`.

## telegram-gateway

### auth-gate-fail-closed
- **What:** Every inbound event (message or button press) is accepted only if it comes from the single configured chat group AND its sender is one of the permitted people; every other event — wrong group, unlisted sender, or no identifiable sender at all — is dropped with zero reply and one durable, content-free audit record.
- **Where:** `internal/telegram/auth.go` (`Gate.Authorized(chatID, userID)`, `extract(*gotgbot.Update)`), `internal/telegram/auth_test.go`
- **Notable:** "Silence is the point. A stranger who probes the channel gets zero signal that the service exists on it — no error, no 'not allowed', nothing. Replying would confirm a bot is there and leak the very existence of the project list." A button press is authenticated by its own sender independently of whoever opened the menu. An event with no identifiable sender (anonymous-admin post, service notice, inaccessible callback context) is treated as unauthenticatable, never a crash (R4). Locked decisions D46, D56.
- **Keywords:** fail-closed, total silence, no-leak rule, content-free audit, `Gate.Refuse`

### refuse-to-start-unconfigured
- **What:** The gateway refuses to start entirely unless both the configured chat group and the non-empty permitted-people list are set; the authentication check is meaningless without them.
- **Where:** `cmd/agentbridge/telegram.go` (`serve`, running only after `startup()`)
- **Notable:** "An unguarded gateway is an open door." Configuration itself loads fine without these values (`configuration.md` R5); the obligation to refuse lives specifically in the gateway. Decision D15.
- **Keywords:** refuse-to-start, R2, open door

### durable-poll-offset-resume
- **What:** The gateway owns its place in the Telegram message stream durably rather than trusting the platform: it loads the highest sequence number it has finished handling at startup, asks for messages strictly after that number each poll cycle, and only advances the saved number after acting on a batch — order is act, then persist, then fetch.
- **Where:** `cmd/agentbridge/telegram.go` (poll loop: `LoadPollOffset` → `GetUpdates(Offset=H+1|0, Timeout=PollTimeout+margin)` → authenticate → dispatch → `SavePollOffset(max processed)`), `internal/store/sqlite/telegram.go` (`SavePollOffset`/`LoadPollOffset`)
- **Notable:** "A restart resumes exactly once: a command sent to the service just before it went down is honoured when it comes back; a command it already answered is not answered again." The library's own `Start()` polling is deliberately never used — the offset is caller-owned. Transient read failures back off with a capped, growing delay and never busy-spin. Decisions D53, D57.
- **Keywords:** place marker, exactly-once resume, long-poll, act-persist-fetch order, D55 timeout split

### orphan-announcement-once-at-startup
- **What:** After session recovery finishes at startup, the gateway posts exactly one message per not-yet-announced orphaned task to the group's main topic, then marks that task's notification obligation delivered so a later restart stays quiet about it.
- **Where:** `cmd/agentbridge/telegram.go` (send → `MarkOrphanNotificationDelivered`, keyed by event id), `internal/store/sqlite/telegram.go` (`ListUndeliveredOrphanNotifications`)
- **Notable:** Delivery is at-least-once by design: "send first, record second, so a crash between the two resends the announcement on the next start. A duplicate announcement is preferred to a silent loss." A send that fails is never recorded as delivered. Decisions D47, D57.
- **Keywords:** at-least-once delivery, orphan obligation, delivered_at marker, non-fatal-if-not-told-never

### losing-the-channel-non-fatal
- **What:** If any send to chat fails, or the bot is kicked/demoted in the group, the gateway logs the failure and keeps polling; it never takes any destructive action (no interrupt, no checkout removal, no cancellation) as a result.
- **Where:** `cmd/agentbridge/telegram.go` (send-failure handling in the run loop)
- **Notable:** "Losing the chat channel is losing the control surface — it is never a command to stop an agent." Re-adding the bot is the entire recovery path. Decision D48.
- **Keywords:** non-destructive, control-vs-work separation, re-add-to-recover

### read-only-health-and-projects-commands
- **What:** `/health` and `/projects` are read-only summaries assembled purely from facts the service already holds, posted to the group's main topic; `/health` reports gateway health, terminal-runtime reachability (as a status word only, never the raw error/path), active/blocked session counts, and version; `/projects` lists each approved project's label, allowed agents, and base branch, never a resolved directory.
- **Where:** `cmd/agentbridge/commands.go` (`buildHealthMessage`/`buildProjectsMessage`, wired via `commandTableFor`)
- **Notable:** The health summary deliberately omits host-uptime — "reading it needs a machine-level call the gateway deliberately avoids so the summary stays a pure assembly over already-known facts. Its absence is a settled choice, not a gap" (decision `68507c6a`). Decisions D45.
- **Keywords:** deliberately minimal, no-leak, status word only, pure assembly

### new-wizard-guided-task-creation
- **What:** `/new` runs a three-step tapped-button selection (project from the allowlist → agent that project allows → mode: read-only or worktree-write), then captures the person's next plain-text message as the task description; a second `/new` from the same person replaces the in-flight selection instead of starting a parallel one.
- **Where:** `cmd/agentbridge/newwizard.go` (3-step inline-keyboard flow keyed by `(chatID,userID)`, plain-message task interception)
- **Notable:** "the 'plain text is never input' assumption of the read-only foundation holds only outside an in-flight selection; inside one, that person's next message is the task." The no-isolation ("use current workspace") third mode option is deferred to its own later slice. Decisions D51 (cap check), D58 (two modes only).
- **Keywords:** guided selection, tapped buttons, per-person in-flight state, D58 two-mode set

### concurrency-cap-checked-at-invocation
- **What:** `/new`'s concurrency cap check runs before a single selection button is shown; a person already at the cap is refused immediately with a message naming the cap, and no selection begins.
- **Where:** `cmd/agentbridge/newwizard.go` (cap refusal at invocation, R10)
- **Notable:** Explicitly documented as a bounded race, not a bug: "Two `/new` calls that pass the check at nearly the same instant can each proceed and land the person one over the cap. This is accepted per the locked decision (D51); a create-time cap is a possible future hardening." Decision D51.
- **Keywords:** invocation-time cap, named-limit refusal, known bounded race

### server-side-button-revalidation
- **What:** A tapped button's payload is whatever the person's own client sends and is never trusted as a validated choice; every collected choice (project, agent, mode) is re-validated server-side against the allowlist at the exact moment the task is built, with the project resolved fresh rather than cached from when the buttons were shown.
- **Where:** `cmd/agentbridge/newwizard.go` (D66 server-side re-validation at submit: `reg.Get` allowlist, `Project.AllowsAgent`)
- **Notable:** "The sender is authenticated (R1); the payload only selects a handler." A forged payload naming an unoffered project/agent/mode is refused exactly as a forged message would be. Decision D66.
- **Keywords:** untrusted button payload, fresh re-resolution, forged-payload refusal

### permission-ceiling-mode-reconciliation
- **What:** The effective isolation mode for a created task is the more restrictive of the project's configured permission and the mode the person tapped (read-only if either says so); a project configured read-only can never be made to run a write agent from chat, even via a forged button.
- **Where:** `cmd/agentbridge/newwizard.go` (`ReadOnly = configuredReadOnly OR chosenReadOnly`)
- **Notable:** "A project's configured permission is a hard ceiling. The mode a person taps can only equal or narrow it, never widen it." Decision D60.
- **Keywords:** permission ceiling, no-widen rule, more-restrictive-of-two

### topic-first-fail-closed-task-creation
- **What:** Task creation happens in strict order — mint the task id, create the task's chat topic, create the task record carrying that topic identity — and if topic creation fails the whole operation aborts before any agent starts; if task creation fails after the topic was made (including an id collision from two simultaneous `/new` calls), the just-created topic is removed and a retryable failure is reported.
- **Where:** `cmd/agentbridge/newwizard.go` (D62/D65 topic-first fail-closed with `DeleteForumTopic` cleanup), `internal/telegram/client.go` (`CreateForumTopic` returns thread id, `DeleteForumTopic`)
- **Notable:** "The invariants: a live agent is never left without a topic to reach it in, and a failed creation never leaves an orphan topic behind." A removal that itself fails is logged and non-fatal. Decisions D62, D59, D65 (create-ordering, implementation-only).
- **Keywords:** topic-first ordering, fail-closed, orphan-topic prevention, id-collision handling

### task-topic-naming-convention
- **What:** Each task's chat topic is named `<NNN> · <task summary> · <Agent>`: `NNN` is the within-day counter tail of the task id, the task summary is the first ~40 characters of the typed task with line breaks/control characters stripped (elided if truncated), and `Agent` is the coding agent's name (e.g. `001 · sua auth refresh · Codex`).
- **Where:** `cmd/agentbridge/newwizard.go` (D59 topic naming)
- **Notable:** "It keeps the topic recognizable in the list (what the task is) and typeable for `/status` (the short handle) at once." Decision D59.
- **Keywords:** topic naming, recognizable-and-typeable, task summary truncation

### create-outcome-reported-in-topic
- **What:** Every task-creation outcome — ready, waiting-for-a-human, or failed — is posted into the task's own topic, which exists regardless of the outcome; on "waiting for a human" the machine posts the parked on-screen content and sends zero keystrokes.
- **Where:** `cmd/agentbridge/newwizard.go` (D61 ready / needs_attention / failed outcomes)
- **Notable:** "it never answers a prompt it does not recognize" — ties directly to the D24 readiness rule owned by `session-lifecycle.md`. Decisions D61, D24.
- **Keywords:** three-way outcome, zero-keystroke rule, topic-always-exists

### sessions-and-status-commands
- **What:** `/sessions` lists a person's tasks (state, project, agent, elapsed time) and `/status <handle>` shows one task by short handle — both read exclusively from the durable stored record, never the agent's live screen, and only display the states the system currently models (running-agent states working/blocked/done are structurally absent).
- **Where:** `cmd/agentbridge/commands.go` (`sessionRowsFrom`/`buildSessionsMessage`, `buildStatusReply`/`sessionMatchesHandle`)
- **Notable:** Worktree path is redacted to its leaf in status output; no live pane read is ever performed for these commands. Decisions D64 (durable-record-only), D33 (M2-era states only), D20 (short-handle resolution).
- **Keywords:** durable-record-only, no-live-screen-cost, state allowlist

### short-handle-resolution
- **What:** `/status <handle>` resolves a short handle as a unique prefix of a task id (like `001` or a longer `ab-…` prefix), resolved like a short source-control revision; a prefix matching more than one task lists the matches and asks the person to be more specific rather than guessing.
- **Where:** `cmd/agentbridge/commands.go` (`sessionMatchesHandle`)
- **Notable:** "It never guesses." Applies the same handle scheme session-lifecycle defines canonically (`ab-YYYYMMDD-NNN` canonical id, any unique prefix as handle). Decision D20.
- **Keywords:** unique-prefix resolution, ambiguous-handle listing, short-SHA analogy

### no-inbound-webhook-poll-only
- **What:** The gateway has no inbound web endpoint of any kind; it only ever asks the platform for new messages (long-poll `GetUpdates`) and is never called by the platform.
- **Where:** `internal/telegram/client.go` (`GetUpdates` via long-poll, `DeleteWebhook` called defensively at startup)
- **Notable:** "This is a deliberate posture, not a limitation of the foundation." A defensive `DeleteWebhook` clears any stale delivery mode the platform may hold so the gateway owns the channel cleanly.
- **Keywords:** poll-only, no webhook, deliberate posture

### thin-telegram-client-with-timeout-split
- **What:** The Telegram client is a thin wrapper over `github.com/PaulSonOfLars/gotgbot/v2` with zero other external module dependencies; ordinary calls (`SendMessage`, `DeleteWebhook`, `GetMe`) use the normal request timeout, while the long-poll `GetUpdates` uses its own `PollTimeout()+margin` so it is never aborted mid-hold.
- **Where:** `internal/telegram/client.go`
- **Notable:** Built with `DisableTokenCheck` so unit tests are hermetic (no network `getMe`); bot token is read only via `config.BotTokenFromEnv`, never logged; content is redacted through `internal/security`'s `DefaultRedactor` before any logging. Decision D55 (the timeout split), D54 (zero-dependency client).
- **Keywords:** D54, D55, hermetic tests, redaction before logging

## persistence

### exclusive-single-service-lock
- **What:** Exactly one service may hold the durable record at a time; ownership is claimed exclusively at startup (before the record is even opened), and a second service instance refuses to start, naming the record it is locked out of.
- **Where:** `internal/store/sqlite/lock.go` (`LockDir`: an `flock` on the data directory taken before `Open`)
- **Notable:** "Two services over one record is not a theoretical duplication: both read back the same open tasks, both re-attach to the same live agents... A stray second instance cancelling a task would send an interrupt into the first one's working agent and kill it." A crash releases the lock with no cleanup step — the OS releases it on SIGKILL, so a fresh service started immediately after a kill takes it without hesitation. Decision D41.
- **Keywords:** flock, exclusive ownership, second-instance refusal, crash-safe release

### migrations-all-or-nothing-repeatable
- **What:** Every pending SQL migration is applied inside a single transaction for the whole run (not one per file); the applied-migrations tracking table is created inside that same transaction, so a first-run failure leaves no trace of it either. Re-running against an already-current record is a safe no-op.
- **Where:** `internal/store/sqlite/migrations.go` (`applyMigrations`), `migrations/0001_init.sql`, `migrations/0002_telegram.sql`, `migrations/embed.go` (`//go:embed *.sql`)
- **Notable:** "A half-migrated record is never an observable state. On a fresh record, a failed run leaves it completely empty; on an already-current record, a failed run leaves the previous content byte-identical and the failing migration unrecorded." Migrations travel embedded inside the service binary, not as files the operator ships separately.
- **Keywords:** all-or-nothing, repeatable, single transaction, schema_version table

### task-record-written-before-it-exists
- **What:** A task's database row is written first — before its checkout is cut and its agent is started; a failed write aborts the whole creation, so no live agent can exist that the durable record has never heard of.
- **Where:** `internal/sessions/{manager,create,state,record}.go` (`CreateSession` before the worktree)
- **Notable:** "The alternative, running the agent anyway and recording it later, is exactly the hole this behavior exists to close... The price is stated plainly: while the record cannot be written, no new task can be created." A write failure *after* the agent is live does NOT kill the agent — the row lags but is not absent. Decisions D35.
- **Keywords:** write-before-exists, fail-the-creation-not-the-agent, B6

### runtime-identity-filled-in-as-known
- **What:** Four match keys a restart re-matches a task by (workspace, terminal/pane, agent target, the agent's own native session id) do not exist when the task row is written; they are filled in via in-place updates the moment each becomes known, and an update that doesn't supply a value leaves it unchanged rather than blanking it.
- **Where:** `internal/store/store.go` (`UpdateSessionRuntime(ctx, id, SessionRuntime, now)` — every field a pointer, `nil` = leave unchanged), `internal/sessions/{manager,create,state,record}.go`
- **Notable:** "Updating only the state would leave the match keys empty forever, and a restart would then match nothing and orphan every live task — with every other part of the system looking perfectly correct." The native agent session id is empty at agent start and compared only when both sides carry a value (never a mismatch when one side is empty). Decisions D38, D39.
- **Keywords:** match keys, partial-update accumulation, best-effort native session id

### branch-not-a-match-key
- **What:** The branch name stored on a task is kept only for the on-disk collision check when minting a new task id — it is explicitly not one of the four runtime-identity match keys used to re-attach a task to a live agent after restart.
- **Where:** `internal/store/store.go` (`branch_name` field), `session-lifecycle.md` (owns id-minting collision check)
- **Notable:** "the runtime's picture of what is alive has no branch in it." Decision D37.
- **Keywords:** branch not identity, collision check only

### task-topic-identity-written-at-create-and-looked-up-both-ways
- **What:** A task's chat-topic identity (`telegram_thread_id`) is written into the row at creation time as part of the create request — never attached by a later step — and can be looked up by matching chat-group-id AND topic-id together (never topic id alone, since topic ids are only unique within a group); a task with no topic recorded is never returned by this lookup.
- **Where:** `internal/store/store.go` / `internal/store/sqlite/sessions.go` (`GetSessionByThreadID(ctx, chatID, threadID)`, returns `ErrSessionNotFound` on no match — a NULL thread id never matches since SQL `NULL = ?` is never true)
- **Notable:** Written specifically so the not-yet-built in-topic control commands can route a topic's messages back to its task. Decisions D62, D65.
- **Keywords:** topic identity, group+topic composite match, NULL-never-matches

### never-store-output-or-credentials
- **What:** The durable record deliberately never stores a line of terminal output (only a hash of the last output, to detect an unchanged screen) and never stores any credential — not the chat bot token, not the coding agents' own credentials.
- **Where:** `internal/store/store.go` (`last_output_hash` field)
- **Notable:** "The shape of the record is frozen in this slice, which is why the switch that keeps a task's first instruction out of it has to exist now" (ties to the `initial_prompt` opt-out switch, R5). Decisions R1, R2 (spec §12, §6.6).
- **Keywords:** no-output-storage, no-credential-storage, hash-only

### store-initial-prompt-toggle
- **What:** Whether a task's first typed instruction is written to the durable record at all is a configuration setting (`sessions.store_initial_prompt`); the field exists in the schema either way, and the setting decides only whether it gets populated.
- **Where:** the switch is `sessions.store_initial_prompt` (documented in `configuration.md`); field is `initial_prompt` in `internal/store/store.go`
- **Notable:** "A person can paste a credential into a chat message, and this switch is the only way to keep that text out of the durable record." Decision R5 (spec §15.7, D15-adjacent).
- **Keywords:** credential-in-chat risk, opt-out toggle, frozen schema

### refusal-audit-log
- **What:** Every refused chat command (unauthorized sender/group) is written durably to an append-only audit table carrying who, where, and what action was refused, with no message content whatsoever.
- **Where:** `internal/store/store.go` / persistence layer (`AppendAuditLog(ctx, entry)` writing `audit_logs`), `internal/store/sqlite/telegram_test.go`
- **Notable:** "A refusal is as much a record as a success — the refusals are the ones worth reading." This is the audit log's first live writer; the spec's other audited actions (create task, send input, interrupt, cancel, remove worktree, reload config, agent-start failure, runtime reconnect) still await their own writers. Decisions D56, R17.
- **Keywords:** content-free audit, first-writer, audit_logs table

### deferred-obligation-as-durable-event
- **What:** When a restart finds a task's terminal gone (orphaned), the fact that its owner must still be told is appended as an event on that task's append-only history — not a log line — with a `delivered_at` marker that starts empty and is set only after a successful send, keyed to the event (not the task) since one task may earn more than one orphan event across restarts.
- **Where:** `internal/store/store.go` (`AppendSessionEvent`), `internal/store/sqlite/telegram.go` (`ListUndeliveredOrphanNotifications` — rows where `event_type='orphaned' AND delivered_at IS NULL`; `MarkOrphanNotificationDelivered(ctx, eventID, now)`)
- **Notable:** "a log line does not survive the next restart, and an obligation that a restart can forget is not an obligation." Decisions D38, D47, D57.
- **Keywords:** deferred obligation, event-keyed delivery marker, survives-restart

### three-table-durable-schema
- **What:** The durable record is three core tables — `sessions` (one row per task), `session_events` (append-only per-task history), `audit_logs` (append-only who/what/refused-or-not) — plus a small key-value `telegram_state` table (one row, `key='poll_offset'`) and an applied-migrations tracking table.
- **Where:** `migrations/0001_init.sql` (sessions, session_events, audit_logs), `migrations/0002_telegram.sql` (additive `telegram_state` key-value table + nullable `delivered_at` column on `session_events`)
- **Notable:** `0002` is additive-only — no `0001` column is touched — and applies on top of `0001` in the same single migration transaction. Schema is asserted in tests against the live database via `pragma_table_info`, column by column, not against migration text.
- **Keywords:** sessions, session_events, audit_logs, telegram_state, additive migration

### sqlite-store-implementation-choices
- **What:** The store uses the pure-Go `modernc.org/sqlite` driver (pinned `v1.53.0`, no cgo, keeps `CGO_ENABLED=0` builds possible), opened in WAL journal mode with a 5000ms busy timeout set in the connection DSN (because the pool opens connections on demand and a setting applied after opening would not reach every connection).
- **Where:** `internal/store/sqlite/store.go` (`Open`, `Ping`, `WithTx`, `Close`)
- **Notable:** "Do not bump [the driver version] without re-running that proof" — proven under `-race` with many simultaneous writers (`docs/history/telegram-remote-agent-bridge/reports/m1-validation.md`, Proof 1).
- **Keywords:** modernc.org/sqlite, pure-Go driver, WAL mode, busy timeout, race-proven

### record-location-under-user-data-dir
- **What:** The durable record file lives under the machine owner's own per-user data directory (`~/.local/share/agentbridge`), not any system-wide state directory; the lock file sits beside the database in the same directory.
- **Where:** `internal/store/sqlite/lock.go`, `internal/store/sqlite/store.go`
- **Notable:** Supersedes the original specification's system-wide state-directory path. Decision D1.
- **Keywords:** per-user data dir, D1 path superseding

## system-overview (architecture-level)

### own-login-account-no-isolation
- **What:** The AgentBridge service, the terminal runtime (herdr), and both coding agents all run under the machine owner's own login account — the account that owns private keys, cloud credentials, browser sessions, and can obtain administrator rights. There is no separate, weaker service identity.
- **Where:** described at the architecture level in `docs/specs/system-overview.md`; concretely realized by `internal/security`'s allowlist validator plus the coding agents' own sandbox settings
- **Notable:** "A chat message therefore arrives with the full reach of the person who owns the machine... What stands between a chat message and that reach is exactly three things — the path allowlist, the coding agents' own permission settings, and secret redaction — and one of them is not really containment at all (an agent with a terminal can type any command the account can run)." Restoring a restricted service account is called out as an open, unresolved product-backlog item that the stated security posture "cannot honestly be signed off without." Decision D1.
- **Keywords:** no isolated service account, full-reach identity, honest security-gap admission

### terminal-runtime-owns-every-terminal
- **What:** The terminal runtime (herdr) owns every terminal process; the AgentBridge service itself never spawns one. This is stated as the single architectural decision everything else — including a person walking up and taking over the exact same terminal by hand — follows from.
- **Where:** architecture-level statement in `docs/specs/system-overview.md`; realized via `herdr-integration.md` (not one of the four read files, referenced only)
- **Notable:** "That is what makes 'take over by hand' possible at all, and it is the architectural decision everything else follows from."
- **Keywords:** runtime owns terminals, hand-off design, take-over-by-hand

### shared-entity-model
- **What:** The system defines five shared entities used across areas: Project (operator-approved repo with allowed agents/permissions/caps, keyed by name), Task/"session" (one chat topic ↔ one workspace+checkout ↔ one terminal running one agent, durable), Runtime identity (agent target + agent's own native session id which *identify*; terminal + workspace which *locate* — branch explicitly excluded), Orphaned (terminal gone, work still on disk — deliberately not "failed" so no cleanup path is licensed to delete unfinished work), Agent status (idle/working/blocked/done/unknown, where idle does not mean ready).
- **Where:** `docs/specs/system-overview.md` ("The shared entities" table)
- **Notable:** "Orphaned... Deliberately not 'failed', so no cleanup path is ever licensed to delete real, unfinished work." "idle does not mean ready" is called out explicitly as a trap the whole notification design has to route around.
- **Keywords:** shared entity glossary, orphaned-not-failed, idle-is-not-ready

### end-to-end-flow-numbered-steps
- **What:** system-overview.md documents the intended chat-to-running-agent flow as ten numbered steps: (1) message authenticated, (2) project looked up by key never path, (3) title reduced to an allowlisted-charset slug, (4) fresh checkout created with a no-follow create and re-check, (5) coding agent started in the dedicated named session via a per-agent adapter that can only select a safe sandbox mode, (6) trust prompt cleared and readiness confirmed by a positive composer match, (7) first instruction typed/confirmed-on-screen/submitted only if ready, (8) task recorded before it exists, (9) service can die without killing the work — exclusive record ownership, single look at what's alive, orphaning is read-only, (10) person watches — real-time notification is not yet built.
- **Where:** `docs/specs/system-overview.md` ("How a chat message becomes a running coding agent")
- **Notable:** Step 1 (chat gateway) and step 9's watching/notification half (step 10) are explicitly marked as the only unbuilt parts of the chain as of this doc's writing; steps 2–8 and the read-only half of 9 are marked built and proven, including "restart survival... proven in two real processes against a real agent."
- **Keywords:** ten-step flow, built-vs-not-built ledger, double-validated checkout

### spec-vs-reality-divergence-tracking
- **What:** The original technical specification (`Telegram Remote Agent Bridge.md`) remains the reference for areas not yet built, but wherever it disagrees with what running the real terminal runtime actually proved (`docs/DISCOVERY.md`), the discovery findings win and the locked decisions record why — explicitly superseding the spec's separate service account, system-wide file locations, "set an environment variable to choose a session," and "sending text delivers it."
- **Where:** `docs/specs/system-overview.md` ("Where reality diverges from the original specification")
- **Notable:** Named as a deliberate documentation policy, not an incidental note: "it was written before the terminal runtime was ever actually run."
- **Keywords:** DISCOVERY wins, spec superseding, discovery-before-implementation

## history/CONTEXT (architecture and process decisions)

### no-dedicated-service-user
- **What:** AgentBridge, herdr, Codex, and Claude all run under the existing primary login user rather than a dedicated non-sudo service account; systemd units are user units (`systemctl --user`) with lingering enabled, and all config/data paths are under that user's home.
- **Where:** described in CONTEXT.md D1; realized in `internal/store/sqlite/lock.go`, config loading
- **Notable:** "the agents run as the primary login user, which has sudo, and can reach the entire home directory — strictly weaker than spec §15.2's non-sudo `agenthost`... Restoring the isolated user is tracked as PBI-2." Decision D1.
- **Keywords:** systemd user units, PBI-2, sudo-capable agent account

### path-allowlist-containment
- **What:** Every project path and worktree path must, after symlink resolution, sit inside a configured allowed-roots list or be rejected at config load; a hard, non-overridable deny list (`/`, `/etc`, `/root`, `/var/lib`, `/mnt/*`, `~/.ssh`, `~/.aws`, `~/.config`) always applies, and an empty allowlist is a hard config-load error (fail closed, never "allow everything").
- **Where:** described in CONTEXT.md D6/D11/D14; realized in `internal/security`
- **Notable:** "`/mnt/c` is denied outright: slow git/build I/O and a Windows permission model that makes Codex's sandbox unreliable." D11 clarifies that the service's own config/data paths are not subject to the allowlist (they're never user-supplied) — found necessary because D1's `worktree_root` sat outside D6's only allowed root, making the two decisions mutually exclusive until D11 unified them into "one validator, two roots, one deny list." Decisions D6, D11, D14.
- **Keywords:** allowed_project_roots, hard deny list, fail-closed-empty-allowlist, D11 unification

### branch-and-slug-sanitization
- **What:** Branch slugs and session slugs are produced only by a dedicated sanitizer package, never taken directly from a prompt or a Telegram-supplied title; worktree creation refuses an unsanitized branch name outright.
- **Where:** described in CONTEXT.md D12; realized in `internal/security`
- **Notable:** "A Telegram task title becomes a branch name becomes a path component under `worktree_root`; `../../../.ssh` in a title would otherwise be created verbatim by `herdr worktree create`, and nothing else in the system would stop it." Found by the M1 security panel — no cell had owned this requirement despite the spec requiring it. Decision D12.
- **Keywords:** slug sanitizer, path-traversal-via-title, panel-caught gap

### two-step-confirmed-submit
- **What:** Sending text to an agent's terminal is a two-step, confirmed submit: the text is typed, then Enter is sent as a separate pane keystroke only after polling confirms the typed text actually landed on screen (3-second deadline, else a typed error — never reported as "delivered" on a bare type).
- **Where:** described in CONTEXT.md D9 (herdr integration boundary, not one of the four inventoried spec files directly)
- **Notable:** "sending Enter too soon loses the submission entirely (observed twice)" during the M0 discovery spike against the real herdr CLI. Supersedes the original spec's single-call `SendAgent(ctx, target, text)` signature. Decision D9.
- **Keywords:** confirmed-landed poll, two-step submit, D9 discovery evidence

### trust-prompt-idle-not-ready
- **What:** A coding agent's first-run "do you trust this directory" prompt reports as `idle` in the terminal runtime, indistinguishable from a genuinely ready agent unless readiness is defined as a positive, recognized composer match rather than the idle status alone; sending input while parked on an unrecognized prompt can kill the agent.
- **Where:** described in CONTEXT.md D10; realized in session-lifecycle readiness logic (not one of the four inventoried files)
- **Notable:** "Since every write-enabled task gets a fresh worktree, every AgentBridge session hits this prompt and looks idle — a Telegram-created session would hang silently forever, since idle fires no notification." Carried forward as the D43 "default-deny readiness" fix referenced in system-overview.md. Decision D10.
- **Keywords:** idle-is-not-ready, default-deny readiness, zero-keystroke-on-unrecognized-prompt

### realtime-event-subscriber-deferred
- **What:** The herdr socket event subscriber (push notifications of agent state changes) is explicitly out of scope through Milestone 4; Milestone 1–4 poll agent status through the CLI client instead, and `Subscribe` returns a typed "not implemented" error until Milestone 5.
- **Where:** described in CONTEXT.md D13; realized as the not-yet-built half of system-overview.md step 10
- **Notable:** Found by the M1 feasibility panel because "the fake was a CLI fake — which structurally cannot deliver socket push events" for a requirement that assumed the real subscriber existed. Herdr server restart/reconnect behavior remains explicitly unprobed and is called out as a live carried-forward risk. Decision D13.
- **Keywords:** deferred socket subscriber, M5 boundary, unprobed reconnect risk

### canonical-session-id-and-short-handle
- **What:** A task's canonical id has one fixed form, `ab-YYYYMMDD-NNN` (date-scoped, never reused, sortable), used for the branch name, worktree path, database key, and runtime agent target; the handle a person types in chat is any unique prefix of it, resolved like a short git SHA, with an ambiguous prefix always triggering a disambiguation list rather than a guess.
- **Where:** described in CONTEXT.md D20; realized in `cmd/agentbridge/commands.go` (`sessionMatchesHandle`) and session-id minting (not one of the four inventoried files directly)
- **Notable:** Three mandatory consequences spelled out: (1) the within-day counter comes from a `max+1` database query, never an in-memory counter, so a restart can't recreate a duplicate id; (2) creation must check the branch/worktree don't already exist on disk and skip forward if they do, not merely trust the database; (3) the date must come from the configured timezone, never UTC. Resolves a literal self-contradiction between two sections of the original spec. Decision D20.
- **Keywords:** ab-YYYYMMDD-NNN, short-SHA-style handle, database-derived counter, timezone-not-UTC

### close-and-cleanup-terminal-state-gating
- **What:** `/close` while an agent is actively running is refused (the person is told to `/cancel` first); `/close` and `/cleanup` are permitted only in a truly terminal state (`cancelled`, `failed`, `orphaned`) — the herdr `done` status is explicitly not sufficient, because `done` still has a live pane that can take more input.
- **Where:** described in CONTEXT.md D49/D50 (in-topic control commands, a later slice not yet realized in the four inventoried files)
- **Notable:** "stopping an agent (a real Ctrl+C) is always an explicit, separate action, never a side effect of closing a window." Also: a human deleting the Telegram topic under a live session does NOT orphan it (orphaned means the pane is gone, not the topic) — it's treated as channel loss per D48 instead. Decisions D49, D50.
- **Keywords:** terminal-state-only cleanup, done-is-not-closable, topic-deletion-is-not-orphaning

### full-command-surface-in-one-slice
- **What:** Milestone 3 was scoped to deliver the entire spec §7 Telegram command surface (long-polling, auth, general-topic commands, topic creation/routing, inline keyboards, and session-topic commands) in one high-risk slice, rather than a thinner end-to-end-first slice; the only notification M3 delivers is the stored orphan notice, with all real-time (§18) notifications held to M5.
- **Where:** described in CONTEXT.md D45
- **Notable:** "User chose the full surface over a thin end-to-end-first slice, having weighed the heavier single review pass. Planning sizes one high-risk slice, not a ladder." Decision D45.
- **Keywords:** single high-risk slice, full-surface-in-one-go, user-chosen scope

### environment-variable-blocklist-known-weak
- **What:** The `environment.allowed` mechanism for what env vars agents may receive is enforced as a blocklist of 17 process-hijacking variable names, and this shape is flagged in the decision record itself as wrong and incomplete (`GIT_INDEX_FILE`, `PERL5OPT`, `RUBYOPT`, `npm_config_*`, `CDPATH`, and the agents' own `*_HOME`/`*_CONFIG` vars are uncovered).
- **Where:** described in CONTEXT.md D19; realized in `internal/security` env-var filtering
- **Notable:** "Raised by the registry implementer against their own work: a blocklist can only deny what someone thought of... filed as a backlog item" rather than silently shipped as sound. Decision D19.
- **Keywords:** known-weak blocklist, self-raised gap, backlog-tracked fix

### sandbox-escape-hatches-refused-at-config-load
- **What:** Configuration keys that would fully disable an agent's sandbox (`codex_sandbox: danger-full-access`, `claude_permission_mode: bypassPermissions`, `capabilities.sudo: true`) are hard validation errors at config load, not merely discouraged or unused.
- **Where:** described in CONTEXT.md D18; realized in config validation
- **Notable:** "D1 made the agent sandbox plus the path allowlist the only containment; a config key that switches the sandbox off defeats the boundary from inside the very file that defines it." Decision D18.
- **Keywords:** hard-refuse at load, containment-of-last-resort, config-level escape hatch closure

## Locked decision IDs (D-number glossary)

| ID | One-line gloss |
|---|---|
| D1 | Service/herdr/agents run under the owner's own sudo-capable login account, not a separate `agenthost` user; canonical config/data paths follow from this. |
| D2 | First slice ships zero Telegram code; bot/supergroup creation are human steps done later. |
| D3 | Go project lives at the airemote repo root (`go.mod`, `cmd/`, `internal/`, etc.). |
| D4 | Slice 1 = Milestone 0 (discovery spike) + Milestone 1 (core foundation), hard stop between them. |
| D5 | Pinned compatibility baseline: Go 1.26.1, herdr 0.7.3, recorded agent versions and integration-install confirmation. |
| D6 | Path allowlist (`allowed_project_roots`) plus a hard, non-overridable deny list; Telegram can never supply a raw path. |
| D7 | Slice-1 done-bar is `go build/vet/test -race`; coverage thresholds and lint deferred past M1. |
| D8 | Every herdr invocation carries an explicit `--session` flag; `HERDR_SESSION` env var is never trusted. |
| D9 | `SendAgent` is a two-step, confirmed-landed submit (type, poll, then Enter), never a bare "type = delivered" call. |
| D10 | Codex's first-run trust prompt reports `idle`; readiness must be a positive composer match, never idle alone. |
| D11 | The allowlist governs agent-reachable paths only; the service's own config/data dirs are exempt, unifying D6 and D1. |
| D12 | Branch/session slugs are produced only by the sanitizer, never taken raw from a title or prompt. |
| D13 | The herdr socket event subscriber is deferred to Milestone 5; M1–M4 poll status via CLI. |
| D14 | An empty path allowlist is a hard config-load error — fail closed, never "allow everything". |
| D15 | Telegram `group_id`/`allowed_user_ids` aren't required at config load, but the gateway (M3) must refuse to start if unset. |
| D16 | TOCTOU on not-yet-existing paths is a known open hole in the M1 validator, deferred to M2 (point-of-use resolution). |
| D17 | The hard deny list is implemented exactly as D6 lists it, deliberately not widened (e.g. `~/.gnupg` not included yet). |
| D18 | Sandbox-disabling config keys (`danger-full-access`, `bypassPermissions`, `sudo: true`) are hard validation errors at load. |
| D19 | The env-var allowlist is enforced via a known-incomplete 17-name blocklist; flagged as an open weakness, not shipped as sound. |
| D20 | Canonical session id `ab-YYYYMMDD-NNN`; chat handle is any unique prefix, resolved like a short git SHA, ambiguous = ask not guess. |
| D24 | The machine never answers an unrecognized on-screen prompt — zero keystrokes sent; the session waits for a human. |
| D33 | Only M2-era states (`creating…orphaned`) are ever displayed; running-agent states (working/blocked/done) can't appear yet. |
| D35 | A task's row is written before its checkout/agent exist; a failed write aborts creation entirely. |
| D37 | Branch name is not part of a task's runtime identity — kept only for the on-disk id-collision check. |
| D38 | Runtime identity (workspace/pane/agent-target/native-session-id) is written in place as each becomes known; a deferred obligation is a durable event, not a log line. |
| D39 | The agent's native session id is a best-effort match key, empty at start, never a mismatch when one side is empty. |
| D41 | Exactly one service may hold the durable record; ownership is an exclusive lock taken at startup, second instance refuses to start. |
| D45 | M3 delivers the full Telegram command surface in one high-risk slice; only the stored orphan notice ships as real-time-ish notification. |
| D46 | An unauthorized message/button press is dropped in total silence, audited without content; callback presses are re-authenticated independently. |
| D47 | The one-time orphaned-work announcement is a proactive push at gateway startup, delivered at-least-once. |
| D48 | Losing the Telegram channel (kick/demote/send failure) is never fatal and never triggers a destructive action. |
| D49 | `/close` while an agent runs is refused; deleting the topic under a live session is channel loss, not orphaning. |
| D50 | `/close`/`/cleanup` are permitted only in a truly terminal state; herdr `done` (pane still alive) is not sufficient. |
| D51 | The `/new` concurrency cap is checked at invocation, before any button is shown; refusal names the bound cap. |
| D53 | The gateway durably owns its place (poll offset) in the Telegram message stream across restarts. |
| D54 | The Telegram client is a thin wrapper with zero other external module dependencies. |
| D55 | Non-poll Telegram calls use the normal request timeout; the long-poll `GetUpdates` uses its own poll timeout + margin. |
| D56 | Every refused chat action is written to a durable, content-free audit log entry. |
| D57 | Poll-offset saving and orphan-notification delivery marking both follow act-then-persist-then-fetch / send-then-mark ordering. |
| D58 | `/new`'s mode step offers only read-only and worktree-write; the no-isolation "use current workspace" mode is deferred. |
| D59 | A task's chat topic is named `<NNN> · <task summary> · <Agent>`. |
| D60 | A project's configured permission is a hard ceiling; the tapped mode can only narrow it, never widen it. |
| D61 | All three create outcomes (ready/needs_attention/failed) are surfaced in the task's topic. |
| D62 | Task creation is topic-first, fail-closed: mint id → create topic → create task record; topic-create failure aborts before any agent starts. |
| D64 | `/sessions` and `/status` read the durable record only, never the agent's live screen. |
| D65 | The topic id is written into the task row at creation time (create-ordering, implementation-level companion to D62). |
| D66 | A tapped button's payload is untrusted; every choice is re-validated server-side, fresh, at task-build time. |
| `68507c6a` | The `/health` summary is deliberately minimal — no host-uptime line — a settled choice, not a gap. |
| `565e68d0` | (referenced in project AGENTS.md, not in the four inventoried files) Independent review is user-invoked, not automatic per feature close. |
| `c2c46488` | (referenced in project AGENTS.md, not in the four inventoried files) A closed feature with approved gates still needing the intake-gate guard to block post-close edits. |

## Could not fully verify / uncertain

- `security-boundary.md`, `configuration.md`, `project-registry.md`, `herdr-integration.md`, `session-lifecycle.md`, and `docs/DISCOVERY.md` are referenced extensively by the four read files (especially for D9, D10, D13, D24, D43 readiness details, and the double-validated-worktree mechanism) but were out of scope for this pass and were not read — several "Notable" entries above cite them only as pointed-to context, not as independently verified content.
- Decision IDs `68507c6a`, `565e68d0`, and `c2c46488` appear as hash-style (non-`D`-numbered) decision references in the source material/AGENTS.md rather than the `D<n>` scheme; included in the glossary for completeness since the task asked for every locked decision ID encountered, but they are a different ID format than D46/D53/etc.
- The M3b "open questions for planning" and "pinned assumptions" subsections of CONTEXT.md (adapter-per-session selection, disambiguation prompt wording, wizard timeout) are forward-looking planning notes rather than settled mechanisms — listed only where they clarify a locked decision's scope, not inventoried as separate features since they are explicitly undecided.
- Did not independently verify the exact current line contents of `internal/telegram/client.go`, `cmd/agentbridge/newwizard.go`, etc. (existence confirmed via `find`, contents relied on solely from the specs' own "Pointers (implementation)" sections, which is a documentation claim about the code, not a direct code read).

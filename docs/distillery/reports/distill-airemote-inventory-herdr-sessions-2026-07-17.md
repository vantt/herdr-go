# Distill inventory: airemote / herdr integration + session lifecycle

Source repo: `upstreams/airemote` (Go project "airemote"/"AgentBridge" — Telegram bot driving
Codex/Claude Code coding-agent sessions inside the `herdr` terminal multiplexer via herdr's
CLI/socket API).

Files read in full:
- `docs/specs/herdr-integration.md` (576 lines)
- `docs/specs/session-lifecycle.md` (619 lines)

Cross-checked real source paths via `ls`/`find` against: `internal/herdr/*.go`,
`internal/agents/*.go`, `internal/sessions/*.go`, `cmd/agentbridge/*.go`.

This is mechanical inventory only — no porting judgment, no recommendation.

---

## Feature/mechanism entries

### session-naming-ab-id-scheme
- **What:** Every task/session gets a canonical id of the form `ab-YYYYMMDD-NNN` (e.g.
  `ab-20260714-001`) — date-scoped (in the configured local timezone, never UTC), sortable,
  never reused. This single string is reused as the branch name, the worktree directory name,
  the durable record key, and the name the runtime knows the agent by.
- **Where:** `internal/sessions/id.go` (`Generator`, `NewGenerator`, `Generate`,
  `MaxSessionSeqForDate`, `maxSeq = 999`); referenced from `internal/sessions/manager.go`,
  `internal/sessions/create.go`.
- **Notable:** The within-day counter `NNN` is read from the **durable record** (largest
  counter already used for that date, plus one) — never from an in-process counter — so a
  mid-day restart never reissues a number already handed out. Minting additionally **skips the
  counter forward** past any on-disk collision (branch already exists OR worktree directory
  already exists), because cleanup deliberately lets a worktree/branch outlive its session, so
  the durable record alone can be rebuilt while stale artifacts remain on disk — trusting only
  the record would hand a new agent an old branch. More than 999 sessions/day is a **loud
  error**, never a silent rollover into a malformed id.
- **Keywords:** D20, canonical id, within-day counter, collision skip, generate-check-skip loop.

### pre-minted-id-ahead-of-create
- **What:** The id-minting loop is exposed standalone (`NextID`) so a caller (the chat gateway)
  can mint an id — with no worktree cut — before the checkout exists, in order to name the
  task's chat topic first. The pre-minted id is then handed into `CreateWithID`.
- **Where:** `internal/sessions/manager.go` (`Manager.NextID`, `CreateWithID`);
  `internal/sessions/id.go`.
- **Notable:** "Minting an id does not reserve it." Two callers can mint the same counter
  simultaneously; the loser is caught later, inside `Create`, at the write-first durable-record
  step, which refuses a duplicate id — so the losing create fails **before the agent is
  started** and nothing is stranded. Runs every guard the normal create path does (write-first,
  readiness, state guard, directory guards).
- **Keywords:** D62, D65, NextID, CreateWithID, not-trusted-to-be-unique.

### session-isolation-dedicated-multiplexer-session
- **What:** All of a service's workspaces, checkouts, and agent terminals live inside one named,
  dedicated multiplexer session — never the person's own default/interactive session. Every
  runtime command names this session explicitly.
- **Where:** `internal/herdr/cli_client.go` (`New` refuses construction without a session name;
  `command()` prepends `--session` unconditionally; `run()` refuses an argv lacking
  `--session`); `internal/herdr/testdata/fakeherdr/main.go` (exits 64 on a session-less
  command).
- **Notable:** The herdr environment variable that the *original spec* said would scope
  commands to a session is **ignored by the real runtime** — setting it and believing the
  service is isolated is a live-observed failure mode: every workspace/checkout/agent gets
  created inside the person's own live terminal. Enforced by four independent layers so
  "forgetting" requires defeating all four (client construction, the single argv builder, the
  single process-exec funnel, and the test double's distinctive refusal).
- **Keywords:** D8, R1, `--session`, silent-default-session hazard.

### sending-is-not-submitting
- **What:** Telling the runtime to "send" text to an agent only types it into the agent's input
  box; the agent never sees it until a separate submit keystroke is sent to the terminal.
  Sending the submit keystroke too early loses the submission entirely (observed twice).
- **Where:** `internal/herdr/cli_client.go` (`SendAgent`, `confirmLanded`); `internal/herdr/client.go`
  (`SendRequest{Target, PaneID, Text}` — supersedes an earlier `SendAgent(ctx, target, text)`
  signature that could not express this); `internal/sessions/control.go` (`SendPrompt`).
- **Notable:** Mechanically three mandatory steps: (1) type the text, (2) **confirm it landed**
  by re-reading the terminal's visible screen (ANSI-stripped) until the tail of the sent text
  appears or a 3s confirmation deadline expires (polled every 100ms), (3) **only then** send the
  submit keystroke. An expired confirmation is a typed failure — "never reported as delivery" —
  and the unsubmitted text stays in the input box so retrying is safe. A terminal read failure
  while polling is *not* treated as proof the text is absent; polling continues to the deadline.
  A send request naming no terminal is refused outright rather than leaving text nobody can submit.
- **Keywords:** D9, R2/R3, send-confirm-submit, ErrSendNotConfirmed, submit-confirmation deadline.

### default-deny-readiness-composer-match
- **What:** A session/agent is judged "ready" only when its on-screen composer shows a single,
  non-enumerated input field, with no enumerated options anywhere on screen and no activity
  indicator. Every other screen shape — menu, question, mid-work screen, or one the build
  doesn't recognize — is not ready and parks the session for a human.
- **Where:** `internal/agents/adapter.go` (`ComposerMatcher` per agent); `internal/sessions/readiness.go`
  (`awaitReady`, `enterNeedsAttention`); `internal/agents/testdata/` (captured screens);
  `internal/agents/adapter_test.go` (`TestComposerAgainstRealScreens`,
  `TestDefaultDeny_UnseenScreenShapes`).
- **Notable:** Each agent draws its **menus** with the same glyph its **composer** uses (the
  glyph in front of an enumerated option is a selection cursor, not a prompt), and leaves that
  same glyph on screen while working. A rule built on "the composer glyph is present" therefore
  called a **blocked** agent ready — not theoretical: one submit keystroke into a screen the
  service had wrongly declared ready selected a menu option that ran a pipe-to-shell installer.
  The fix is default-deny matched on **shape** (how many composer-led lines, whether an option
  is enumerated, whether an activity line is present, whether the glyph sits inside a box or
  leads a list) rather than enumerating known-bad menus — an enumerated-menus approach would
  only ever recognize menus already seen, and every release can add one.
- **Keywords:** D43, D25, D24, D22, D10, R15/R15a/R15b, needs_attention, ready screen.

### per-agent-launch-adapters
- **What:** An `AgentAdapter` interface (`Kind`, `BuildStartCommand`, `NormalizeOutput`,
  `BuildCancelKeys`, `Composer`) is implemented once per supported coding agent (Codex, Claude),
  hiding differences in how each launches, signals readiness, and is interrupted.
- **Where:** `internal/agents/adapter.go` (interface), `internal/agents/codex.go` (`CodexAdapter`),
  `internal/agents/claude.go` (`ClaudeAdapter`), `internal/agents/fake.go` (`FakeAgentAdapter`).
- **Notable:** Concrete differences captured in the spec: Codex launches as
  `codex --sandbox {workspace-write|read-only} --ask-for-approval on-request -c <trust>`; Claude
  launches as `claude --permission-mode {default|plan}`. Composer glyphs differ and are verified
  live: Codex uses `›` (U+203A), Claude uses `❯` (U+276F). `codexComposer` requires exactly ONE
  `›`-leading line with nothing enumerated and no activity line; `claudeComposer` requires
  exactly ONE `❯`-leading line **inside a rule-delimited box** with nothing enumerated (an older,
  looser "glyph anywhere between two rules" form accepted a boxed menu and was tightened). Only
  Codex has the trust-flag mechanism (`-c` inline TOML table); Claude has its own distinct
  first-run prompt (importing configuration from outside the working directory) that is *not*
  pre-empted and instead falls to needs_attention.
- **Keywords:** D25, D44, AgentAdapter, BuildStartCommand, Composer, agent kind.

### directory-trust-flag-preemption
- **What:** For the one agent (Codex) with a first-run "do you trust this directory?" question,
  a per-launch command-line flag pre-clears the question so a normal launch reaches the composer
  instead of stopping on the prompt. The flag names the repository, not the task's checkout.
- **Where:** `internal/agents/codex.go` (`codexTrustConfig`, builds
  `projects={"<repo-root>"={trust_level="trusted"}}`); `internal/sessions/create.go` (passes
  `project.RepositoryPath` as the trust value while the agent's CWD stays the checkout);
  `internal/agents/adapter_test.go` (`TestCodexTrustOverride_PathTravelsInTheValueHalf`,
  `TestSessionTrustKeyIsRepositoryRoot`).
- **Notable:** Two facts were established by **experiment against the real agent**, not by
  reading documentation: (1) Codex resolves trust at the **repository**, not at a linked
  checkout — a task's checkout is a linked worktree whose `.git` is a pointer file back to the
  main repo, so Codex walks up and asks about the repository on the prompt itself; a flag naming
  the checkout pre-empts nothing (every fresh checkout still met the prompt under the old
  design). (2) Codex's `-c` flag splits its **name** half on every separator with no
  quote-awareness, so a path placed there gets torn apart; only the **value** half is
  TOML-parsed. So the repository path must live in the value half, keyed by repository, as a
  structured entry. The flag is per-launch only: never a keystroke, never written into the
  person's own `~/.codex/config.toml` (verified identical md5 before/after). The earlier
  recorded mechanism (D23/D31) was written, reviewed, and shipped — and **never actually
  suppressed the prompt**; D44 is the replacement mechanism, proven by exec'ing the real argv
  against a fresh worktree of a never-trusted repo.
- **Keywords:** D23, D44, D31, trust flag, value-half vs name-half, projects table, TOML.

### trust-flag-value-sanitization
- **What:** The repository path embedded in the Codex trust flag is untrusted input (a
  read-only task may point at an operator-configured path, not a fresh sanitized worktree), so
  it is escaped or rejected before being placed into the flag.
- **Where:** `internal/agents/codex.go` (`codexTrustConfig`).
- **Notable:** Two real behaviors of Codex, both proven against the live agent, drive the rule:
  a raw quote character would break out of the quoted key (so quotes are **escaped**); Codex
  splits the flag on its **first equals sign** before parsing the value, so an equals sign
  inside the path lands in the wrong half and corrupts the setting — escaping cannot help
  because the split happens too early, so an equals sign is **rejected** outright, along with
  control characters, newline, tab, and NUL (all "cannot be represented safely").
- **Keywords:** D31, escape-vs-reject, first-equals-sign split.

### sandbox-safe-launch-flags-only
- **What:** Each adapter can only select one of two safe permission/sandbox levels when building
  an agent's launch command line — the normal write-enabled mode, or a restricted read-only
  mode. A setting that would disable the sandbox entirely cannot be produced by any input.
- **Where:** `internal/agents/codex.go`, `internal/agents/claude.go` (adapter command-line
  construction); implied guard verified via adapter tests (`internal/agents/adapter_test.go`).
- **Notable:** "The sandbox-disabling values are not expressible at all — because those values
  are written **nowhere** in the adapter, not merely guarded against." An independent probe
  feeding a hostile repository value confirmed the sandbox-disabling tokens never appear as a
  standalone flag, only as inert data inside the trust key. Rationale given: under the product's
  identity model, the sandbox is one of only a few things standing between an agent and the
  whole machine, so a launch flag able to turn it off would defeat the boundary from the inside.
- **Keywords:** D18, safe launch flags, read-only downgrade, write-enabled vs read-only.

### no-event-subscription-status-polling
- **What:** The runtime has a push-event channel, but the service does not use it. Agent status
  (`idle`/`working`/`blocked`/`done`/`unknown`) is obtained only by asking (polling), never by
  subscription.
- **Where:** `internal/herdr/errors.go` (`ErrNotImplemented` for Subscribe);
  `internal/sessions/manager.go` (`defaultPollInterval = 500ms`).
- **Notable:** The request to subscribe deliberately fails with an explicit "not implemented" so
  nothing can accidentally come to depend on events that are never delivered. The
  subscriber, reconnection, snapshot-on-reconnect, and duplicate-event suppression are all
  named as future work (the event-subscriber milestone). Separately, the runtime is observed to
  fire the **same status change twice in the same millisecond** — whatever eventually consumes
  status changes must de-duplicate, or a person is notified twice for one event.
- **Keywords:** D13, R5, ErrNotImplemented, polling, event-subscriber milestone, duplicate events.

### idle-status-ambiguity
- **What:** The runtime's `idle` agent status is reported both when an agent is sitting blocked
  at a first-run prompt awaiting a person AND when it is genuinely ready at its composer — the
  status alone carries no information about readiness, so it is never trusted for readiness
  decisions.
- **Where:** Documented behavior underlying `internal/sessions/readiness.go` (readiness ignores
  agent status entirely and reads only the terminal pane).
- **Notable:** "A coding agent sitting on a first-run trust question reports as idle —
  indistinguishable from finished." Sending an instruction while an agent waits on such a
  prompt was observed to **kill the agent**. This single ambiguity is one of the three headline
  safety findings called out at the top of `herdr-integration.md`, alongside session isolation
  (D8) and send-not-submit (D9).
- **Keywords:** D10, idle vs ready, blocked-vs-finished ambiguity, trust-prompt hazard.

### keystroke-state-guard
- **What:** Any operation that sends a keystroke to an agent — delivering a prompt, cancelling —
  is refused unless the session's current state is `ready`. The check happens before anything
  reaches the terminal runtime, and is enforced by session **state**, not by the order in which
  the creating code happens to call things.
- **Where:** `internal/sessions/state.go` (`requireReady`, `StateError`/`ErrNotReady`,
  `ErrUnknownSession`); `internal/sessions/control.go` (`SendPrompt`, `Cancel` both call
  `requireReady` before touching the client).
- **Notable:** "Sequencing is not a guard; state is." During creation the step order already
  keeps keystrokes off an unrecognized prompt, but the moment any *other* caller (a future state
  machine, a future chat command) sends a prompt or cancel to a session parked in
  `needs_attention`, sequencing protects nothing — so the guard lives on the session's state
  instead, meaning a keystroke can never land on a `needs_attention` session (where a
  pipe-to-shell update menu may be on screen).
- **Keywords:** D28, R6, requireReady, ErrNotReady, needs_attention.

### unrecognized-prompt-zero-keystroke-park
- **What:** When readiness polling reaches its startup deadline without matching the recognized
  ready screen, the session enters `needs_attention`: the pane content is captured for a human,
  and zero keystrokes are ever sent. The human decides what to do.
- **Where:** `internal/sessions/readiness.go` (`awaitReady`, `enterNeedsAttention`);
  `internal/agents/adapter_test.go` (`TestSessionStartedOntoMenu_NeedsAttentionZeroKeys`).
- **Notable:** "Startup prompts cannot be enumerated." One agent's update-available prompt is
  time-throttled (appears on a schedule, not in response to anything the service does) and its
  **first option runs a pipe-to-shell command** — so blindly pressing a key into an unknown menu
  is dangerous. Any "detect prompt X and answer it" design is guaranteed to break the day a new
  prompt appears. The only pre-empted exception is the directory-trust prompt (via the launch-time
  trust flag, never a keystroke). Asserted end-to-end in tests as an **absence** on the key log,
  never merely in prose.
- **Keywords:** D24, D22, B4, R5, pipe-to-shell hazard, needs_attention.

### two-step-cancellation
- **What:** Cancelling a session sends an interrupt keystroke to the terminal; the first
  interrupt stops the agent's current work and leaves the terminal alive, and — after a grace
  period — a second interrupt exits the agent and closes the terminal. The agent process is
  never force-killed.
- **Where:** `internal/sessions/control.go` (`Cancel`: two `SendKeys` interrupts with
  `cancelGrace` between; no `kill -9` path anywhere).
- **Notable:** Verified live against a real agent: "the first interrupt stops the work and
  leaves the terminal alive; a second one exits the agent and closes the terminal. No forcible
  kill is needed." Cancellation is refused on any non-ready session by the keystroke state guard,
  so it can never blind-fire an interrupt into an unrecognized prompt either.
- **Keywords:** D9 (adjacent), B7, cancel grace, two-stage interrupt, no forcible kill.

### durable-session-record-write-first
- **What:** A session's durable record is written to storage **before** the working-directory
  checkout is cut and before the coding agent is started. If that write fails, session creation
  is abandoned entirely — no directory, no checkout, no agent left behind.
- **Where:** `internal/sessions/create.go` (`Create`); `internal/sessions/record.go`
  (`toRecord`/`fromRecord`/`metaFromRecord`); `internal/store/sqlite/` (backing store, implied
  by `persist_test.go`).
- **Notable:** "The consequence is accepted deliberately — while the durable record is unusable,
  no new session can be created — because it makes 'a live agent the record does not know
  about' structurally impossible, and that hole is the one thing that would make everything
  below pointless." Once the agent is live, a **subsequent** failure to persist a state change
  never kills the live agent — the failure is reported loudly and the record is merely "behind,
  not absent," survivable because recovery re-matches on identity keys that don't go stale.
- **Keywords:** D35, R9, write-first, creating state, never-a-phantom-agent.

### restart-recovery-single-snapshot-evidence
- **What:** On service restart, recovery loads every stored session that was still "open," takes
  exactly **one** snapshot of everything the runtime currently has alive, and decides every
  stored session against that single picture — it never re-asks the runtime task by task.
- **Where:** `internal/sessions/recovery.go` (`Reconcile([]store.Session, *herdr.Snapshot)`,
  `Manager.Recover()`); `cmd/agentbridge/main.go`/`debug.go` (startup order: lock → open+migrate
  store → herdr client → manager → recovery before ready).
- **Notable:** "One snapshot is the only evidence of what is alive" — because "I cannot reach the
  runtime" and "the runtime has nothing alive" are **structurally distinct answers**: an
  unreachable runtime fails the command outright and returns no picture at all, while a reachable
  but empty runtime returns a perfectly valid picture with nothing in it. Only a *successful*
  snapshot may support the conclusion that a terminal is gone; recovery is proven **read-only**
  (closes no terminal, removes no checkout, sends no keystroke, mints no duplicate id) — in a
  real restart against a real agent the terminal's visible contents were byte-for-byte identical
  before and after.
- **Keywords:** D34, D36, B8/B11, R11/R12/R16/R18, Reconcile, snapshot-once.

### unreachable-vs-empty-runtime-distinction
- **What:** A failed attempt to reach the runtime is never treated as evidence that a terminal
  is gone. A failed snapshot is retried with backoff up to a 30-second unavailability bound;
  past that, the service reports the runtime unavailable and concludes/orphans nothing.
- **Where:** `internal/sessions/recovery.go` (`ErrRuntimeUnavailable` after the 30s retry bound).
- **Notable:** Probed live on herdr 0.7.3: `herdr --session no-such-session api snapshot` →
  `Error: Os { code: 2, kind: NotFound }`, exit 1, **no JSON**; a live but empty session →
  `{"agents":[],"panes":[],"workspaces":[],"protocol":16}`. "Treating a failure as an empty
  picture would silently declare every live task's terminal gone, which is a lie the person then
  acts on." Note the CLI verb is `api snapshot`, not `session snapshot` (which doesn't exist in
  0.7.3).
- **Keywords:** D36, R11/R16, ErrRuntimeUnavailable, ~30s bound, ambiguity resolution.

### orphaned-session-state
- **What:** A terminal `orphaned` state, set only during recovery, meaning the stored session's
  terminal is genuinely gone (including reused by someone else's agent) but its checkout/work is
  still on disk.
- **Where:** `internal/sessions/state.go` (`StateOrphaned`).
- **Notable:** Deliberately **not** `failed` — "failed means the agent broke or never ran, and
  conflating the two would license a cleanup path to delete a real, unfinished checkout." The
  obligation to tell the person is recorded **durably** as its own record (not a log line), and
  is delivered by the chat gateway the next time the service starts — one message to the group's
  main topic, at-least-once (a crash between announcing and recording resends rather than
  loses it). Delivery is not real-time: a session orphaned while the gateway is already running
  is announced only at the *following* startup.
- **Keywords:** D38, D40, D47, orphaned, at-least-once notification.

### recovery-non-orphan-non-match-outcomes
- **What:** Four distinct recovery outcomes exist that are neither a successful match nor
  `orphaned` — each keeps the session entirely out of service (asking for it returns "unknown
  session," unreachable by construction, not merely refused by a state check).
- **Where:** `internal/sessions/recovery.go` (outcome tags: matched / orphaned / agent-gone /
  never-started / refused-path / foreign-session / unadopted-panes).
- **Notable:** The four cases and their reasoning: (1) stored working directory no longer
  resolves inside an allowed root (a symlink was swapped while the service was down) →
  `needs_attention` with the refusal recorded — not orphaned, because the terminal may be alive,
  only the *path* is untrustworthy. (2) a record with no terminal/workspace/agent target (crash
  between record-write and agent-start) → `failed` ("agent never started") — not orphaned,
  because calling a phantom orphaned would license deleting work that never existed. (3) a
  record naming a different runtime session than the one this service is configured for →
  nothing concluded, nothing written (else a successful look at *another* session would
  mass-orphan everything while technically satisfying "only a successful look"). (4) terminal
  still exists but no agent is running in it → parked in `needs_attention`, not orphaned, because
  the terminal visibly is not gone. A live terminal that no stored session claims is reported as
  a warning and never adopted.
- **Keywords:** D40, refused-path, foreign-session, never-started, unadopted-panes.

### runtime-identity-match-keys
- **What:** A stored session is re-attached to a live agent after restart by exactly four values:
  agent target (= the session's own canonical id), the coding agent's own native session id
  (best-effort), the terminal id, and the workspace id — and no others.
- **Where:** `internal/sessions/recovery.go` (`Reconcile`); `internal/sessions/record.go`.
- **Notable:** The four keys split into two roles. **Identity keys** (agent target; native
  session id) identify a live agent as ours and can **refute** a claim otherwise. **Locator
  keys** (terminal id; workspace id) only **corroborate** a match and are **refreshed** from
  whatever the runtime currently reports — a stale stored terminal id never refutes a live agent
  that still carries our agent target, "because the record is allowed to lag the runtime by
  design." The native session id is empty when the agent starts and appears later, so it is
  used only when both sides carry a value; an empty value on either side is never a mismatch.
  **The branch is never a match key** at all — the runtime's picture of what's alive carries no
  branch information whatsoever (verified: `internal/herdr/snapshot.go`'s `toWorkspace` never
  sets `Branch`; it's populated only from the worktree-create response) — so a restart cannot
  and does not match a stored task on its branch.
- **Keywords:** D37, D39, agent target, native session id, terminal id, workspace id, identity-vs-locator.

### worktree-double-validated-target-directory
- **What:** The agent's working directory is validated twice: once when the service itself
  creates it (a no-follow create, re-checked to be inside an allowed root), and again on
  whatever directory the runtime's checkout tool actually reports back after cutting the
  worktree.
- **Where:** `internal/sessions/create.go` (guard one: `security.Validator.SafeCreateDir`; guard
  two: `security.Validator.Validate(ws.CheckoutPath)` with `RemoveWorktree` teardown on escape).
- **Notable:** Necessary because the runtime's checkout tool follows a symbolic link at an
  ancestor path component **even though the service's own no-follow creation did not** — verified
  live on the host. If the returned directory escapes the allowed roots, the worktree is torn
  down and the session is marked `failed`; the agent's working directory (and the record's
  stored working directory) is set only from the path that cleared the *second* check, never the
  pre-guard-two one.
- **Keywords:** D16, D27, D29, D39, guard one/guard two, SafeCreateDir, RemoveWorktree teardown.

### branch-name-slug-validation-last-gate
- **What:** A worktree-creation request's branch name is validated against the slug sanitizer
  rule at the last gate before the value leaves the service, refusing anything that isn't
  exactly what the sanitizer would itself have produced.
- **Where:** `internal/herdr/cli_client.go` (`CreateWorktree` + `validateBranch`,
  segment-by-segment; runs before any arg assembly, so a hostile branch is refused with zero
  runtime invocations).
- **Notable:** Live-verified against the real runtime: a branch name of `../../../.ssh` is
  refused; "without the gate, the runtime would create it verbatim." The conventional two-segment
  `agent/<slug>` form survives because each segment is checked independently.
- **Keywords:** D12, R4, validateBranch, slug sanitizer, path traversal refusal.

### command-argv-no-shell-execution
- **What:** All runtime commands (and an agent's launch command line) are executed as separate
  argv elements handed directly to the process — never assembled into a string that a shell
  would re-parse.
- **Where:** `internal/herdr/cli_client.go` (`command()`, `run()`); `internal/agents/*.go`
  (adapters build command lines as arg slices).
- **Notable:** Stated as R8 ("Runtime commands are executed as separate arguments, never through
  a shell") and reiterated for agent launch in B9/B5 — the command line is "passed as separate
  arguments — never assembled into a string handed to a shell."
- **Keywords:** R8, argv, no-shell, injection avoidance.

### error-reporting-redaction
- **What:** Every runtime command failure is reported carrying the logical command name (never
  the full command line, which can contain a person's private instruction text), the exit
  status, and the runtime's own error output passed through a single shared redactor.
- **Where:** `internal/herdr/errors.go` (`CommandError` carries command name, exit code,
  redacted stderr; `ErrMissingSession`, `ErrMissingPaneID`, `ErrSendNotConfirmed`,
  `ErrIncompatible`, `ErrInvalidBranch`, `ErrMalformedOutput`).
- **Notable:** Both of the runtime's structured error output streams are examined "as structured
  data, never as text patterns" — the runtime reports failures as structured errors on either
  stream depending on failure class.
- **Keywords:** R10, CommandError, redactor, structured-error-parsing.

### protocol-version-compatibility-check
- **What:** At startup the service takes a snapshot and compares the runtime's wire **protocol**
  version number against a pinned value this build was verified against — not the runtime's
  human-readable marketing version string, which can move independent of the protocol.
- **Where:** `internal/version/version.go` (`HerdrProtocol = 16`, with a test that fails if it
  changes without re-running the discovery spike).
- **Notable:** A mismatch is a typed, reportable error — never a crash and never a silent
  "probably fine." Verified baseline recorded: herdr 0.7.3, socket protocol 16; codex-cli 0.144.3;
  Claude Code 2.1.208.
- **Keywords:** D5, R6, HerdrProtocol, wire protocol vs marketing version.

### unrecognized-status-degrades-to-unknown
- **What:** Exactly five agent status values exist (`idle`, `working`, `blocked`, `done`,
  `unknown`); any runtime-reported status value this build does not recognize degrades to
  `unknown` rather than crashing the polling loop.
- **Where:** Implied in `internal/herdr/client.go` (`AgentStatus`); `internal/sessions/manager.go`
  polling path.
- **Notable:** "A new runtime state must degrade to 'we do not know', never crash the polling
  loop." `blocked` is the only status that earns a person a notification (in the not-yet-built
  notification design).
- **Keywords:** R7, AgentStatus, unknown, graceful degradation.

### deadline-and-poll-interval-tuning
- **What:** Four distinct timing constants govern runtime interaction: 15s ordinary command
  deadline, 45s agent-start deadline (deliberately longer), 3s submit-confirmation deadline, and
  a 100ms confirmation poll interval.
- **Where:** `internal/herdr/client.go` (`Timeouts`).
- **Notable:** The submit-confirmation deadline is explicitly "not a deployment setting: it is a
  property of how fast a terminal echoes typed text, not of the machine" — distinguishing it
  conceptually from ordinary tunable timeouts.
- **Keywords:** command deadline, agent-start deadline, submit-confirmation deadline, poll interval.

### per-task-agent-choice
- **What:** Which coding agent (Codex or Claude) runs a given task is a per-task choice carried
  in the session-create request, not one agent fixed for the whole service. An unnamed or
  unrecognized choice falls back to the service's configured default agent.
- **Where:** `internal/sessions/manager.go` (`Config.Adapter` as default/fallback,
  `Config.Adapters map[agents.AgentKind]agents.AgentAdapter`; `Create` resolves from
  `req.AgentKind`); `internal/sessions/adapter_dispatch_test.go`.
- **Notable:** Implemented as an additive change (M3b): the three existing call sites that set
  `Config.Adapter` compile and pass unchanged even after per-task dispatch was added — an
  explicit backward-compatibility property called out in the spec.
- **Keywords:** D63, AgentKind, per-task dispatch, default fallback.

### isolation-mode-narrowing-only
- **What:** A session-create request also carries a chosen isolation mode (read-only vs
  worktree-write). This is reconciled against the target project's configured permission and can
  only **narrow** it, never widen it — the effective mode used is always the more restrictive of
  the two.
- **Where:** Enforced by the chat gateway before create is called (`telegram-gateway.md` R12,
  referenced from `session-lifecycle.md`); consumed in `internal/sessions/create.go`.
- **Notable:** A create request naming a write mode against a read-only-configured project is
  narrowed to read-only *before* create is even invoked — create itself only ever selects a
  "safe" sandbox (read-only or worktree-write), never a disabled one (ties into D18).
- **Keywords:** D58, D60, R16, narrow-never-widen.

### task-topic-identity-in-create-request
- **What:** The chat topic identifying a task's conversation is carried directly in the
  session-create request and persisted with the durable record in the very same write that
  creates the session — there is no later, separate step that attaches the topic afterward.
- **Where:** `internal/sessions/manager.go` (`CreateRequest.TelegramThreadID`, "persisted by the
  birth-row write").
- **Notable:** Explicitly framed against an alternative design (attach the topic in a later
  step) that was rejected — "there is no later step that attaches the topic."
- **Keywords:** D65, TelegramThreadID, birth-row write, single-write persistence.

### single-service-durable-record-lock
- **What:** At startup, a service takes exclusive ownership of its durable record (a file lock)
  and refuses to start if another live service instance already holds it.
- **Where:** `internal/store/sqlite/lock.go` (`LockDir`, `flock` on the data directory).
- **Notable:** "Not tidiness": two services over one record would both recover the same sessions
  and both drive the same live agent, so one service's cancel would fire an interrupt into the
  *other's* working agent and kill it. The keystroke state guard (D28) cannot help here because
  it knows a session's state, not who owns the process. A crash releases the lock with no
  cleanup step (released by the OS on SIGKILL); the next process takes it immediately. Error
  surfaced: "database directory is locked by another agentbridge instance."
- **Keywords:** D41, R15, LockDir, flock, one-service-per-record.

### agent-outlives-service
- **What:** The coding agent process lives inside the terminal runtime, not inside the
  AgentBridge service — killing or crashing the service does not kill or disturb any running
  agent. Recovery re-attaches to the exact same live agent from a single snapshot without typing
  anything.
- **Where:** `internal/sessions/recovery.go`; cross-referenced in both spec files as a proven,
  not assumed, property.
- **Notable:** Proven with a real two-process experiment: an agent survived its gateway process
  being `kill -9`'d, and a completely fresh service process found the same session again — same
  session id, same terminal, same workspace, same checkout, with the pane diff **empty** (no
  bytes changed). What is explicitly *not* yet probed: what happens to live agents (and a
  connected service) when the **herdr runtime itself** restarts — recorded as an open, unresolved
  risk, not assumed away.
- **Keywords:** D34, R18, service-death-survival, kill -9 proof, unprobed-runtime-restart gap.

### composer-testdata-ground-truth
- **What:** The readiness-matching rule is judged, in tests, against verbatim captured real
  terminal screens from the real coding agents rather than synthetic or invented fixtures.
- **Where:** `internal/agents/testdata/` (`codex_trust_prompt.txt`, `codex_composer_ready.txt`,
  `codex_working.txt`, `claude_composer_ready.txt` — all read through the product's own
  `ReadPane` + `StripANSI` path in an isolated throwaway herdr session);
  `internal/agents/adapter_test.go`.
- **Notable:** `TestComposerAgainstRealScreens` runs every fixture against both adapters with a
  **completeness assertion** that fails if a new fixture is ever added and never judged.
  `TestDefaultDeny_UnseenScreenShapes` separately tests 10 menu/prompt shapes that **no fixture
  contains**, proving the rule denies by default rather than merely remembering a list. One
  screen — an agent's update-offer prompt — could not be re-captured (the test machine's agent
  is already at its newest version, and downgrading a person's agent to reproduce it was
  refused); it is documented from the evidence of the run where it caused harm and explicitly
  never presented as a captured screen.
- **Keywords:** D43, ReadPane, StripANSI, TestComposerAgainstRealScreens, documented-not-captured.

### ansi-stripping-scanner
- **What:** Terminal output is stripped of ANSI/terminal control codes before being compared
  against expected text or matched against composer shapes, using a purpose-built scanner rather
  than a regular expression.
- **Where:** `internal/herdr/snapshot.go` (`StripANSI`).
- **Notable:** Explicitly built as "a scanner, not a regex" because "OSC sequences have two legal
  terminators" — a detail a naive regex would mishandle.
- **Keywords:** StripANSI, OSC sequences, ANSI stripping.

### mutation-tested-guards
- **What:** Every safety guard in the herdr-integration area was mutation-tested — deliberately
  broken to confirm the test suite catches the break, then restored.
- **Where:** Referenced generally across `internal/herdr/*_test.go`, `internal/agents/adapter_test.go`.
- **Notable:** Called out as a blanket verification practice applied to "every guard in this
  area," not a specific single test.
- **Keywords:** mutation testing, guard verification.

### worktree-symlink-follow-hazard
- **What:** The herdr runtime's checkout-creation tool follows a symbolic link at its target
  directory (and at ancestor path components), a behavior the service's own directory-creation
  step does not exhibit — meaning naming a safe target directory is not by itself a guarantee.
- **Where:** `internal/security/` (validator, referenced as `security.Validator.SafeCreateDir`
  and `security.Validator.Validate`); `internal/sessions/create.go`.
- **Notable:** Verified on the actual host, not assumed. This is the mechanical reason the
  create sequence needs guard one (no-follow creation) **and** guard two (independent
  re-validation of whatever the runtime reports back) — "naming the target and re-checking the
  result are two halves of one guard."
- **Keywords:** D27, D29, symlink-follow, guard-one/guard-two.

### worktree-target-directory-explicit
- **What:** Every worktree-creation request names the exact target directory the runtime must
  cut the checkout into — the service safe-creates that directory itself first and passes it —
  rather than letting the runtime pick a location by its own undocumented default.
- **Where:** `internal/herdr/client.go` (`CreateWorktreeRequest.Path`, sent as `--path`).
- **Notable:** "Without it, the runtime picks the location by an undocumented default and
  nothing validates where it lands." `--path` flows through the same args slice as the session
  name, so it inherits the leading `--session` prepend automatically.
- **Keywords:** D27, D29, CreateWorktreeRequest.Path, `--path` flag.

---

## Locked decision IDs — one-line gloss

| ID | Gloss |
|---|---|
| D5 | Compatibility is pinned to the runtime's wire protocol number, checked at startup — not the marketing version string. |
| D8 | Every runtime command must name its session explicitly; the runtime ignores the env var meant for this, so the client/command-builder/executor/test-double all refuse session-less commands. |
| D9 | Sending text to an agent is not submitting it: mandatory type → confirm-on-screen → submit sequence; an unconfirmed send is never reported delivered. |
| D10 | `idle` status never means "ready" — it's reported both when blocked at a first-run prompt and when genuinely ready at the composer. |
| D12 | A worktree branch name must exactly match what the slug sanitizer would itself produce; enforced as the last gate before leaving the service. |
| D13 | No event subscription exists yet; the service polls for status instead, and Subscribe explicitly returns "not implemented." |
| D16 | The working directory is created with a no-follow operation (guard one of two). |
| D18 | Sandbox-disabling launch flag values exist nowhere in any adapter's code — not expressible by any input. |
| D20 | Canonical session id `ab-YYYYMMDD-NNN`: local-timezone date, durable-record-sourced counter, on-disk collision skip. |
| D22 | The readiness rule never matches on prompt wording, only on screen shape, so unseen prompts are refused by default. |
| D23 | A per-launch trust flag pre-empts Codex's first-run trust prompt; never a keystroke, never written into the user's own config file. |
| D24 | An unrecognized screen sends zero keystrokes and parks the session in `needs_attention` for a human. |
| D25 | Each agent's composer glyph differs and is not shared; the glyph alone is never sufficient evidence of readiness. |
| D26 | Claude's `blocked` transition (idle → working → blocked, ~6s) is confirmed live, closing a prior open item. |
| D27 | The runtime's checkout tool follows a symbolic link at the target/ancestor even though the service's own no-follow create does not — requires re-validation of the returned path (guard two). |
| D28 | Keystroke-sending operations (send prompt, cancel) are refused unless session state is `ready` — a state guard, not sequencing. |
| D29 | A checkout location that fails the second validation is torn down (worktree removed) and the session marked `failed`. |
| D30 | The safe directory-creation guard is not atomic; a race yields repeated refusal (denial of service), never an escape. |
| D31 | The trust-flag value is escaped (quotes) or rejected (`=`, control chars, newline, tab, NUL) because Codex splits on the first `=` before parsing. |
| D33 | The running-agent state machine (working/blocked/done/idle) and notifications are not yet built — deferred to the event-subscriber milestone. |
| D34 | Restart recovery is read-only against the runtime (one snapshot, no other command); the service's death never kills a live agent. |
| D35 | The session's durable record is written before the checkout is cut and the agent starts; a failed write aborts creation entirely. |
| D36 | An unreachable runtime is never treated as evidence of an empty/gone terminal; only a successful snapshot can support "gone." |
| D37 | The runtime's picture of what's alive carries no branch information; branch is never used as a recovery match key. |
| D38 | The obligation to notify a person about an orphaned session is recorded as a durable record, never just a log line. |
| D39 | Record fields are filled honestly (never placeholders) and overwritten with the doubly-validated path once guard two clears. |
| D40 | Four recovery outcomes distinct from matched/orphaned: refused-path, failed/never-started, foreign-session, terminal-with-no-agent. |
| D41 | Only one service instance may own the durable record at a time (file lock); a second instance refuses to start. |
| D42 | The fix-first defect slice that repaired the composer-glyph-alone bug and the never-working trust-override mechanism. |
| D43 | Default-deny readiness: ready only on a recognized-good single non-enumerated composer screen with no activity indicator; judged against real captured screens. |
| D44 | The Codex trust flag names the repository root (not the checkout) and carries the path in the setting's value half, since the name half is naively split. |
| D47 | Orphan notifications are delivered by the chat gateway at next startup, at-least-once. |
| D58 | The isolation mode (read-only vs worktree-write) is a per-task choice carried in the create request. |
| D60 | The chosen isolation mode can only narrow the project's configured permission, never widen it. |
| D62 | A session id can be minted ahead of the create sequence (`NextID`) without cutting a worktree. |
| D63 | The coding agent (Codex/Claude) is a per-task choice, falling back to the service default when unnamed/unrecognized. |
| D65 | The task's chat topic identity is carried in the create request and persisted in the same write as the record, no later attach step. |

---

## Unresolved / could not fully verify

- Both spec files (576 + 619 lines) were read in full; no truncation.
- Source-path cross-check was done via `ls`/`find` listing of directory contents only (file
  names, not full file bodies) for `internal/herdr/`, `internal/agents/`, `internal/sessions/`,
  `cmd/agentbridge/`, and top-level `internal/`/`cmd/` — every file path cited in this report
  matches a real file name observed in those listings. Function/symbol names inside those files
  (e.g. `awaitReady`, `requireReady`, `codexTrustConfig`) are taken verbatim from the specs'
  "Pointers (implementation)" sections and were **not** independently re-verified by opening
  each `.go` file's body.
- Two items the source documents itself as open/unverified (carried through above, not
  independently checked further): (1) the submit-confirmation read-the-visible-screen behavior
  is proven only for Codex's input box, unproven for Claude's, which may render differently; (2)
  notification debouncing for duplicate status-change events is described as required but "not
  built."
- `docs/specs/telegram-gateway.md` and `docs/specs/persistence.md` are referenced repeatedly by
  both files read (e.g. B7/B9/R12/R14 cross-references, topic persistence, short-handle
  resolution) but were **not** part of this task's scope and were not opened.

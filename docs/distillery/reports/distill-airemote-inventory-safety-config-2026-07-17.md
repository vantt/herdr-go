# airemote (AgentBridge) inventory — safety & config — distill extraction report

Source: local clone at `upstreams/airemote`. Files read in full:
`docs/DISCOVERY.md` (148 lines), `docs/specs/security-boundary.md` (366 lines),
`docs/specs/configuration.md` (246 lines), `docs/specs/project-registry.md` (266 lines).
Method: full read of all four files, no partial/paginated reads. Mechanical inventory
only — no porting judgment, no cross-source comparison.

Project context: AgentBridge is a Go Telegram bot that drives Codex/Claude Code coding-agent
sessions running inside the `herdr` terminal multiplexer, on the machine owner's own
administrator-capable login account (no separate restricted service account exists).

---

## security-boundary

### path-allowlist-validation
- **What:** `internal/security.Validator.Validate(path)` (B1) is the single check every
  agent-reachable path goes through before it can be handed to the terminal runtime. The
  order of its seven steps is itself part of the contract, not an implementation detail.
- **Where:** `internal/security/paths.go`
- **Notable:** The ordering is explicitly load-bearing: (1) reject non-absolute paths outright
  — "guessing a working directory to complete one is how escapes happen"; (2) reject
  parent-traversal components outright rather than cleaning them, because "cleaning a
  traversal before symbolic links are resolved computes the wrong parent whenever a component
  is a link"; (3) check the deny list on the **unresolved** path first, so a denied path is
  refused "because it is denied, not because the machine happened to refuse to look at it";
  (4) resolve every symbolic link (a not-yet-existing path resolves through its deepest
  existing ancestor); (5) check the deny list **again** on the resolved path, catching a link
  planted inside an allowed root that points at the system config tree; (6) check containment
  last, **component by component, never as a text prefix** (`…/projects-evil` beside
  `…/projects` shares a text prefix but is not contained); (7) deny by default — "there is no
  allow branch of last resort." A path that fails to resolve for any reason other than
  not-yet-existing is denied (fail-closed corollary).
- **Keywords:** allowed roots, resolved path, deny-then-resolve-then-deny-again, containment by component, fail-closed

### hard-deny-list
- **What:** A fixed set of directories is refused outright regardless of configuration (D6):
  the filesystem root, the system configuration tree, the administrator's home, the system
  state tree, every mounted foreign filesystem, the user's shell-key directory, the user's
  cloud-credential directory, and the user's per-user configuration directory.
- **Where:** `internal/security/paths.go` (`deniedAbsoluteSubtrees`: `/etc`, `/root`,
  `/var/lib`, `/mnt`; `deniedHomeSubtrees`: `.ssh`, `.aws`, `.config`)
- **Notable:** The per-user config directory is denied specifically to keep an **agent** out
  of the service's own configuration and the chat bot credential — "the service reads its own
  configuration directly and never through this check, so this entry costs it nothing" (D11).
  Open Gaps flags this list is known-incomplete (D17): the signing-key directory,
  cluster-credential directory, container-registry credential directory, network-credentials
  file, source-forge credential store, and shell history are all **absent**, "latent" only
  because no configured allowed root currently contains them — widening the list is deliberately
  left as an operator decision, not an implementer default.
- **Keywords:** D6, D17, deny list, not configurable

### boundary-construction-refusal
- **What:** B2 — the security boundary itself refuses to be constructed at startup if given
  zero allowed roots, or if any allowed root is itself on/inside the deny list.
- **Where:** `internal/security/paths.go` (constructor), `docs/specs/configuration.md` R4
- **Notable:** "Adding the system configuration tree to the allowed roots is a startup error,
  not a permission grant" — this is what makes the deny list un-widenable even by an operator
  who tries.
- **Keywords:** D6, D11, startup error, un-widenable deny list

### slug-sanitization
- **What:** B3 turns untrusted chat text (a task title) into a branch/directory-safe slug.
  Charset is an **allowlist** (lowercase letters, digits, hyphen), not a blocklist. Two
  variants: branch slug (60-char cap) and session slug (40-char cap, used as chat topic
  title).
- **Where:** `internal/security/slug.go` (`SanitizeBranchSlug`, `SanitizeSessionSlug`,
  `MaxBranchSlugLen = 60`, `MaxSessionSlugLen = 40`)
- **Notable:** Operates on **bytes, not decoded characters** — "a foreign character, a
  malformed byte and an over-long encoding of a slash all collapse to the same separator, so
  there is no decoding step for an attacker to disagree with about what a character 'really'
  is." Empty-slug outcome (nothing survives reduction, or input was `..`) is an **error**, never
  a fallback to raw input — the caller must substitute its own generated identifier. Runs of
  separators collapse and are trimmed from both ends so a truncation cut never leaves a
  trailing hyphen.
- **Keywords:** allowlist charset (not blocklist), byte-level reduction, empty-slug error, no silent fallback

### branch-slug-last-gate-refusal
- **What:** B4 — a worktree-creation request's branch name is accepted only if it is **exactly**
  what the sanitizer itself would have produced; each path segment (e.g. `agent/<slug>`) is
  checked independently.
- **Where:** `internal/security/slug.go` (`ValidateBranchSlug`)
- **Notable:** "This check sits at the last gate before the value leaves the process, not only
  at the caller: a caller can be bypassed by a future code path; this cannot" (D12). Without
  this gate a task titled `../../../.ssh` would be created **verbatim** as a directory by the
  terminal runtime "and nothing else in the system would notice."
- **Keywords:** D12, last-gate defense, verbatim-creation risk

### secret-redaction
- **What:** B5 — every text about to leave the service toward a chat user, and every external
  command's failure output, is passed through a single redactor that replaces matches with a
  labeled placeholder naming the secret class (named-secret assignment, password assignment,
  chat bot token, model-provider keys, source-forge tokens, cloud access key id, bearer token).
- **Where:** `internal/security/redact.go` (`NewRedactor(secretEnvNames []string)`,
  `DefaultRedactor()`, `Redact(string)`)
- **Notable:** Idempotent by construction — "the placeholders it writes match none of its own
  patterns, so already-redacted text passes through unchanged." Explicitly **best-effort**:
  documented as unable to catch a secret with no recognizable shape (bare password, opaque
  blob), and deliberately tuned against over-redaction because "redacting so aggressively that
  ordinary prose is mangled makes operators turn it off" — so the bare word "password" in prose
  is *not* redacted, only an assignment is. Cannot be disabled; there is exactly **one**
  redactor implementation in the system ("a second implementation would be a second place for a
  secret to leak from"). An earlier version missed a credential embedded inside a URL — "its
  most common leaked form" — and a test caught it; that shape is now explicitly matched.
- **Keywords:** redaction classes, idempotent, best-effort, cannot be disabled, single redactor

### safe-create-point-of-use
- **What:** B6 — closes the time-of-check/time-of-use gap for a not-yet-existing
  agent-reachable directory (today: a new session's working directory) via a no-follow create
  plus re-validation, rather than trusting the startup allowlist check alone.
- **Where:** `internal/security/create.go` (`SafeCreateDir(target)`)
- **Notable:** Three ordered steps: (1) create the final directory component with a no-follow
  operation (`os.Mkdir`) — if a symlink was planted at that component since validation, the
  create **fails** rather than writing through it, and a pre-existing real directory fails the
  same way ("promises a freshly created, exclusively owned leaf, never an adopted one"); (2)
  re-resolve the **whole** created path and re-run B1's checks on it (a parent component can be
  a link even when the leaf is real); (3) on escape, tear the created directory down — "an
  escaped path is never left on disk." Motivated by an explicit observation that "the terminal
  runtime's own checkout tool *follows a symbolic link at its target*" (D27, verified on this
  host) — so the runtime cannot be trusted to refuse the swap either; protection must happen at
  the moment of creation, by the service itself. A companion **second** check (D29) then
  re-validates the path the runtime *reports back* after cutting the worktree, living in
  `session-lifecycle.md` B2 (not in this file). Known non-atomicity is recorded as D30: a
  race between create and re-check yields "repeated refusals — a denial of service — never an
  escape"; full atomicity would need `openat2`/`RESOLVE_BENEATH`, unavailable in the Go stdlib,
  and is deferred deliberately rather than silently accepted.
- **Keywords:** D16, D27, D29, D30, TOCTOU, no-follow create, checkout-tool-follows-symlink, not atomic by design

### redaction-and-path-checker-singularity
- **What:** Both the path checker and the redactor are structurally singular: `internal/security`
  imports nothing else from the project (not even the config package — roots are passed in),
  and `internal/projects` is the sole place config and security are wired together. There is
  deliberately no second path-checking implementation anywhere (R4).
- **Where:** `internal/security/paths.go`, `internal/projects/registry.go`
- **Notable:** "Two checkers eventually disagree, and an attacker only needs the more
  permissive one." Verified independently (per `project-registry.md`) that `internal/projects`
  contains **no** `HasPrefix`/`EvalSymlinks`/`filepath.Rel` containment logic of its own — every
  verdict comes from `internal/security`.
- **Keywords:** single source of truth, no duplicate containment logic

### adversarial-and-mutation-testing
- **What:** The security package is covered by unit tests plus two independent adversarial test
  suites written by a different author, and is mutation-tested against specific known-wrong
  implementations.
- **Where:** `internal/security/{paths,slug,redact,create}_test.go`,
  `internal/security/adversarial_test.go`, `internal/security/create_adversarial_test.go`
- **Notable:** "Swapping component containment for `strings.HasPrefix` fails the prefix trap;
  moving symlink resolution after the containment check fails all five escape cases." The
  create tests prove a leaf swap and a parent swap are each refused, "each preceded by a
  naive-create proof that the swap *would* have escaped, so the fix is not vacuous."
- **Keywords:** adversarial suite, mutation testing, non-vacuous proof

### chat-boundary-no-path-input
- **What:** No operation anywhere in the product accepts a directory from a chat user; a chat
  user only ever names a project **key** that an operator put in the registry (R9, spec §15.3).
- **Where:** conceptual/cross-cutting (`docs/specs/security-boundary.md` Actors & Access,
  `docs/specs/project-registry.md` R9)
- **Notable:** "The allowlist is never reachable from chat" — stated as the reason a whole
  category of attack (chat-supplied path traversal) doesn't need to be defended against at the
  boundary layer at all, only at the slug layer.
- **Keywords:** project key, no path from chat

### not-a-sandbox-honest-limit
- **What:** The security-boundary doc opens by stating its own limit before anything else: this
  mechanism is not a sandbox. It governs only which paths the *service* hands to the terminal
  runtime; once a coding agent is running, nothing in this area stops it from typing any command
  the login account can run, including reading files this boundary refuses to *point* it at.
- **Where:** `docs/specs/security-boundary.md` (Purpose, Open Gaps)
- **Notable:** "The stated security posture cannot honestly be signed off without" the deferred
  fix — running the service and agents under a separate, non-administrator account — which is
  tracked as a product backlog item, not implemented. R1 (D1) states the path allowlist, the
  agents' own permission settings, and secret redaction are "the only containment that exists"
  given the service runs as the machine owner's own admin-capable account.
- **Keywords:** D1, not a sandbox, honest limit, deferred restricted account

## configuration

### strict-config-decoding
- **What:** B1/B2 — settings are decoded with `yaml.Decoder.KnownFields(true)`: any
  unrecognized or misspelled setting name is a **startup error** naming the offending key, never
  silently ignored or defaulted.
- **Where:** `internal/config/loader.go`, `internal/config/validation.go`
- **Notable:** The doc gives the reasoning explicitly: "a mistyped security setting that falls
  back to a default is a security setting the operator believes is in force and is not." A
  wrong-typed value is likewise an error naming the **key**, not the line number — "making them
  count lines to find it is a defect." All validation failures are collected and reported
  together (`errors.Join`), not one per run.
- **Keywords:** strict decoding, KnownFields, fail loud not lenient, per-key errors

### defaults-layering
- **What:** B1 — built-in defaults are established first, then the settings document is layered
  over them; a leading `~` in any path is expanded to home, and every path is made absolute
  before anything downstream compares or joins paths.
- **Where:** `internal/config/config.go` (`Defaults()`), `internal/config/loader.go`
  (`expandPaths(home)`)
- **Notable:** "An empty document is therefore a valid, fully-defaulted configuration" — proven
  by a test that the shipped `configs/config.example.yaml` loads.
- **Keywords:** defaults layering, path expansion, absolute paths only

### bot-token-env-only
- **What:** The Telegram bot credential is never a settings field; it is read from the process
  environment (`TELEGRAM_BOT_TOKEN`) by exactly one reader in the whole system and is never
  logged or serialized.
- **Where:** `internal/config/loader.go` (`BotTokenFromEnv()`), sentinel
  `ErrBotTokenMissing`
- **Notable:** "There is exactly one reader of it in the whole system, and it is held in no
  settings field, so it cannot be serialized or logged by accident." Strict decoding turns any
  attempt to place the token in the settings document into an error, since no such setting
  exists. Canonical secrets file location (D1): `~/.config/agentbridge/agentbridge.env`, stated
  mode 600 — but "the secrets file's permissions are stated, not enforced" (Open Gaps: nothing
  checks this at startup).
- **Keywords:** D1, D15, single reader, env-only credential, unenforced file mode

### empty-allowlist-fail-closed
- **What:** B4 — an allowed-project-roots list with zero entries is a hard startup error (D14),
  never interpreted as "no restriction" or "allow everything."
- **Where:** `internal/config/validation.go`
- **Notable:** "The specification never said which way an unset allowlist falls, and with the
  service running as the machine owner's own administrator-capable account, failing open would
  mean an unset value grants an agent the entire filesystem — the worst possible reading of a
  blank line." This is a case where the spec was silent and the implementation had to pick a
  fail-closed default deliberately.
- **Keywords:** D14, fail-closed default, unset-allowlist ambiguity resolved

### deferred-auth-obligation
- **What:** R5 (D15, D2) — `telegram.group_id` and `telegram.allowed_user_ids` (together "the
  entire authentication check") are **not** required at config load time, but the chat gateway
  is obligated to refuse to start if either is unset.
- **Where:** `docs/specs/configuration.md` (spec only — see Open Gaps below;
  `internal/config` holds the settings fields)
- **Notable:** "The obligation moves, it does not vanish... A gateway that starts without them
  is an open door." Explicitly flagged in Open Gaps as **not yet implemented**: "no chat code
  exists in this slice (D2)... Until it is built, the authentication check exists only as this
  rule." This is a documented-but-unenforced security obligation, distinct from an
  implementation bug.
- **Keywords:** D2, D15, deferred enforcement, documented-not-built

### store-initial-prompt-toggle
- **What:** `sessions.store_initial_prompt` (default `true`) controls whether a task's first
  chat instruction is written to the durable record at all; set false and the field is left
  empty.
- **Where:** `internal/config/config.go`
- **Notable:** R7 — "This exists because a person can paste a credential into a chat message; it
  is the only way to keep that text out of the durable record, and the record's shape is fixed
  in this slice, so the switch has to exist now or never." A rare case of a config knob whose
  entire justification is a security/privacy escape hatch shipped ahead of when it's needed.
- **Keywords:** durable record, credential-in-chat risk, ship-now-or-never rationale

### terminal-runtime-timeouts
- **What:** Distinct timeout settings for ordinary Herdr commands (`herdr.command_timeout_seconds`,
  default 15s) versus starting a coding agent (`herdr.start_agent_timeout_seconds`, default 45s)
  — deliberately different because starting an agent takes materially longer.
- **Where:** `internal/config/config.go`
- **Notable:** The doc's Pointers section flags that three deadlines
  (`gitops.command_timeout_seconds`, `telegram.request_timeout_seconds`,
  `herdr.start_agent_timeout_seconds`) were **omitted by the original specification** even
  though the spec's own timeout table requires them — these were added by the implementer to
  fill a gap the spec left.
- **Keywords:** spec-gap-filled, differentiated timeouts

### cap-tightening-only
- **What:** R8 — a project's per-project caps (`limits.max_concurrent_sessions`,
  `limits.max_runtime_minutes`, `limits.max_tail_lines`) may only tighten the global setting,
  never widen it (enforced in project-registry).
- **Where:** `internal/config/config.go` (global defaults), enforced in
  `internal/projects/validation.go`
- **Notable:** One-directional override is a recurring pattern across both configuration.md and
  project-registry.md — surfaced here as the global side of the same rule.
- **Keywords:** monotonic caps, tighten-never-widen

### canonical-locations
- **What:** D1 fixes the canonical settings/data locations to the machine owner's own
  per-user config/data directories, explicitly **superseding** the original specification's
  system-wide directories and separate service account.
- **Where:** `internal/config/loader.go`; canonical paths
  `~/.config/agentbridge/config.yaml`, `~/.config/agentbridge/agentbridge.env` (mode 600),
  data under `~/.local/share/agentbridge`; `--config` and `--projects` CLI overrides
- **Notable:** This is a locked decision that explicitly overrides the upstream spec rather
  than merely filling a gap in it — an example of implementation reality diverging from
  written spec *by design decision*, not by discovery.
- **Keywords:** D1, canonical locations, spec superseded

## project-registry

### registry-sole-reachability-point
- **What:** The project registry is stated as "the only place in the system where a directory
  becomes reachable by a coding agent" — an operator-written allowlist of project entries, each
  naming a repository, permitted agents, permissions, caps, and worktree location.
- **Where:** `internal/projects/registry.go`, `internal/projects/project.go`
- **Notable:** "It contains no path-containment logic of its own, by design" — it constructs the
  security boundary from configured allowed roots and routes every path it holds through that
  boundary, never re-implementing containment itself.
- **Keywords:** sole reachability point, no own containment logic

### registry-load-and-validate-all-startup
- **What:** B1 — the registry document is strictly decoded, refused if it declares zero
  projects, has its security boundary constructed once, and then every entry is fully
  validated against the boundary, the machine, and the settings — with every failing entry
  reported together, never silently skipped.
- **Where:** `internal/projects/registry.go`
- **Notable:** "A registry that quietly drops a misconfigured project boots 'successfully' into
  a state the operator did not write and cannot see" — this is the explicit reasoning for
  all-or-nothing startup validation rather than best-effort loading. The loaded registry is
  also immutable post-load (deep-copied via `clone()` on access) so no caller can mutate it
  after the one-time validation.
- **Keywords:** strict decode, all-entries-validated, immutable registry, no silent skip

### worktree-root-not-inside-repo-check
- **What:** B2 — the worktree root must not lie inside any repository, checked by walking it
  and its ancestors — including when the directory doesn't exist yet.
- **Where:** `internal/projects/validation.go` (`repositoryContaining()`)
- **Notable:** "Creating it later would drop it inside that repository... an agent writing there
  writes inside a repository nobody meant to expose." Concretely non-hypothetical on the
  reference host: "the operator's repository tree is itself a repository, so putting worktrees
  under it would violate the rule" — the canonical worktree location deliberately sits outside
  it (R4, spec §10).
- **Keywords:** worktree-not-nested-in-repo, ancestor walk, non-hypothetical example

### sandbox-escape-hatch-refusal
- **What:** B3 (D18) — three settings are refused **at load**, not merely discouraged: the
  coding agent's full-filesystem-access sandbox value (`danger-full-access`), the other agent's
  permission-bypass mode (`bypassPermissions`), and a declared `capabilities.sudo: true`.
- **Where:** `internal/projects/validation.go` (`refusedPermissions` holds
  `danger-full-access` and `bypassPermissions`)
- **Notable:** "The original specification only said do not use them" — the implementation
  goes further and makes them load-time errors, reasoning that "a setting that switches the
  sandbox off defeats the boundary from inside the very file that is supposed to define it."
  Framed as intentionally difficult to reverse: "a later milestone that genuinely needs full
  access must add the value deliberately, and sees why it is refused while doing so." This is a
  case of the implementation deliberately hardening beyond the literal spec text.
- **Keywords:** D18, escape hatches, refused-not-discouraged, deliberate-friction-to-reverse

### env-var-blocklist
- **What:** B4 (D19) — a fixed list of 17 process-hijacking environment variable names
  (dynamic-loader hooks, search path, shell startup-file hooks, runtime option hooks,
  source-control command hooks) is refused when an operator tries to pin them via
  `environment.values`/`environment.allowed`.
- **Where:** `internal/projects/validation.go` (`deniedEnvNames`, 17 names)
- **Notable:** Explicitly self-flagged as the **wrong shape**: "a blocklist can only refuse what
  somebody thought of." Named gaps: source-control index/object-directory overrides,
  interpreter option hooks for other languages, package-manager option variables, the shell's
  directory-search variable, and the coding agents' **own** home/config variables — any of which
  can redirect what an agent runs or where it reads credentials from. Also: "the values of
  allowed variables are entirely unvalidated." The doc states the honest fix as an operator
  decision, not an implementer default: "either allowlist the names, or pass no
  operator-supplied environment to agents at all" — filed as backlog, deliberately not decided
  in this slice.
- **Keywords:** D19, blocklist-known-weak, unvalidated values, filed as backlog

### environment-contradiction-check
- **What:** A pinned environment value (`environment.values`) whose name is not present in
  `environment.allowed` is treated as a contradiction and rejected at load, rather than guessed
  at.
- **Where:** `internal/projects/validation.go` (`validateEnvironment()`)
- **Notable:** "Guessing which half the operator meant is how a variable gets set that they
  believed they had excluded — so it is an error." Never "set it anyway," never "drop it
  silently."
- **Keywords:** contradiction-is-error, no guessing

### agent-binary-executability-check
- **What:** B2 — every allowed agent's binary is checked for existence and executability at
  **startup**, not at task creation time, following symbolic links (because what has to be
  executable is what actually runs).
- **Where:** `internal/projects/validation.go`
- **Notable:** "checked at startup, not at task creation when no operator is watching" — an
  explicit design choice to surface a broken binary path while a human is present rather than
  silently at the moment a chat user triggers a task.
- **Keywords:** startup-time check, symlink-followed, fail-early

### closed-agent-set
- **What:** `agents.allowed` draws from a closed, known set of agent names; an unrecognized name
  is a load error, not a silent no-op or a silent allow-everything.
- **Where:** `internal/projects/validation.go` (`validateAgents()`)
- **Notable:** "A typo that silently allowed nothing — or worse, everything — is a defect found
  in production." A duplicate agent in the allowed list is likewise a startup error.
- **Keywords:** closed set, typo-as-error

### deploy-capability-declared-not-enforced
- **What:** `capabilities.deploy` is validated and stored but has no consuming behavior yet — a
  project claiming `deploy: true` gets no actual effect.
- **Where:** `internal/projects/project.go` (`Capabilities`)
- **Notable:** Explicit Open Gaps warning: "an operator could reasonably believe otherwise" —
  flagged as a trap where the config surface implies more than the code delivers.
- **Keywords:** declared-not-enforced, config-implies-more-than-code

### environment-not-yet-wired-to-runtime
- **What:** The registry's per-project `environment.allowed`/`environment.values` fields are
  validated at load but not yet applied to a running agent process — that wiring belongs to the
  (not-yet-built) session lifecycle.
- **Where:** `internal/projects/project.go` (`Environment` struct, declarations only)
- **Notable:** "Until then, these fields are declarations, not enforcement" — another explicit
  gap between what the registry validates and what actually happens at runtime.
- **Keywords:** declaration-vs-enforcement gap

### registry-toctou-gap
- **What:** Open Gaps notes registry startup validation does not survive the machine changing
  underneath it — e.g., a repository created inside the worktree root, or a symlink swapped in
  at a not-yet-existing path, after startup.
- **Where:** conceptual (no dedicated file; cross-referenced against `security-boundary.md` D16)
- **Notable:** Explicitly named as "the same time-of-check/time-of-use class as D16 in
  security-boundary.md and has the same fix: call the boundary at each point of use, not only at
  startup" — i.e., the registry has the same class of gap that B6 (safe-create-point-of-use)
  closes for the security boundary, but has not yet had its own B6-equivalent applied.
- **Keywords:** D16 (cross-reference), unclosed TOCTOU in registry layer

## discovery (herdr reality vs. written spec)

### herdr-viable-verdict
- **What:** Milestone 0 spike verdict: herdr 0.7.3 (socket protocol 16, schema_version 1)
  exposes the control surface the specification (`docs/specs/Telegram Remote Agent Bridge.md`)
  assumes; every command spec §21 requires exists and works; all 5 spec §8 agent-status values
  (`idle`, `working`, `blocked`, `done`, `unknown`) are present in the schema (4 of 5 observed
  live). Milestone 1 cleared to proceed.
- **Where:** `docs/DISCOVERY.md`; raw evidence in `docs/discovery/probe-cli.md`,
  `docs/discovery/probe-agent-state.md`, `docs/discovery/herdr-api-schema.json`,
  `docs/discovery/socket-events.log` (not read in this pass — see caveats)
- **Notable:** "YES, with three deltas — one of which changes the design." The compatibility
  check is recommended to pin **socket protocol 16**, not the marketing version string, because
  that's the number that's actually contractual.
- **Keywords:** D5, milestone 0, protocol 16, verdict

### delta-1-codex-trust-prompt-invisible
- **What:** DISCOVERY CONTRADICTS SPEC — Codex's first-run "Do you trust the contents of this
  directory?" prompt sets no "Action Required" OSC title, so Herdr's `osc_title_blocked` rule
  reports `agent_status: idle` (not `blocked`) while the agent is actually stuck waiting on a
  human answer.
- **Where:** `docs/DISCOVERY.md` (Delta 1, marked HIGH — changes the design)
- **Notable:** Spec §4.5 gives every write-enabled task a **fresh git worktree**, and a fresh
  worktree is by definition a directory Codex has never seen — so "every AgentBridge session
  hits this prompt on startup, and every one of them looks idle — indistinguishable from
  'finished, waiting.'" Consequence: "A session created from Telegram would hang silently and
  forever: idle fires no notification under spec §18." Also noted as actively dangerous: "the
  spike's first Codex process died because a prompt was sent while it was waiting on the trust
  question." This invalidates spec §8's session-start flow as written ("starting → launch agent
  → wait for Herdr to detect agent → send first prompt → working" is "incomplete") and is
  carried forward as a first-class Milestone 2 design problem, not a footnote.
- **Keywords:** DISCOVERY CONTRADICTS SPEC, osc_title_blocked, trust prompt, spec §4.5, spec §8, spec §18

### delta-2-agent-send-no-submit
- **What:** DISCOVERY CONTRADICTS SPEC — `herdr agent send <target> "<text>"` types text into
  the agent's input box and stops; it does **not** submit it. The prompt only reaches the agent
  after a separate `pane send-keys <pane> Enter` call, and sending Enter too quickly after the
  text loses the submission entirely (observed).
- **Where:** `docs/DISCOVERY.md` (Delta 2, MEDIUM)
- **Notable:** Spec §9.6 treats `agent send` as "the complete 'send a prompt' primitive; it is
  not." Consequence for the Herdr client: `SendAgent` must be implemented as
  send-text-then-confirm-then-Enter, not a single call.
- **Keywords:** DISCOVERY CONTRADICTS SPEC, spec §9.6, two-step submit, race on Enter

### delta-3-session-env-var-ignored
- **What:** DISCOVERY CONTRADICTS SPEC — `HERDR_SESSION=remote-agents` set in the environment is
  **ignored** by herdr 0.7.3's CLI; `herdr status server` still reports the default session's
  socket. The `--session <name>` flag (or connecting to that session's socket directly) is what
  actually selects a named, isolated session.
- **Where:** `docs/DISCOVERY.md` (Delta 3, MEDIUM, explicitly called out as a safety finding)
- **Notable:** Spec §9.1 states every Herdr command from AgentBridge must pass
  `HERDR_SESSION=remote-agents` in the environment. Following that literally "would set
  `HERDR_SESSION=remote-agents`, believe it was isolated, and in fact create every workspace,
  worktree and agent **inside the user's own interactive Herdr session**." The doc states "this
  happened during this very spike, and it is why the finding is recorded rather than quietly
  fixed" — the Herdr client must pass `--session` explicitly on every invocation, and the
  systemd unit must start `herdr --session remote-agents server`.
- **Keywords:** DISCOVERY CONTRADICTS SPEC, spec §9.1, HERDR_SESSION ignored, --session flag required, session isolation failure

### delta-4-duplicate-status-events
- **What:** `pane.agent_status_changed` was observed firing **twice in the same millisecond**
  for a single transition.
- **Where:** `docs/DISCOVERY.md` (Delta 4, LOW)
- **Notable:** "The event consumer must de-duplicate or Telegram will double-notify." Spec §15.6
  already requires a notification debounce; this finding makes that debounce "mandatory rather
  than cosmetic."
- **Keywords:** duplicate events, spec §15.6, debounce now mandatory

### delta-5-integration-install-not-required
- **What:** `herdr integration install codex|claude` is not actually a prerequisite on 0.7.3 —
  agent detection works out of the box via auto-refreshed remote manifests
  (`~/.local/state/herdr/agent-detection/remote/*.toml`), fetched with no install step observed
  during the spike.
- **Where:** `docs/DISCOVERY.md` (Delta 5, LOW; also §1 "Agent detection installation")
- **Notable:** Spec §19.4 lists this install command as a manual acceptance step. "The command
  still exists; it is simply not required." Recommendation: the health check should assert
  **manifest status**, not installation.
- **Keywords:** spec §19.4, manifest auto-refresh, install-step-unnecessary

### delta-6-undocumented-commands
- **What:** `herdr pane send-keys` works but appears in no `--help` output (only documented in
  the socket schema as `pane.send_keys`); `herdr server` (headless mode) is real but missing
  from the top-level usage summary.
- **Where:** `docs/DISCOVERY.md` (Delta 6, cosmetic)
- **Notable:** "Both are safe to depend on; neither should be discovered again by trial" — a
  note-to-future-self rather than a design-affecting finding.
- **Keywords:** undocumented commands, cosmetic delta

### agent-detection-latency-margin
- **What:** The `pane.agent_status_changed{blocked}` socket event for a genuine Codex approval
  prompt arrived 11ms **before** a 250ms polling loop could even see the prompt rendered on
  screen.
- **Where:** `docs/DISCOVERY.md` §3
- **Notable:** Compared explicitly against the spec's AC-06/AC-07 bar of 10 seconds: "Herdr is
  ~2 orders of magnitude inside it." Notification latency will be dominated by Telegram, not by
  Herdr.
- **Keywords:** AC-06, AC-07, socket-event-latency, polling-loop-comparison

### cancel-two-ctrlc-behavior
- **What:** Cancel behaves exactly as spec §8 designs it for real Codex: first `Ctrl+C` stops
  the work and leaves the pane alive; second `Ctrl+C` exits the agent and closes the pane. No
  `kill -9` is needed.
- **Where:** `docs/DISCOVERY.md` §3
- **Notable:** This is one of the few items explicitly confirmed to match spec (not a delta) —
  included here because it's a concrete, verified design mechanic (graceful vs. hard cancel via
  repeated signal) rather than assumed behavior.
- **Keywords:** spec §8, graceful cancel, double Ctrl+C, no kill -9

### isolation-and-recovery-verified
- **What:** Two agents in two workspaces reported independent statuses simultaneously
  (`claude: idle` / `codex: blocked`) confirming spec's AC-09 isolation requirement; `api
  snapshot` returns workspaces (with `worktree.checkout_path`), panes, and agents (with
  `agent_status` and the native `agent_session` id) sufficient for the SQLite reconciliation
  spec §17 needs to re-match a stored row to a live pane after a restart.
- **Where:** `docs/DISCOVERY.md` §3
- **Notable:** Both cited as spec requirements that were verified to actually hold on real
  herdr, rather than assumed.
- **Keywords:** AC-09, spec §17, session recovery, snapshot API

### unproven-carried-to-milestone-2
- **What:** Explicitly flagged as **not yet proven**, to avoid the design being finalized on an
  unverified assumption: (1) Claude's `blocked` transition was never actually triggered in the
  spike — only Codex's approval prompt drove `blocked`; Claude's `blocked` behavior is assumed,
  not proven. (2) Whether Codex's trust-prompt problem (Delta 1) also appears for Claude in a
  fresh worktree is untested. (3) Herdr server restart / socket reconnect behavior (spec §9.8
  steps 4–5) was not probed at all.
- **Where:** `docs/DISCOVERY.md` §5 ("Still unproven — carry into Milestone 2")
- **Notable:** The doc is explicit that these gaps must not be silently assumed-good: "Claude
  detection and idle are confirmed; its blocked behavior is assumed, not proven, and the
  notification design must not be finalized on that assumption."
- **Keywords:** unproven, Claude blocked untested, reconnect untested, spec §9.8

## Locked decision IDs encountered

- **D1** — Service/terminal runtime/agents all run as the machine owner's own
  administrator-capable login account (no separate restricted account); also fixes canonical
  config/data locations under the user's per-user directories, superseding the spec's
  system-wide paths.
- **D2** — No chat gateway code exists in this slice; the group/allowed-users startup-refusal
  obligation (paired with D15) is recorded but not yet built.
- **D5** — Version baseline for the M0 discovery spike (herdr 0.7.3, protocol 16, codex-cli
  0.144.3, claude 2.1.208, Go 1.26.1).
- **D6** — The hard deny list is compiled-in and cannot be extended, reduced, or overridden by
  any configuration.
- **D7** — golangci-lint is deferred (absence at discovery time is consistent with this
  decision).
- **D11** — Project locations and worktree locations are both agent-reachable and go through one
  check with one deny list; the service's own config/data dirs are not agent-reachable paths and
  bypass the allowlist entirely.
- **D12** — The worktree-creation gate refuses any branch name that isn't exactly what the
  sanitizer would have produced, as a last-gate check independent of the caller.
- **D14** — An empty path allowlist is a hard startup error (fail closed, never fail open).
- **D15** — The Telegram group id and allowed-user list are not required at config load, but the
  chat gateway must refuse to start without them (paired with D2, not yet enforced).
- **D16** — The time-of-check/time-of-use hole for a path validated once at startup is closed at
  the point of use (via B6), with a residual availability-only boundary (see D30).
- **D17** — The hard deny list is exactly as enumerated and known to omit several sensitive
  locations (signing keys, cluster/container-registry/network credentials, source-forge
  credential store, shell history); widening it is left as a deliberate operator decision.
- **D18** — The three sandbox/permission escape-hatch settings (`danger-full-access`,
  `bypassPermissions`, `capabilities.sudo: true`) are refused at load, not merely discouraged.
- **D19** — The environment-variable refusal list is a blocklist and is known to be the wrong
  shape (misses several process-hijacking vectors); filed as backlog, not fixed in this slice.
- **D27** — Verified on-host that the terminal runtime's own checkout tool follows a symbolic
  link planted at its target, so the runtime cannot be relied on to refuse a symlink swap.
- **D29** — The caller performs a second B1-family check on the path the runtime reports back
  after cutting a worktree, tearing it down if it escaped (companion to B6).
- **D30** — The point-of-use safe-create (B6) is not atomic; it guarantees confinement
  (no escape) but not availability (a race yields repeated refusals, i.e. denial of service,
  never an escape).

## Unresolved / not fully verified

- I did not open the raw evidence files DISCOVERY.md references (`docs/discovery/probe-cli.md`,
  `docs/discovery/probe-agent-state.md`, `docs/discovery/herdr-api-schema.json`,
  `docs/discovery/socket-events.log`) — task scope was the four named files only, so these are
  cited from DISCOVERY.md's own summary, not independently checked.
- I did not open `docs/history/telegram-remote-agent-bridge/CONTEXT.md` or `.bee/decisions.jsonl`
  (the two files security-boundary.md/configuration.md/project-registry.md cite as the systems
  of record for decision IDs D1/D2/D5/D6/D7/D11/D12/D14/D15/D16/D17/D18/D19/D27/D29/D30) — decision
  glosses above are reconstructed entirely from how each spec file describes and uses the ID
  inline, not from the decision records themselves. If the decision files contain additional
  nuance not repeated in the specs, it is not captured here.
- `session-lifecycle.md` is referenced repeatedly (as the home of B6's companion second check,
  D29, and Delta 1's Milestone-2 fix) but was not one of the four files in scope, so that
  mechanism is only described here from the referencing side, not verified against its own spec.
- The original upstream specification document itself,
  `docs/specs/Telegram Remote Agent Bridge.md`, was not read — all "spec §N" references in this
  report are quoted/paraphrased from how DISCOVERY.md and the three spec files describe it, not
  independently confirmed against the spec's own text.

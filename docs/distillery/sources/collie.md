---
name: collie
type: git-repo
url: https://github.com/AltanS/collie
local: upstreams/collie
last_analyzed_commit: 8c898a0
last_analyzed_date: 2026-07-28
domains_covered: [harness, hooks, workflow, orchestration, context-memory, planning, quality-gates, docs-style, tooling, config-packaging, repo-layout, safety, ux, testing-evals]
---

# collie — Feature Index

> Extracted from HEAD `8c898a0` on 2026-07-28. Clone: `upstreams/collie`. Full inventory report:
> `docs/distillery/reports/distill-collie-inventory-2026-07-28.md`. Collie is a Herdr plugin: a
> phone-first PWA (Vite+React+TS+shadcn) plus a Bun/TS bridge that talks to Herdr's Unix socket,
> letting an operator monitor and reply to their agent herd from a phone over Tailscale.

## harness

### harness-adapter-capability-fence
- **What:** Per-agent dialog-lifting adapters live under `web/src/lib/harness/`, and every module
  there except `guard.ts` must stay I/O-pure (no network) because a socket call types keystrokes
  into a live terminal. A test fails the build if any other module in the tree imports the API
  client, matching the import specifier as text anywhere in the file (defeats a line-wrapped
  import slipping past an AST-based linter).
- **Where:** `web/src/lib/harness/fence.test.ts`, `web/src/lib/harness/guard.ts`, `HARNESS_CONTRIBUTING.md`
- **Notable:** the repo's own ESLint `no-restricted-imports` rule encodes the same fence but is
  admitted to be "aspirational" — no ESLint runner is wired into CI, so it never runs. The text-scan
  test is called out as the *real* gate. Lesson: an unenforced lint rule is not a control; if a
  linter isn't actually wired into CI, a plain assertion that does run is worth more.
- **Keywords:** capability fence, I/O-pure module boundary, aspirational lint rule
- **Seen:** 8c898a0

### harness-capability-tier-ladder
- **What:** New per-agent support ships incrementally: Tier 0 (free raw terminal mirror, no
  adapter needed), Tier 1 (read-only lift — status/draft extraction plus detection of a *brand-new*
  block kind with no keystroke recipe yet, so a mis-detection only costs cosmetics), Tier 2
  (interactive — taps send real keystrokes; requires a dated fixture corpus, a written
  choreography-notes file, a green conformance run, and maintainer live-verification before the
  send path is enabled).
- **Where:** `HARNESS_CONTRIBUTING.md`
- **Notable:** explicit trap called out for contributors — if a new detector recognises an
  *existing* interactive block kind (prompt-select/wizard/multi-select), its keystroke recipe is
  already wired, so it is automatically Tier 2 the instant the detector matches, even though the
  change looks like a safe Tier-1 read-only addition.
- **Keywords:** incremental capability, mergeable from fixtures alone, existing-kind trap
- **Seen:** 8c898a0

### harness-fail-closed-detector-contract
- **What:** Every dialog detector must return `null` on anything it does not confidently
  recognise; a partial/best-guess lift is a bug, not a nicety, because a false positive types a
  keystroke into a real shell. Canonical example: a numbered menu past option 9 bails to the raw
  mirror rather than half-model it, since option 10 would need the unsendable multi-char key `"10"`.
- **Where:** `HARNESS_CONTRIBUTING.md`, `web/src/lib/harness/claude/*.ts`
- **Notable:** the unsendable-key example ties the abstract "fail closed" rule to a concrete,
  checkable edge case rather than leaving it as a vibe.
- **Keywords:** fail-closed, confident-or-null, unsendable key
- **Seen:** 8c898a0

### harness-fixtures-first-detector-development
- **What:** Dialog detectors are developed and gated entirely against byte-faithful pane captures
  (`scripts/capture-fixture.sh <paneId> <name>`) taken from a real sandbox agent, never guessed
  from screenshots; detectors are pure functions over `StyledLine[]` tested through the real
  `parseAnsi → splitLines` pipeline, tail-anchored so a dialog scrolled off-screen (real output
  below it) must stop matching.
- **Where:** `scripts/capture-fixture.sh`, `web/src/fixtures/panes/`, `HARNESS_CONTRIBUTING.md`
- **Notable:** tail-anchoring is named "the core false-positive guard." The capture script itself
  carries a standing warning that fixtures land in a *public* repo — review every capture with
  `less -R` for secrets before `git add`.
- **Keywords:** byte-faithful fixture, tail-anchored detection, public-repo secret review
- **Seen:** 8c898a0

### harness-conformance-suite
- **What:** `describeAdapterConformance(adapter, {ownFixtures, foreignFixtures, neutralFixtures})`
  is a shared test helper every adapter's test file calls, asserting three invariants in one call:
  conservative detection (must stay raw-only against fixtures belonging to *other* agents and
  neutral buffers), tail-anchoring, and key-grammar validity (every emittable keystroke passes
  `isValidHerdrKey`).
- **Where:** `web/src/lib/harness/conformance.ts`, `web/src/lib/harness/conformance.test.ts`
- **Notable:** bundling "doesn't false-positive on foreign agents' output" into the same reusable
  assertion as "every key it could send is real" means a new adapter can't accidentally ship
  having skipped either check.
- **Keywords:** adapter conformance, conservative detection, key-grammar validation
- **Seen:** 8c898a0

### harness-race-guard-verify-before-submit
- **What:** An RPC ack from the terminal socket means only "herdr took the bytes," never "the TUI
  acted on them" — a focused dialog can swallow typed text and consume Enter with both calls
  reporting success. Every guarded send path re-reads the pane and re-derives model equality after
  typing, and only fires the submit keystroke once the typed text is verifiably visible on screen.
  Because Herdr's `revision` field is an empirically-confirmed stub (always 0), the guard can't
  trust it as a change signal and diffs actual re-derived content instead.
- **Where:** `web/src/lib/harness/guard.ts` (`entryGuard`, `pollUntil`), `bridge/server.ts`
  (`sendReplySteps`), `web/src/lib/reply-action.ts` (`sendGuardedReply`), `HERDR_API.md`
- **Notable:** exists because of a real production incident (#34) — a reply sent while a
  permission dialog was focused answered the dialog instead, destroying the message while the
  bridge reported success. The fix generalized into type→verify→submit, applied uniformly across
  every dialog-family send path.
- **Keywords:** ack ≠ delivered, revision field is a stub, verify-then-submit, three-valued poll
- **Seen:** 8c898a0

## hooks

### hooks-installable-hookspath-switch
- **What:** Git hooks live *in* the repo as versioned files (`scripts/git-hooks/pre-commit`,
  `pre-push`) rather than requiring each clone to manually populate `.git/hooks/`; a single
  idempotent script activates them for a checkout via `git config core.hooksPath
  scripts/git-hooks` + `chmod +x`, instead of symlinking individual hook files.
- **Where:** `scripts/install-hooks.sh`, `scripts/git-hooks/pre-commit`, `scripts/git-hooks/pre-push`
- **Notable:** `core.hooksPath` redirects git's hook lookup to a whole versioned directory in one
  config write, instead of the older pattern of symlinking each hook file into `.git/hooks/`
  individually — simpler to keep in sync as hooks are added/removed, and the hooks themselves stay
  reviewable in PRs since they're ordinary tracked files.
- **Keywords:** core.hooksPath, versioned git hooks, one-command activation
- **Seen:** 8c898a0

## orchestration

### orchestration-one-bridge-serves-every-named-session
- **What:** With `COLLIE_MULTI_SESSION` on (default), a single bridge process discovers and serves
  every named Herdr session found under the config root through the *same* URL — not one bridge
  instance per session — periodically rescanning for new/vanished sessions;
  `COLLIE_MULTI_SESSION=0` restricts it to just the primary session.
- **Where:** `bridge/sessions.ts` (the registry's refresh method), `README.md` (Security §"One
  bridge fronts every session")
- **Notable:** explicitly named as a security tradeoff in the README, not just a convenience
  feature — serving every discovered session (including a throwaway/sandbox session) through the
  same URL means anyone who can reach the URL can read/drive *all* of them, not just the primary;
  the offered mitigation is simply turning multi-session off, not scoping per-session access.
- **Keywords:** session auto-discovery, one-URL-many-sessions tradeoff
- **Seen:** 8c898a0

## context-memory

### context-memory-transcript-not-terminal-scrollback
- **What:** Because an agent's TUI runs on the terminal's alternate screen (`ESC[?1049h`), the
  emulator keeps no scrollback ring at all and the live pane-read API can never return more than
  the current viewport — so "scroll back through what an agent said earlier" is implemented by
  reading the agent's own on-disk session transcript file instead of the terminal, a genuinely
  separate data source with different fidelity (turns and their text, not a replay of the screen).
- **Where:** `bridge/transcript.ts`, `web/src/routes/history.tsx`, `ARCHITECTURE.md` §5
- **Notable:** the transcript reader also follows "continuation" — when a conversation is resumed,
  forked, or promoted from a background session, the CLI copies the thread into a *new* file, so
  naive single-file reading would silently lose history. The reader walks to the freshest
  same-root sibling with an explicit guard that following a continuation can never show *less*
  history than before, so an edge case in the chain-following logic fails toward "show more,"
  never "show less."
- **Keywords:** alt-screen has no scrollback ring, transcript-as-history-source, never-show-less guard
- **Seen:** 8c898a0

## planning

### planning-parking-lot-for-deliberately-unbuilt-ideas
- **What:** The architecture doc keeps an explicit "Future ideas" section (and a shorter
  "Considered, not built" list under the security section) for ideas surfaced while exploring the
  upstream API, framed as a parking lot so they "don't get re-discovered from scratch or acted on
  by accident" — each entry names why it's not (yet) adopted and what would need to be true to
  adopt it deliberately.
- **Where:** `ARCHITECTURE.md` §8 ("Future ideas"), §6 ("Considered, not built")
- **Notable:** distinct from an ADR (which records a decision already made and closes the
  question) — this is for ideas nobody has decided on yet, explicitly not scheduled, existing
  purely so a future contributor proposing the same thing finds context instead of starting from
  zero, or worse, half-implementing it by accident (the raw-ANSI-streaming idea is explicitly
  flagged: "adopt this deliberately... or not at all. Don't half-do it.").
- **Keywords:** parking lot, not-yet-decided vs already-decided, half-do warning
- **Seen:** 8c898a0

## safety

### safety-one-managed-front-door
- **What:** The bridge manages the lifecycle of exactly one tunnel technology end-to-end
  (`tailscale serve` — publish, record ownership, only tear down what it recorded); every other
  tunnel/mesh is explicitly out of scope (`COLLIE_SKIP_SERVE=1` + operator-owned ingress) rather
  than a `case` branch per VPN mesh.
- **Where:** `.adr/0001-one-managed-front-door.md`, `scripts/collie-ctl.sh` (`cmd_serve`, the
  `tailscale-managed-handler` ownership record)
- **Notable:** reached by *rejecting* a fully-built, careful 1441-line PR that added a second
  front door (NetBird), on three checked-not-assumed grounds: nobody was actually blocked (the
  one-line manual workaround already worked); the maintainer had no install/CI for the second
  tool, so its CLI contract would be maintained by reading a PR description rather than anything
  that runs; and the PR's own generated process leaked `--with-pin`/`--with-password` as plain
  command-line arguments, readable by any local user via `ps -eo args` or `/proc/<pid>/cmdline`.
  Stated criterion: "we manage only what we run and can test."
- **Keywords:** credential leak via ps -eo args, scope-creep rejection, tunnel-agnostic core
- **Seen:** 8c898a0

### safety-device-auth-fail-closed
- **What:** Per-device write authorization (`COLLIE_DEVICE_HEADER`/`COLLIE_DEVICE_ALLOWLIST`)
  fails closed on every ambiguous case: feature off → fully authorized (back-compat); header
  absent → read-only with *no* loopback exemption (explicit design comment: "every supported front
  door is a proxy co-located with the bridge... a loopback peer says nothing about whether the
  caller is the operator"); header present but not allowlisted (or the literal `"unknown"`
  sentinel) → read-only.
- **Where:** `bridge/server.ts` (`deviceAuth`, `guard`), `README.md` security section
- **Notable:** this fail-closed behavior was itself a breaking-change security fix (CHANGELOG
  0.15.0) — the header used to grant full write access when *absent*, letting any tailnet client
  skip the proxy meant to inject it. Lesson: an auth gate defaulting open on "header not present"
  is broken even when the intended deployment always sets it, because the bridge itself is still
  reachable without going through that proxy.
- **Keywords:** fail-closed default, device allowlist, breaking security fix
- **Seen:** 8c898a0

### safety-same-origin-host-allowlist-csrf
- **What:** Every request passes a same-origin/CSRF gate: loopback always trusted; a write with no
  `Origin` from a non-loopback `Host` is rejected outright; `Host` itself is checked against an
  explicit allowlist (loopback, `COLLIE_PUBLIC_HOSTS`, or the host of an `COLLIE_ALLOWED_ORIGINS`
  entry) specifically to defeat DNS rebinding, where a hostile page's browser is tricked into
  sending a `Host` that matches the attacker's own origin.
- **Where:** `bridge/server.ts` (`checkAccess`, `isHostAllowed`), `ARCHITECTURE.md` §6, `README.md`
- **Notable:** the README calls the host allowlist "effectively mandatory" once TLS is skipped
  (`COLLIE_SERVE_MODE=http`) — a hardening knob whose necessity is conditional on a *different*
  config flag, worth remembering when documenting "optional" controls.
- **Keywords:** DNS rebinding, Host header allowlist, CSRF via missing Origin
- **Seen:** 8c898a0

### safety-loopback-bind-bounds-remote-not-local
- **What:** Binding to `127.0.0.1` only is documented as bounding *remote* reach, not local — any
  other uid on the same host can still open the TCP port and drive every pane, unlike Herdr's own
  Unix-socket API whose filesystem permissions bound callers to the owning uid. The device-auth
  gate restores a write boundary for that case but explicitly not a read one ("it bounds damage,
  not disclosure") — snapshots/pane output/transcripts stay readable by any local uid regardless.
- **Where:** `ARCHITECTURE.md` §6, `README.md` security section
- **Notable:** precise, reusable vocabulary for scoping a mitigation's claim — distinguishing "this
  control gates writes only" from "this control gates disclosure" prevents a reader from assuming
  broader coverage than exists. The doc also names the concrete residual attack (a firewall rule on
  the port won't stop a same-host peer) and the real fix (own network namespace, or a uid-match
  nftables rule).
- **Keywords:** TCP port vs unix socket permission model, damage vs disclosure, uid boundary
- **Seen:** 8c898a0

### safety-path-traversal-guards-transcript-and-static
- **What:** Two independent traversal guards share a shape: static file serving rejects any
  resolved path that isn't exactly the web root or doesn't start with `webDir + separator`
  (catching the sibling-directory-prefix collision case, e.g. `dist-x` vs `dist`, that a naive
  `startsWith(webDir)` would wrongly allow); transcript reads never accept a client-supplied path —
  only a pane id, mapped server-side to a strict-v4-UUID-validated session id, then
  containment-checked via `realpath` *after* symlink resolution.
- **Where:** `bridge/server.ts` (`resolveStaticPath`), `bridge/transcript.ts`
- **Notable:** the sibling-prefix collision is a traversal-guard bug pattern easy to miss even when
  you remember to check "does it start with the allowed prefix" — the fix needs exact-match-OR-
  prefix-plus-separator, not prefix alone.
- **Keywords:** startsWith traversal bug, symlink-after-resolution check, UUID-validated id
- **Seen:** 8c898a0

### safety-audit-log-never-fails-the-action
- **What:** Every write-level action is appended to a sanitized, mode-0600 JSONL audit log for
  defense-in-depth traceability, but the write is fire-and-forget and can never throw or block the
  action it's auditing — a logging failure is swallowed and only `console.warn`'d.
- **Where:** `bridge/audit.ts`
- **Notable:** for a system whose whole point is remote availability, an audit trail that could
  fail the operation it logs would turn an availability bug (disk full) into a correctness bug
  (blocked writes). Matches `ARCHITECTURE.md`'s own framing: "a trail is not a gate."
- **Keywords:** fire-and-forget audit, availability over auditability
- **Seen:** 8c898a0

### safety-upload-validation-server-generated-filenames
- **What:** Uploads are validated *before* buffering (Content-Length pre-check ahead of parsing
  `formData`, 10MB cap, image-only MIME allowlist), and the stored filename is always
  server-generated (sanitized pane id + timestamp + random UUID slice) — never derived from the
  client-supplied filename.
- **Where:** `bridge/server.ts` (upload handling)
- **Notable:** checking `Content-Length` before buffering the body avoids the classic "read the
  whole multipart body into memory before learning it's too big" DoS-shaped bug.
- **Keywords:** server-generated filenames, pre-buffer size check
- **Seen:** 8c898a0

## workflow

### workflow-poll-not-stream-event-poked
- **What:** Live state uses interval polling of a single "give me everything" RPC
  (`session.snapshot`) as the sole source of truth, plus a long-lived event-subscription stream
  that does nothing except trigger an immediate debounced re-poll on any relevant event. Cadence
  itself adapts — relaxes while the stream is healthy, drops back to fast polling when it's
  down/reconnecting. Since the poll is authoritative and the stream only "pokes" it, a dropped
  event stream costs one extra poll interval of latency and never a correctness gap.
- **Where:** `bridge/event-poker.ts`, `bridge/state-engine.ts`, `ARCHITECTURE.md` §5
- **Notable:** built this way because the underlying transport has no raw output-stream event to
  stream in the first place. Reusable shape for "the source API only offers polling primitives but
  you still want low-latency reactivity": adaptive-cadence-poll-plus-poke gets most of push's
  benefit without needing push support from the source.
- **Keywords:** adaptive poll cadence, debounced poke, snapshot-is-truth
- **Seen:** 8c898a0

### workflow-two-independent-recovery-loops-by-design
- **What:** Both connection layers (bridge↔upstream, browser↔bridge) get their own trivial,
  poll-based recovery loop designed in from the start rather than retrofitted — a failed poll just
  marks the link degraded and keeps retrying next tick; there is no WebSocket to the browser at all
  and thus no reconnect/resubscribe choreography to build.
- **Where:** `ARCHITECTURE.md` §5
- **Notable:** framed directly as a consequence of choosing pull over push at both hops
  ("polling makes reconnect trivial") — useful lens for evaluating push vs. pull on any live-state
  UI: push needs explicit backpressure/reconnect machinery, bounded polling gets resilience as a
  byproduct of the transport choice, at the cost of latency.
- **Keywords:** pull over push, backpressure-free polling, reconnect-by-default
- **Seen:** 8c898a0

### workflow-old-server-fallback-by-specific-error-match
- **What:** When the primary single-RPC snapshot method isn't supported by an older server, the
  client detects this *only* by matching the specific "unknown method" error shape naming that
  exact method, and falls back permanently to a 3-call trio for that connection. Any other error on
  the same call is treated as transient and does not trip the fallback.
- **Where:** `bridge/state-engine.ts` (`fetchWire`), `HERDR_API.md`
- **Notable:** deliberate distinction between "the server lacks this feature" (detected precisely,
  cached as a permanent downgrade) vs. "this call failed" (retried) — conflating the two would let
  a transient network blip permanently downgrade a modern server to the slower path.
- **Keywords:** capability-detection via specific error match, permanent-vs-transient fallback
- **Seen:** 8c898a0

### workflow-multi-session-registry-key-not-path
- **What:** Supporting multiple concurrent upstream sessions treats a client-supplied session name
  strictly as a `Map` lookup key; the code carries an explicit inline comment that it must never be
  used to build a filesystem path. Session discovery instead scans a config root and matches
  directory names server-side, never concatenating untrusted client input into fs calls.
- **Where:** `bridge/sessions.ts`
- **Notable:** small, concrete pattern for "a user-supplied string selects among several
  server-side resources": look it up via a map/enum keyed by trusted-side-discovered names, never
  build a path string from the client's own value even when it happens to match a real directory
  today.
- **Keywords:** map-key not path-concat, session discovery
- **Seen:** 8c898a0

### workflow-dual-signal-update-monitor
- **What:** "An update is needed" is derived from two independent signals: comparing the running
  version against GitHub release tags, and comparing an order-independent stamp of every backend
  source file + lockfile captured at process boot against the same stamp recomputed live — the
  second one catches "files on disk were rebuilt/updated but this running process was never
  restarted," which a version-tag check alone can't see.
- **Where:** `bridge/update.ts` (`UpdateMonitor`, `bridgeStampSync`, `stampOf`)
- **Notable:** the repo's own CLAUDE.md names "forgetting to restart after a backend change" as the
  #1 "my change didn't take" trap — this gives that exact failure mode a dedicated automated
  detector instead of relying on the operator remembering.
- **Keywords:** stale-process detection, order-independent file stamp, two-signal freshness check
- **Seen:** 8c898a0

### workflow-frontend-self-update-hysteresis
- **What:** The PWA's own update flow requires the server's build-id header to show a new build
  across two *consecutive* polls (not one) before prompting, plus a once-per-build sessionStorage
  guard against re-prompting for the same build, plus a reload-hold check that defers any
  auto-reload while the user has unsent composer text, an open upload, or an open sheet.
- **Where:** `web/src/lib/self-update.ts`, `web/src/lib/reload-guard.ts`, `web/src/lib/server-build.ts`
- **Notable:** three independent guard layers stacked on what could have been a single "new build
  header seen → reload" check, each covering a different failure mode (a flaky build-id read,
  repeat-prompting, yanking the page out from under an in-progress edit) — kept as separate
  composable checks rather than one combined heuristic.
- **Keywords:** two-consecutive-poll debounce, reload-hold on in-flight edits
- **Seen:** 8c898a0

## quality-gates

### quality-gates-version-triple-lockstep-enforcement
- **What:** A version must agree across the plugin manifest, root `package.json`, `web/package.json`,
  and the newest CHANGELOG heading, checked by one script invoked from three independent places:
  the build script itself, a git pre-commit hook (blocks committing functional-code changes when
  the version literal didn't move), and CI. The pre-commit hook also enforces strict monotonic
  increase via `sort -V` so a copy-paste typo can't silently downgrade the version.
- **Where:** `scripts/check-version.sh`, `scripts/git-hooks/pre-commit`, `.github/workflows/ci.yml`,
  `CLAUDE.md` (Versioning section)
- **Notable:** three redundant enforcement points for the same invariant rather than trusting any
  single one — CLAUDE.md is explicit that the human is still "the first line," these are backstops.
  `sort -V` for "did this go backwards" is a cheap trick that avoids hand-parsing semver.
- **Keywords:** sort -V monotonic check, three-layer redundant enforcement
- **Seen:** 8c898a0

### quality-gates-atomic-build-swap
- **What:** The frontend build always writes to a staging directory first and only swaps it into
  the live served directory (`rm -rf dist && mv dist-staging dist`) after success — a failed build
  never leaves the live directory empty or half-written.
- **Where:** `scripts/collie-ctl.sh` (`cmd_build`)
- **Notable:** this atomicity is what makes the advertised "frontend rebuild is live immediately,
  no restart" operational property (the bridge serves `web/dist` from disk at request time) safe to
  rely on.
- **Keywords:** build-to-staging-then-swap, serve-from-disk hot reload
- **Seen:** 8c898a0

### quality-gates-escape-hatches-are-named-env-vars
- **What:** Every enforcement gate (version check, typecheck, tests) has exactly one escape hatch,
  always an explicitly-named env var passed inline on the command being bypassed
  (`SKIP_VERSION_CHECK=1 git commit …`, `SKIP_TESTS=1 git push`), never a silent flag or a buried
  config toggle.
- **Where:** `scripts/git-hooks/pre-commit`, `scripts/git-hooks/pre-push`, `CLAUDE.md`
- **Notable:** naming the hatch after exactly what it skips, required inline at the point of use,
  keeps the bypass visible in shell history/CI logs at the exact moment it happens — a good default
  shape for "let people override a gate without hiding that they did."
- **Keywords:** named env-var escape hatch, visible-at-point-of-use bypass
- **Seen:** 8c898a0

## docs-style

### docs-style-adr-when-to-write-heuristic
- **What:** Guidance for when a decision needs a written ADR: "the signal is that you find yourself
  explaining *why not* rather than *how*" — a decision that closes off an option a future
  contributor will reasonably propose again. Explicit non-example: "Use Vitest for the web suite"
  is not ADR-worthy since that's simply what the repo does and CLAUDE.md already covers it.
- **Where:** `.adr/README.md`
- **Notable:** a crisp, transferable heuristic for deciding whether something needs a dedicated
  decision doc versus staying implicit in code/CLAUDE.md.
- **Keywords:** why-not vs how, ADR non-example
- **Seen:** 8c898a0

### docs-style-doc-role-separation-table
- **What:** Four docs are given non-overlapping, explicit roles so none restates another:
  CLAUDE.md = the short normative rule (links to ADRs for the argument), ARCHITECTURE.md = how it's
  built today, README.md = how an operator runs it, `.adr/` = why a road was *not* taken. A
  superseded ADR is never edited into agreement with the present — it's marked "Superseded by NNNN"
  and a new one is written.
- **Where:** `.adr/README.md`
- **Notable:** "the wrong turn is the useful part" — decision-history docs should preserve rejected
  paths rather than tidy them away, since the rejection reasoning is what stops the same proposal
  from being re-litigated from scratch.
- **Keywords:** doc-role separation, never edit a superseded ADR
- **Seen:** 8c898a0

## config-packaging

### config-packaging-thin-plugin-launcher-vs-supervised-daemon
- **What:** A long-lived network-facing service is deliberately not run as a plugin pane (which
  dies whenever the pane closes, the terminal detaches, or the host restarts — exactly when a
  remote user isn't watching); the plugin manifest is reduced to a thin launcher whose actions
  shell out to `systemctl --user` against an independently-supervised `systemd --user` service.
- **Where:** `herdr-plugin.toml`, `ARCHITECTURE.md` §3, `systemd/collie.service`
- **Notable:** reusable rule-of-thumb for "should this be a plugin/pane or an independent daemon":
  if it needs to survive the host restarting or its container closing, it can't live inside that
  host's own pane/plugin lifecycle.
- **Keywords:** plugin-as-launcher, pane-lifecycle-tied-to-host, systemd --user supervision
- **Seen:** 8c898a0

### config-packaging-install-vs-link-build-timing
- **What:** The manifest's `[[build]]` step only runs on a hosted/GitHub plugin install, never on
  a local dev `plugin link` — link-mode installs build lazily on first `start` instead (an
  idempotent `ensure_build` check), so both paths share one build definition without needing two
  different triggers wired at the host level.
- **Where:** `herdr-plugin.toml`, `scripts/collie-ctl.sh` (`ensure_build`)
- **Notable:** avoids writing build logic twice for "installed from a registry" vs. "linked from a
  local checkout" — one script, two callers, each appropriate to how that install path is actually
  exercised.
- **Keywords:** install vs link install paths, lazy-build-on-first-start
- **Seen:** 8c898a0

### config-packaging-fail-closed-startup-warnings
- **What:** On every startup, a pure function evaluates the loaded config and emits explicit nags
  for weakened-security states (no trusted user, empty device allowlist, no public-host allowlist,
  and a specific conflicting combination: skip-serve set together with a trusted-user check that
  then has nothing to validate against).
- **Where:** `bridge/config.ts` / `bridge/server.ts` (`startupWarnings`)
- **Notable:** modeling this as a pure function (config in, warning strings out) rather than inline
  logging scattered through startup makes the whole warning surface independently unit-testable.
- **Keywords:** pure startup-warning function, conflicting-config detection
- **Seen:** 8c898a0

## repo-layout

### repo-layout-tag-per-release-drives-github-release
- **What:** Cutting a release means the version files + newest CHANGELOG heading all agree (steps
  enforced elsewhere), then pushing an annotated git tag matching that version (`vX.Y.Z`) is what
  actually triggers the GitHub Actions release workflow to auto-create the GitHub Release
  (extracting that CHANGELOG section as the release notes) — the tag push is the sole trigger, not
  a separate manual "create release" step.
- **Where:** `.github/workflows/release.yml`, `CLAUDE.md` (Versioning section)
- **Notable:** this makes the git tag the single source of truth that a release happened —
  CLAUDE.md is explicit that tagging is "not hook-enforced — it's on you," the one step in the
  whole versioning discipline left as a manual human action rather than gated by a hook,
  deliberately, since tagging is also the irreversible "this shipped" signal.
- **Keywords:** tag-triggered release workflow, one manual un-hooked step by design
- **Seen:** 8c898a0

## ux

### ux-needs-you-triage-not-chronological
- **What:** The home screen is organized by urgency, not recency: blocked agents ("NEEDS YOU")
  float to the top, working/idle/done collapse below; simultaneous blocked-transitions across
  multiple agents batch into *one* summary push notification rather than one per agent.
- **Where:** `web/src/lib/agent-groups.ts` (`AGENT_GROUPS`), `bridge/notifications.ts`
  (`NotificationCoordinator` coalesce), `ARCHITECTURE.md` §4
- **Notable:** the coalesce behavior is explicit product intent captured in the architecture doc
  itself ("not three races") — a multi-item notification system needs an explicit coalesce policy,
  not just per-item dedup.
- **Keywords:** urgency-first triage, notification coalescing
- **Seen:** 8c898a0

### ux-structured-blocks-not-raw-screenful
- **What:** Instead of showing the raw last screenful when an agent needs input (often
  mid-stack-trace, with the real question scrolled above), recognized dialog patterns are parsed
  into interactive tappable blocks with the raw pane kept below for context; an explicit
  "send exactly what I type" free-text fallback always exists since detection is heuristic.
- **Where:** `web/src/lib/blocks.ts`, `ARCHITECTURE.md` §4
- **Notable:** the doc is candid that this shipped simpler than designed — the original plan had
  the *bridge* capture the exact output chunk at the moment an agent goes blocked; what shipped is
  client-side pattern-based parsing of whatever the pane currently shows, which "works because
  agent prompts are formulaic, and degrades to raw-read when they aren't." An honestly-documented
  shipped-vs-designed gap, plus why the simpler version still degrades safely.
- **Keywords:** structured-block parsing, heuristic-with-raw-fallback, documented design gap
- **Seen:** 8c898a0

### ux-close-the-trust-loop-on-async-actions
- **What:** After sending a reply, the UI shows an explicit "Sent ✓" the moment the HTTP response
  lands, distinct from and prior to the later blocked→working status transition — without an
  immediate ack, latency causes users to double-tap send.
- **Where:** `ARCHITECTURE.md` §4
- **Notable:** reusable principle for any fire-and-forget action against a slow/async backend:
  separate and show "we received your action" from "the action's effect is now visible" rather than
  waiting for the effect (which may lag or never arrive) before giving any feedback.
- **Keywords:** immediate ack vs eventual effect, double-tap prevention
- **Seen:** 8c898a0

### ux-connection-state-machine-with-hysteresis
- **What:** Connection-lost UI uses two independently-thresholded, one-way-latching signals sharing
  one clock: a short "trouble" signal (4s) that never latches, and a longer "lost" signal (15s)
  that does latch once tripped — feeding a banner with separately-timed enter/exit so a flapping
  connection doesn't produce a flickering banner.
- **Where:** `web/src/lib/connection-health.ts`, `web/src/hooks/use-connection-lost.ts`,
  `web/src/components/connection-banner.tsx`
- **Notable:** the explicit choice to key health off actual poll success/failure and never off
  `navigator.onLine` ("it can lie after airplane-mode toggles") is called out in two separate files
  — a specific, named, transferable gotcha rather than a per-project rediscovery.
- **Keywords:** latching vs non-latching thresholds, navigator.onLine is unreliable, hysteresis banner
- **Seen:** 8c898a0

### ux-idle-lock-wall-clock-not-visibility
- **What:** The phone-theft mitigation (auto-lock after 30 min idle) tracks elapsed wall-clock time
  re-checked on foreground, rather than reacting to `visibilitychange` — so a backgrounded or
  OS-throttled tab (where timers don't fire reliably) still locks correctly once the user returns.
- **Where:** `web/src/hooks/use-idle-lock.ts`
- **Notable:** transferable gotcha for any "lock/expire after N minutes idle" feature built on
  `setTimeout`/`setInterval`: mobile background throttling makes the timer itself untrustworthy;
  recompute elapsed time from a stored timestamp on regaining foreground instead.
- **Keywords:** background timer throttling, wall-clock recheck on foreground
- **Seen:** 8c898a0

### ux-self-echo-suppression-for-in-flight-edits
- **What:** The "draft currently in the terminal's own input box" (extracted from raw pane text
  every poll) is debounced into a stable value that only surfaces once the same normalized text has
  persisted 1.5s — specifically to avoid flashing the user's own just-sent reply back at them as if
  it were pre-existing draft text mid-transition.
- **Where:** `web/src/hooks/use-terminal-draft.ts` (`isSelfEcho`, `normalizeDraft`)
- **Notable:** named pattern ("self-echo suppression") for any UI polling shared/external state that
  its own actions also mutate — a stability window filters out the observer seeing its own
  just-completed write reflected back as new incoming state.
- **Keywords:** self-echo, debounced draft stability
- **Seen:** 8c898a0

## testing-evals

### testing-msw-fake-input-box-for-guard-loop-tests
- **What:** MSW test handlers simulate a fake terminal input box that reacts to the app's own
  guarded-reply POSTs (unsubmitted text) by updating mocked pane text to show that typed draft on
  its prompt line — giving the race-guard's real poll-until-verified loop something genuine to
  converge on in tests instead of spinning until timeout against static fixture text.
- **Where:** `web/src/test/handlers.ts` (`recordReply`, `paneTextWithDraft`)
- **Notable:** reusable pattern for testing any "poll until my own side-effect is visible" client
  logic — the mock server needs to reflect back the state change the client's own action caused, or
  the poll-until-verified path never gets meaningfully exercised.
- **Keywords:** stateful mock server, race-guard test double
- **Seen:** 8c898a0

### testing-fake-cli-on-scratch-path-for-shell-lifecycle-tests
- **What:** The control script's lifecycle test suite puts fake `tailscale`/`systemctl` executables
  earlier on a scratch `PATH` and points `HOME`/config-dir at a throwaway location, so the real
  `collie-ctl.sh` runs unmodified against fakes and "runs anywhere and touches nothing real." The
  fake tailscale's serve-ownership state is a JSON file the test stages/rewrites to simulate
  different ownership situations before each scenario.
- **Where:** `scripts/collie-ctl.test.sh`
- **Notable:** lightweight, container-free way to get deterministic, host-independent lifecycle test
  coverage for a bash control script that shells out to real system tools — fake binaries on a
  prepended PATH plus a JSON state file the fakes read/write.
- **Keywords:** fake CLI on scratch PATH, JSON-file-backed fake state
- **Seen:** 8c898a0

## tooling

### tooling-manual-test-tool-reuses-real-send-path
- **What:** The manual dev CLI for firing a real push notification imports and reuses the bridge's
  actual `Push` class and config loading rather than reimplementing a simplified send call, so
  running it exercises the real VAPID→FCM→device path including dead-endpoint pruning.
- **Where:** `scripts/push-test.ts`
- **Notable:** general pattern for any manual/dev smoke-test tool: import the real class instead of
  duplicating its logic, so the tool can't silently rot out of sync with what actually ships.
- **Keywords:** reuse-real-class for manual testing, no parallel test-only implementation
- **Seen:** 8c898a0

### tooling-config-dir-cascading-resolution
- **What:** The bridge's config directory resolves through an ordered cascade: an injected env var
  takes priority, then asking the host's own CLI for the plugin's config dir, then the host's
  conventional path *if it already has a `.env` file there*, finally a hardcoded fallback — so the
  same binary works whether launched by the host plugin system, manually via systemd, or ad hoc.
- **Where:** `scripts/collie-ctl.sh` (config dir resolution)
- **Notable:** reusable shape for a service launchable multiple ways: cascade from most-specific/
  most-trusted signal down to a hardcoded default, checking for actual evidence (file existence) at
  the middle tier rather than just an env var's presence.
- **Keywords:** cascading config resolution, multi-launch-mode config discovery
- **Seen:** 8c898a0

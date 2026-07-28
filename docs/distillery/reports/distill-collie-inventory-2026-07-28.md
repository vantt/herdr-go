# collie — full inventory report (2026-07-28)

Source: `upstreams/collie` @ `8c898a0` (full scan, never analyzed before). Collie is a Herdr
plugin: a mobile-first PWA (Vite+React+TS+Tailwind+shadcn) plus a Bun/TS bridge that talks to
Herdr's Unix socket, letting an operator monitor and reply to their agent herd from a phone over
Tailscale. Curated entries derived from this report live in `sources/collie.md`.

## Root docs (read directly)

- `ARCHITECTURE.md` — problem statement, deployment model (systemd user service not a plugin
  pane), the core interaction loop (blocked → push → structured tap-reply → "Sent ✓" → transition),
  architecture notes (herdr-client is the only socket-aware module, poll-not-stream event-poked
  design, two independent recovery loops, ANSI rendered as React text nodes), §6 security model
  (loopback bind semantics, same-origin/CSRF gate, idle timeout, audit log, destructive-confirm),
  §7 Tailscale/PWA notes, §8 parked ideas (a real PTY-stream mode, deliberately not built —
  would need a real terminal emulator and breach the React-text-nodes XSS boundary).
- `HERDR_API.md` — empirically verified Herdr socket contract (v0.7.2/protocol 16): one-shot RPC
  framing, `session.snapshot` single-round-trip bootstrap + old-server fallback via a specific
  "unknown variant" error match, `pane.send_keys` key grammar (NOT tmux syntax — `ctrl+c` not
  `C-c`, no PageUp/Home/End/Delete, single-digit literal keys, multi-modifier chords in any
  order), rename/close/move method semantics and their quirks (pane.rename is the only one that
  clears via `null` and emits no event; tab.move keeps numbers stable, workspace.move renumbers),
  an ack means "herdr took the bytes" never "the TUI acted on them", event stream catalog and
  scoping rules.
- `HARNESS_CONTRIBUTING.md` — how to add a per-agent harness adapter: capability fence (only
  `guard.ts` may touch the network under `harness/`), fixtures-first workflow
  (`scripts/capture-fixture.sh`), the three-tier capability ladder (raw mirror / read-only lift /
  interactive — with the "existing block kind is automatically Tier 2" trap), the fail-closed
  detector contract (`null` on anything not confidently recognised), the two CI/safety gates
  (`describeAdapterConformance`, `fence.test.ts`).
- `CLAUDE.md` (repo working agreement, injected in full by the harness's own hook when this
  session first touched the clone) — mandatory triple-version-lockstep enforcement
  (`herdr-plugin.toml`/`package.json`/`web/package.json`/CHANGELOG heading, checked by
  `scripts/check-version.sh` from build script + pre-commit + CI), build/run operational facts
  (frontend rebuild is live with no restart because the bridge serves `web/dist` from disk;
  backend changes need a `systemctl --user restart`), frontend data layer (React Router data mode,
  no TanStack Query — deliberately), Herdr socket gotchas, security posture summary.
- `CHANGELOG.md` — real production incidents behind several safety-net mechanisms: #34 (a reply
  sent while a dialog was focused answered the dialog instead → guarded two-step send), #32
  (`/api/config` skipped the same-origin gate), #30 (401/403 rendered as endless "reconnecting"),
  #28 (device-header-absent used to grant full write access — now fails closed), #25/#27 (Windows
  named-pipe dial support).
- `herdr-plugin.toml` — manifest fields (`id="herdr.collie"`, `min_herdr_version`, `platforms`),
  `[[build]]` only fires on hosted install not `plugin link`, `[[actions]]` all shell to
  `collie-ctl.sh` and never host the server directly.
- `package.json` / `web/package.json` — script surfaces, dependency choices (React 19.2.7,
  react-router 7.18.0 not TanStack Query, Tailwind v4, vite-plugin-pwa, vitest+msw), both at
  version 0.17.0 in lockstep with the manifest.
- `.adr/README.md` + `.adr/0001-one-managed-front-door.md` — ADR format/discipline and the one
  real decision record: rejecting a second managed tunnel (NetBird) after a careful 1441-line PR,
  on three checked grounds including a concrete credential-leak-via-`ps -eo args` finding in that
  PR's own generated code.

## bridge/ — Bun/TypeScript backend (36 files, all read)

Grouped by sub-purpose:

**Socket client**: `dial.ts` (POSIX unix-socket vs Windows named-pipe dialer, one file owns the
platform difference), `wire.ts` (pure line-decoders, no I/O), `herdr-client.ts` (the ONLY module
that knows Herdr method names/shapes — `sessionSnapshot`, `subscribeEvents`, `readPane`,
`sendPaneText`/`sendPaneKeys`, rename/close, `ping`), `event-poker.ts` (owns the
`events.subscribe` lifecycle: ack→healthy, events→debounced poke, down→backoff reconnect;
builds a deliberately narrow global+per-agent-pane subscription set, excluding several event
types that would break subscribe on older servers).

**State engine / polling**: `state-engine.ts` (interval poll, `session.snapshot`-first with
permanent-on-specific-error-only fallback to the 3-call trio, in-flight guard + queued
follow-up poke, adaptive cadence via `setCadence`; also scrapes a Claude pane's `/rename`d
session name via regex against the bottommost prompt line), `sessions.ts` (multi-session
registry; explicit inline security comment that a client session name is a Map key, never a
path fragment; primary session spawned eagerly, others discovered/disposed on refresh).

**HTTP server / routes** (`server.ts`, 1137 lines, the integration point importing from nearly
every other module): route table (`/api/snapshot`, tab/workspace create, pane actions
`reply/keys/upload/close/rename/history`, config/subscribe/notification-prefs/update-check,
static PWA fallback with SPA index.html routing). Security mechanisms: `checkAccess`
(same-origin/CSRF + Host allowlist, explicit anti-DNS-rebinding comment, optional
Tailscale-identity check), `deviceAuth` (per-device write authorization, fails closed on
every ambiguous case including "header absent" with NO loopback exemption), `guard` (combines
both for a route's required level), strict CSP + fixed security headers on every response,
`resolveStaticPath` (traversal guard catching both `..` and sibling-dir-prefix collisions),
upload validation (size pre-check before buffering, MIME allowlist, server-generated
filenames), `startupWarnings` (pure fn nagging about weak config combos), every write action
calls into the audit log, `sendReplySteps` (two-step guarded reply: type → wait → submit, with
distinguishable partial-failure state).

**Audit logging**: `audit.ts` — fire-and-forget, never throws, sanitizes (collapses newlines,
truncates long strings), mode-0600 JSONL appender.

**Config**: `config.ts` — all env-driven, strict numeric parsing (`envInt` rejects trailing
garbage, clamps to bounds), default socket path per platform, full enumerated list of `COLLIE_*`
env vars.

**HTTP cache helpers**: `http-cache.ts` — ETag via `Bun.hash`, conditional gzip only above a
byte threshold and only if the client advertises support.

**Notifications/push**: `notifications.ts` (`NotificationCoordinator` — debounce+cancel,
coalesce into one herd summary, retract on resolve), `push.ts` (optional Web Push, zero hard
dependency, separate collapse topics for herd-summary vs update pushes so one can't clobber the
other, atomic subscription persistence via a save-chain to avoid interleaved writes),
`notify-prefs.ts` (persisted prefs with pure default-filling coercion), `snooze.ts` (global DND
deadline, self-expiring).

**Uploads**: `uploads.ts` — pure prune-candidate function + best-effort TTL sweep that tolerates
every failure mode (missing dir, vanished file, failed unlink) without aborting.

**Update-availability monitor**: `update.ts` — two independent staleness signals
(GitHub-release-tag comparison vs a source-file-stamp comparison that catches "rebuilt but not
restarted"), strict semver-tag parsing that rejects prereleases, edge-triggered de-duped
notification, concurrent-caller de-dup via an in-flight promise.

**Transcript/history**: `transcript.ts` — reads Claude Code's own on-disk session JSONL (because
the pane runs on the terminal alternate screen and has no real scrollback ring), strict
UUID-shaped id validation before any fs touch, containment check via `realpath` AFTER symlink
resolution, follows conversation-rotation continuations to the freshest sibling with a
never-show-less-history guard, pure grammar parser distinguishing real human turns from
injected plumbing lines.

**Domain types**: `types.ts` — the internal domain model decoupled from Herdr's wire shapes
(which live only in `herdr-client.ts`), including the triage sort order `STATUS_RANK`.

**Entry point**: `index.ts` — wires config, per-session `Push`/`Snooze`/`NotifyPrefsStore`/
`AuditLog` singletons, `UpdateMonitor` (captures version+stamp at boot, never re-reads), the
per-session factory (`HerdrClient`+`StateEngine`+`EventPoker`+`NotificationCoordinator`),
`SessionRegistry`, `startServer`, periodic unref'd timers, graceful SIGINT/SIGTERM shutdown.

**Tests (17 files)**: pure-logic unit tests per repo convention; anything touching
`Bun.serve`/sockets directly stays untested by design. `server.test.ts` is the most
security-dense (exhaustive `checkAccess`/`deviceAuth`/`guard`/`isHostAllowed`/
`resolveStaticPath` coverage).

## web/src/ — React/TS PWA frontend (223 files, all accounted for)

**routes/**: `home.tsx` (needs-you-first dashboard), `root.tsx` (mounts polling/push/transition
hooks, boot splash, error boundary), `space.tsx` (workspace drill-in, auto-redirects home if the
space vanishes from a healthy snapshot), `history.tsx` (whole-transcript-fetch + growing-window
render to cap DOM nodes, find-in-history, `shouldRevalidate: false` so the 1.5s poll never
re-pulls it), `detail.tsx` (pane detail, handles a just-created pane not yet in any snapshot via
nav-state fallback), `settings.tsx`.

**hooks/**: polling/connection (`use-polling.ts` — HOT/COLD interval by urgency, deliberately
never gates on `navigator.onLine`, self-heals a wedged revalidation; `use-connection-lost.ts` —
latching 15s "lost" vs non-latching 4s "trouble" sharing one clock; `use-online.ts` — cosmetic
only), idle-lock/terminal (`use-idle-lock.ts` — wall-clock recheck on foreground, not
visibilitychange, so background-timer-throttling can't defeat it; `use-key-queue.ts`;
`use-terminal-draft.ts` — debounced self-echo suppression; `use-display-prefs.ts`;
`use-keyboard.ts` — `visualViewport`-based on-screen-keyboard detection; `use-swipe.ts`;
`use-long-press.ts` — pointer-based with a `contextmenu` fallback for Android/iOS robustness;
`use-auto-scroll.ts`), data/actions (`use-spaces.ts`, `use-pending-confirm.ts`, `use-push.ts`,
`use-notify-prefs.ts` — optimistic + revert-on-failure, `use-transitions.ts`).

**lib/**: core data layer (`api.ts` — per-verb timeouts, ETag+body pane cache; `loaders.ts` —
navigation-vs-revalidation detection via last-seen-URL comparison, serves stale cache instantly
during a known escalated outage rather than blocking on a doomed fetch; `connection-health.ts` —
the one shared connection clock; `types.ts`; `session.ts`; `nav.ts`; `status.ts`/`busy.ts` — tiny
pub/sub stores), self-update pipeline (`server-build.ts`/`build.ts`/`self-update.ts` —
2-consecutive-poll hysteresis + once-per-build guard; `reload-guard.ts` — holds against an
auto-reload while composer text/uploads/open sheets exist; `pwa.ts`; `push.ts`/`push-decision.ts`
— pure decision fn for the service-worker push handler), ANSI/terminal rendering pipeline
(`ansi.ts` — hand-rolled SGR-only parser, precomputed `CSSProperties`, always renders as React
text nodes; `blocks.ts` — the `Block` discriminated union + the invariant that joining raw block
lines reproduces the visible mirror text exactly), **harness/** (the pluggable per-agent
detection seam: `types.ts`/`registry.ts`/`index.ts`, `guard.ts` — the only network-touching
module, `entryGuard`+`pollUntil` three-valued race guard, `sanitizeTypedText`;
`conformance.ts` — the shared CI-gate helper; `claude/` — the one shipped adapter: `markers.ts`
lexing primitives, `chrome.ts` tail-only chrome stripping, `prompt-select.ts`/`wizard.ts`
(reads segment background styling for the current-step highlight)/`multi-select.ts`/
`preview-select.ts` each with a `signature`/`coreSignature` freshness check since Herdr's
`revision` field is an empirically-confirmed stub), the four guarded send-path files
(`prompt-action.ts`/`wizard-action.ts`/`multi-select-action.ts`/`preview-action.ts`/
`reply-action.ts` — `sendGuardedReply` is the #34 fix: type unsubmitted → poll for verified
landing → only then submit), plus `agent-commands.ts`, `agent-groups.ts`, `spaces.ts`, `find.ts`,
`transcript-search.ts`, `markdown.ts` (hand-rolled, no underscore-emphasis by design so it
doesn't mangle `snake_case`; `safeHref` scheme allowlist), `format.ts`, `destructive.ts`
(word-boundary regex set driving the two-tap confirm), `key-queue.ts`, `utils.ts`.

**components/**: terminal rendering (`ansi-output.tsx`, `markdown-text.tsx`,
`transcript-view.tsx`, `option-button.tsx` — the one shared visual language across all four
dialog-block components; confirmed no `dangerouslySetInnerHTML` anywhere in the repo), pane
detail shell (`agent-chat.tsx` — the "frozen mirror" pattern pairing shown-text with the
revision it was read at; `composer.tsx`; `nav-tray.tsx`/`key-queue-strip.tsx`;
`command-palette.tsx`; `quick-actions.tsx`; `terminal-draft-preview.tsx`), dashboard/navigation
(`agent-card/list/sidebar.tsx`, `space-overview/strip/view.tsx`, `tab-strip.tsx`,
`pane-strip.tsx`, action sheets, `new-space-sheet.tsx`, `session-switcher.tsx` — portals to
`document.body` to escape header `backdrop-filter` containment, `app-header.tsx`,
`collie-home.tsx`/`dog-gallop.tsx` — pure-CSS `steps(6)` sprite, `connection-banner.tsx` — amber→
red→green with distinct enter/exit timing, probes `/api/config` only while red to distinguish
Herdr-down from bridge-unreachable, `status-area.tsx`/`busy-bar.tsx`/`status-badge.tsx`,
`find-bar.tsx`, `read-only-banner.tsx`), settings cards, update banners/build-stamp, and hand
-rolled `ui/` primitives (`sheet.tsx` — drag-to-dismiss with `preventDefault` to suppress
pull-to-refresh, backdrop-tap-to-close guarded against a long-press's synthesized click;
`button.tsx`/`badge.tsx`/`switch.tsx`/`chip.tsx`/`card.tsx`; `chat/chat-input.tsx`;
`chat/chat-message-list.tsx`).

**test/ + fixtures/**: `setup.ts` (one shared MSW server, resets connection-health anchor per
test, jsdom polyfills), `handlers.ts` (MSW handlers + fixture data — notably a fake terminal
input box that reacts to the app's own guarded-reply POSTs so the race-guard's poll-until-
verified loop has something real to converge on in tests), `pwa-register-stub.ts`,
`use-swipe.test.ts`. `fixtures/panes/*.txt` — byte-faithful real Claude Code pane captures
covering every dialog state family plus an in-flight-send/self-echo corpus; a README documents
provenance and a public-repo secret-sanitization warning.

## scripts/ + systemd/ + .github/ + .adr/ (tooling/CI/decisions, ~20 files, all read)

`check-version.sh` (extracts and cross-checks 4 version sources), `collie-ctl.sh` (control
script: build with atomic staging-dir swap, serve with ownership-recording `tailscale serve`
management, update via self-re-exec after `git pull`, config-dir cascading resolution),
`install-hooks.sh`, `git-hooks/pre-commit` (version-consistency + bump-on-functional-change +
monotonic-increase-via-`sort -V`), `git-hooks/pre-push` (typecheck + full test suite, all with
named env-var escape hatches), `collie-ctl.test.sh` (fakes `tailscale`/`systemctl` on a scratch
PATH with a JSON-file-backed fake serve-state, so it "runs anywhere and touches nothing real"),
`push-test.ts` (reuses the bridge's real `Push` class to smoke-test the actual send path),
`capture-fixture.sh`. `systemd/collie.service` (reference unit: `StartLimitIntervalSec=0` since
a phone-only operator can't run `systemctl reset-failed`). `.github/workflows/release.yml`
(tag-push-triggered CHANGELOG-section extraction → GitHub Release), `ci.yml` (mirrors the
pre-push gates on every PR/push-to-main, with the version check called out as a "MANDATORY
invariant" step). `.adr/README.md` + `.adr/0001-one-managed-front-door.md` (see Root docs above).

No files were unreadable or skipped across any of the three gather passes or the direct reads.

# Terminal scrollback for agent panes — Context

**Feature slug:** terminal-scrollback-agent-panes
**Date:** 2026-07-27
**Exploring session:** complete
**Scope:** Standard
**Domain types:** CALL, SEE

## Feature Boundary

Let the mobile terminal-detail screen scroll back past the current ~70-row
viewport into a pane's real history, for both plain-screen panes (free, via
herdr's existing API) and full-screen alt-screen agent panes (via replaying
the agent's own scroll keybinding) — ending at reading older content on
screen; it does not add a persistent gateway-side content store.

## Locked Decisions

These are fixed. Planning must implement them exactly — cited, never reinterpreted.

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | The current ~70-row ceiling occurs for panes running a full-screen TUI in the terminal's alternate screen buffer; a fresh/idle primary-screen pane with no output yet can look identical (no extra history to give either) but for a different, harmless reason. | Live-verified against herdr 0.7.4: every Claude Code agent pane reports `scroll.max_offset_from_bottom == 0` permanently, at any requested `--lines` up to 1000 — standard VT100/xterm alternate-screen semantics in the vendored `libghostty-vt` engine (`ActiveScreen::Alternate`) — no real terminal retains scrollback in alt-screen (same reason exiting vim/htop loses their screen). Fresh-eyes review found a captured live snapshot (`src/herdr/testdata/live-snapshot.json`) where a bare, agent-less shell pane also reports `max_offset_from_bottom: 0` (nothing printed yet) — so this field alone cannot distinguish "alt-screen, permanently zero" from "primary-screen, temporarily zero"; D3 resolves this by not depending on the field being definitive. |
| D2 | For primary-screen panes, request `source: recent` with an explicit `lines: 1000` (herdr's own hard cap) instead of the current default of 80. | Live-verified real scrollback for: a plain shell pane (863 lines via `recent,lines:1000` vs 71 visible), a Codex CLI pane (254 vs 67, survived a live pane split with no gap), an Antigravity CLI pane (221 vs 67) — tails matched their `visible` reads exactly in all three. Gateway's `src/herdr/socket.rs:466` currently omits `lines` entirely, silently defaulting to herdr's `unwrap_or(80)`. This is a pure additive fix to an existing call, zero new endpoints. |
| D3 | Strategy selection is result-based, not field-based: always attempt `NativeScrollback` (D2) first; only fall through to `EscapeInjection` (D4/D5) when that result is no richer than the current `visible` text. Never branch on the pane's `agent` name string. | Fresh-eyes review found the `scroll.max_offset_from_bottom` field is carried only on `snapshot.panes[]` (`PaneInfo.scroll: Option<PaneScrollInfo>`) — the `pane.read` response itself (`PaneReadResult`) has no scroll field at all, and the gateway's own `ScreenRead`/`Pane` wire types don't carry it either. Branching on that field would need cross-referencing a separately-fetched, possibly-stale snapshot per pane, and D1 shows `==0`/absent is ambiguous anyway (alt-screen vs. genuinely-empty primary-screen). Comparing the actual `recent` result against `visible` sidesteps both problems: it needs no extra field, no snapshot cross-reference, and is correct in both of D1's zero-history cases (an empty primary-screen pane gains nothing from either strategy either way, so no escalation is needed and none happens). Rejected agent-name branching for the same reasons as before: plain shell panes carry no `agent` field at all; a hardcoded name list silently mishandles any future/unknown agent; an agent's own alt-screen usage can change between its own versions while its name string stays the same, causing silent drift. |
| D4 | For alt-screen panes, request older content by replaying the agent's own scroll keybinding via raw bytes (`herdr pane send_text` with the literal VT escape sequence), then re-read `visible`; restore the live bottom view afterward the same way. | Live-verified reproducibly on a real Claude Code pane this session: sending raw PageUp (`\x1b[5~`) made Claude Code redraw its own transcript to reveal earlier content and show its own "Jump to bottom (ctrl+End)" hint; sending raw Ctrl+End (`\x1b[1;5F`) correctly restored the live bottom (re-read matched the pre-scroll tail exactly). The agent's own process already holds this history correctly — nothing needs to be captured or stored gateway-side for it. |
| D5 | The escape-sequence path must use herdr's `pane.send_text` (raw byte passthrough), never `pane.send_keys` or the mixed `pane.send_input`. The gateway's own `Herdr` trait needs a new method for this — `send_input`/`send_keys` are the only two it currently exposes (`src/herdr/mod.rs:116-170`); nothing calls `send_text` anywhere in `src/` or `web/src/` today. | `send_keys`'s named-key vocabulary has no PageUp/PageDown/Home/End token in the installed herdr version — confirmed both by a live `invalid_key: unsupported key pageup` response and by reading `upstreams/herdr/src/config/keybinds.rs::parse_key_combo` (the function `encode_api_keys` calls; its match arms cover only space/enter/esc/tab/shift+tab/backspace/arrows/punctuation/single-char/f-keys/modifier-combos). `send_input`'s `text` field passes through `encode_api_text`, which wraps it in bracketed-paste markers when the pane has negotiated bracketed paste — that would corrupt a raw escape sequence into literal pasted text. `send_text`'s handler (`handle_pane_send_text`) calls `runtime.try_send_bytes` directly on the raw parameter with no validation and no wrapping, so it is the only channel that delivers the exact bytes a real keypress would. Planning must add a `send_text` method to both `SocketHerdr` and `FakeHerdr`. |
| D6 | Open Gap, not yet resolvable: whether an alt-screen agent CLI other than Claude Code implements its own internal PageUp-triggered scroll is unverified. No fallback design is chosen for an alt-screen agent that does not respond to the escape sequence. | Codex CLI and Antigravity CLI were both live-tested this session and turned out to be primary-screen (D2 covers them) — neither ever exercised the D4/D5 path. Claude Code is the only alt-screen case verified so far. A previously-designed fallback (a gateway-side line-diff/dedupe history buffer, with its own retention cap and GC-on-pane-vanish) was worked out in detail during this session's discussion and explicitly superseded by the D4 discovery — it is parked, not chosen, and should only be revisited if D6 actually triggers against a real unresponsive alt-screen agent. |
| D7 | Rejected: reverse-engineering herdr's `herdr-client.sock` binary SemanticFrame attach protocol. Do not resurrect without new evidence. | Investigated this session: that channel renders the same underlying terminal grid the JSON API already reads (same `libghostty-vt` engine), so alt-screen panes would show zero history there too. It also has exclusive-attach (kicks the user's real desktop client), wrong per-pane granularity (renders the whole session), and an undocumented/fragile wire format. Dead end specifically for the alt-screen case, independent of effort invested. |
| D8 | Rejected: forking herdr itself to add scrollback exposure. Do not resurrect without new evidence. | The vendored source (`upstreams/herdr/`, AGPL-3.0-or-later) is real and readable, and forking was a legitimate path for D2's primary-screen case before D2 was found to already work via the stock API unmodified — so it is now unneeded there. It does not help the alt-screen case either: alt-screen zero-history is fundamental VT semantics, not something herdr is hiding, so a fork cannot manufacture data that was never captured. |
| D9 | Code shape: a hexagonal port `PaneScroller` with a `ScrollStrategy` enum (`NativeScrollback`, `EscapeInjection`) and `read_history(pane_id) -> Result<ScreenRead>` (as-implemented, superseding this row's original `Vec<String>` wording below), where the port's own implementation runs D3's compare logic internally and picks the strategy. | Matches this repo's existing hexagonal convention (`src/herdr/mod.rs`'s `Herdr` trait; `src/store/mod.rs`'s `Store` trait, literally commented "A hexagonal port"). Keeps the two retrieval mechanisms swappable/extensible per pane without touching call sites. **Supersession (implementation, `src/herdr/pane_scroller.rs`):** returns `ScreenRead{text, revision}` (the existing wire type from `src/herdr/wire.rs`), not a bare `Vec<String>` — found during cell 1's execution that the `?history=1` endpoint (D11's Agent's Discretion) needs a `revision` value to build `ScreenBody`, which a bare `Vec<String>` cannot carry. Also supersedes this decision's original `strategy_for(&ScrollMetrics)` signature — D3's fix removed the `ScrollMetrics` input entirely, so the port takes only `pane_id`. |
| D10 | The strategy-selection decision point in `read_history` must carry a WHY-comment explaining the alt-screen VT semantics (D1) and why selection is result-based, not field-based or agent-name-based (D3) — not a WHAT-comment. | User-requested explicitly, to prevent a future reader from "fixing" the branch to use `scroll.max_offset_from_bottom` or agent names instead, both of which D3's fresh-eyes review showed to be unreliable. Draft wording already agreed in chat before D3's fix; planning/implementation should preserve the intent (VT semantics + why result-based beats both field-based and name-based), updated to match D3's actual mechanism. |
| D11 | This is a genuine public-contract change (new/changed gateway API surface for requesting older pane history), not a quiet internal change. `docs/specs/terminal-detail.md`, `docs/specs/web-api.md`, and `docs/specs/herdr-port.md` all need updating once implemented — `herdr-port.md` specifically because it specs the `pane.read`/input-delivery surface this feature extends, and currently documents neither `lines` nor a `send_text` operation. | Follows this repo's existing spec style for contract-affecting areas. |

### Agent's Discretion

Exact shape of the new "give me older lines" request/response (an offset param
on the existing pane-read endpoint vs. a new endpoint) is left to planning —
D9's port interface is fixed, but its wire exposure to the web client is an
implementation detail.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Primary-screen pane | A pane where `recent` (D2) returns more than `visible` — herdr already holds real scrollback for it. |
| Alt-screen pane | A pane running a full-screen TUI in the terminal's alternate screen buffer, which by VT100/xterm semantics never accumulates scrollback (D1) — `recent` returns nothing beyond `visible`. |
| Escape injection | Sending a raw VT escape sequence via `pane.send_text` to make an alt-screen agent replay its own internal scroll behavior (D4/D5). |

## Existing Code Context

### Reusable Assets

- `src/herdr/mod.rs:116-170` — the `Herdr` trait (port) + `SocketHerdr`/`FakeHerdr` (adapters); the same hexagonal shape D9 follows. Currently exposes `send_input`/`send_keys` only — `send_text` (D5) is new.
- `src/store/mod.rs` — the `Store` trait, explicitly commented "A hexagonal port (decision `4e3ef1a1`)"; also documents the "never store terminal output" rule (airemote never-store rule) that rules out a durable/sqlite-backed history store for this feature.
- `src/herdr/socket.rs:466` — `SocketHerdr::read_pane`'s current request body, missing the `lines` key (D2's fix site).
- `src/herdr/wire.rs` — `ScreenRead`/`Pane` wire types; neither currently carries a `scroll` field (confirms D3's result-based approach needs no new wire field).

### Established Patterns

- `Herdr`/`Store` hexagonal port-per-capability convention — one trait, swappable adapters, no global architecture layers.

### Integration Points

- `web/src/views/terminal.ts` — the frontend poll loop (`fetchScreen`, 1500ms interval, cleared on screen leave) that will need a "load older" trigger point when the user scrolls to the top.
- `docs/specs/terminal-detail.md`, `docs/specs/web-api.md` — spec updates owed once the new request shape lands (D11).

## Canonical References

- `docs/history/learnings/20260718-terminal-scrollback-herdr-limit.md` — prior investigation (herdr 0.7.4 at the time) that concluded the JSON API has no scrollback and recommended filing a herd feature request. **Superseded by D1-D5**: the conclusion was correct only for alt-screen panes; primary-screen panes already had real scrollback the prior session did not request enough `lines` to see, and the alt-screen case has a working answer (D4/D5) that needs no herd change.
- `docs/history/terminal-nav-keys/plan.md` — prior plan claiming `pageup`/`pagedown`/`home`/`end` were "confirmed-valid key names" for `send_keys`. **Known wrong** for the currently-installed herdr version (D5); do not trust without re-verifying live against whatever herdr version is current at implementation time.
- `upstreams/herdr/` (main checkout only) — vendored herdr source (AGPL-3.0-or-later), read directly this session for D1, D2, D5 evidence. **Not available in this feature's worktree** (`herdr-gateway--wt--terminal-scrollback-agent-panes`): the directory is untracked by git in the main checkout (`git ls-files upstreams` returns nothing), so `git worktree add` did not carry it over. D1/D2/D5's exact quotes and line numbers above are self-contained (captured from a live read), but re-verifying anything beyond what's quoted requires either working from the main checkout or manually copying `upstreams/herdr/` into this worktree first.

## Outstanding Questions

### Deferred To Planning

- [ ] Exact request/response shape for "give me older lines" (offset param vs. new endpoint) — D9's Agent's Discretion note; implementation detail, not a product decision.
- [ ] Where exactly the escape-injection restore-to-bottom step fires relative to the frontend's own poll loop, so a restored view doesn't fight the next live poll tick.

## Deferred Ideas

- Gateway-side pane-history buffer (line-based diff/dedupe, hexagon port, GC-on-pane-vanish, backend watch-list independent of frontend screen lifecycle) — designed in detail this session as the pre-D4 fallback plan. Not built now; only revisit if D6 actually triggers (a real alt-screen agent found not to respond to escape injection). Appended to `docs/backlog.md` as PBI-057 (this feature itself is PBI-056).

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked
decisions, code context, canonical references, and deferred-to-planning questions.
Validating and reviewing use locked decisions for coverage and UAT.

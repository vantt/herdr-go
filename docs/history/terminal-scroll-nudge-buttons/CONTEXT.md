# Terminal Scroll Nudge Buttons — Context

**Feature slug:** terminal-scroll-nudge-buttons
**Date:** 2026-07-27
**Exploring session:** complete
**Scope:** Standard
**Domain types:** SEE

## Feature Boundary

Add a discoverable up/down control to the terminal-detail view (`web/src/views/terminal.ts`) that lets a mobile user pull older pane content into view and return to the live tail — and fix the pre-existing defect where the regular 1500ms poll immediately overwrites any older content the user just scrolled to, so the "load older" gesture that already exists today (`loadOlder()`, terminal.ts:236-248) actually sticks instead of flashing and reverting.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | UX mechanism: floating up/down chevron buttons, bottom-right corner, auto-hiding after a period of idle (no touch/scroll interaction). Positioned `position: absolute` against a `position: relative` wrapper (the `.view-terminal` root, `web/src/styles.css:746-753` — currently sets no `position`, needs one added) — never `position: fixed`, per this repo's existing mobile-Safari overlay precedent (`styles.css:749`, `:1162-1165`; `.term-viewport` itself is the scroll container per `styles.css:930-939` and cannot be the positioning parent for its own children). The buttons hide (not just fade) whenever `.reply-sheet` or `.keys-pad` is open — those sheets already cover the same corner and, per D6, opening either already returns the view to live, so the buttons have nothing useful to do while a sheet is open. | User-chosen after comparing against two-finger swipe, edge-swipe strip, and long-press popup — floating button won on discoverability, tap accuracy, and lower implementation risk on mobile web (no OS/browser gesture conflicts). Positioning and sheet-collision specifics added after fresh-eyes review found the original wording unimplementable (`.term-viewport` is itself the scroll container; a plain `position:fixed` would repeat a previously-fixed mobile-Safari bug class). |
| D2 | Buttons render for **every** session type (Claude, codex, agy) — not restricted to Claude. | Original premise ("codex/agy already return up to 1000 lines natively so need no button") is contradicted by code: the default poll always requests only 80 lines for every agent type (`src/web/screen.rs:41-48`, `ReadSource::Recent, 80`); only the `?history=1` path (`PaneScroller::read_history`) returns richer content, and it is agent-agnostic by design (prior feature's CONTEXT.md D3: "Never branch on the pane's `agent` name string"). Restricting the button to Claude would leave codex/agy sessions with the identical defect (D3 below) unfixed. |
| D3 | Up-button tap triggers the same history-escalation the app already runs when the user drag-scrolls `.term-viewport` to its top — i.e. call the existing `loadOlder()` (terminal.ts:236-248), not a new raw-key send via the `sendKeys`/Keys-pad path (terminal.ts:337-344, which has no read/render/restore choreography and is the wrong mechanism here). Down-button tap returns to the live tail: scroll `.term-viewport` to `scrollHeight` (the same jump used by `applySheetInset`, terminal.ts:284) and resume live polling (D4). | Corrects an earlier premise (from prior discussion, not this session) that the button should "send Page Up/Down to the pty directly" — that is not how the existing, already-shipped history mechanism works; `loadOlder()`/`fetchScreen(pane_id, true)` is the correct, already-built call. |
| D4 | While the user is viewing history (away from the live bottom — via the up-button or the existing scroll-to-top trigger), the regular 1500ms poll (`poll()`, terminal.ts:213-226) must **pause** entirely rather than continue overwriting the view. It resumes only once the user returns to the live tail (down-button tap, or scrolling back near the bottom). | Root-cause finding this session: today, `applyScreen()`'s own comment (terminal.ts:187-196) documents that a history read is "a one-shot expansion, not a persistent poll mode" — the very next poll tick (≤1500ms later) fetches the live (short) screen and overwrites the just-loaded history, collapsing the view back toward live. This is the actual reason "no scroll feeling" was reported: any historical view — old or new — survives under 1.5 seconds before snapping back. Pausing poll while off-bottom is the fix; user chose this over "poll continues but skip re-render when off-bottom," which the user rated as needlessly complex. |
| D6 | Opening the Reply or Keys sheet (`openReply()`/`openKeys()` → `applySheetInset()`, which already forces `viewport.scrollTop = viewport.scrollHeight` at `terminal.ts:284`) is the "return to live" trigger for D4's pause gate too — the programmatic scroll-to-bottom fires the same native `scroll` event the bottom-resume threshold (D4's discretion note) listens for, so polling is already resumed by the time the user taps a key or sends a reply. The two existing direct `poll()` calls after a key press or reply send (`terminal.ts:342,355`) are therefore left exactly as-is — no additional gating needed at those call sites. | Fresh-eyes review flagged that a naive pause gate would make those two `poll()` calls silently no-op while the user is viewing history (screen never updates after sending a key/reply). Reusing the sheets' existing forced-scroll-to-bottom as the resume trigger closes that gap without new code paths — it also matches product intent: opening Reply/Keys means the user wants to drive the live session, not read history. |
| D5 | Rejected alternatives (do not reopen): **two-finger swipe** — poor discoverability, conflicts with mobile browser/OS multi-touch gestures (pinch-zoom, back-navigation). **Edge-swipe strip** — poor discoverability, narrow hit-zone risks false-positives against normal scroll, more code to disambiguate from drag-scroll. **Long-press popup** — an extra step before the actual action, slower than a direct tap. | User's own comparison during exploration; floating button beat all three on discoverability and implementation simplicity. |
| D7 | The down-button, in addition to jumping to `scrollHeight` and resuming poll, calls `void poll()` immediately afterward (mirroring the existing "reflect promptly" pattern at `terminal.ts:342,355`) rather than waiting up to 1500ms for the next tick. | Fresh-eyes review noted the stale history render would otherwise sit on screen for up to one full poll interval after the user asks to return to live. |

### Agent's Discretion

- Exact idle timeout before auto-hiding the buttons, and exact "near the bottom" threshold for auto-resuming poll on drag-scroll (symmetric to the existing `HISTORY_SCROLL_THRESHOLD = 4` px top-threshold, terminal.ts:41) — tune during planning/implementation, not a product decision.
- The existing scroll-to-top auto-trigger for `loadOlder()` (terminal.ts:250-258) stays as-is and continues to co-exist with the new up-button — the button is an additional, more discoverable affordance for the same action, not a replacement.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Live tail | The pane's current, actively-polled screen content (`ReadSource::Recent, 80`, `src/web/screen.rs:47`) — as opposed to escalated history. |
| History view | Content returned by the `?history=1` escalation (`PaneScroller::read_history`), rendered once via `applyScreen()` until poll resumes and overwrites it. |

## Existing Code Context

### Reusable Assets

- `loadOlder()` (`web/src/views/terminal.ts:236-248`) — already implements the fetch-history/render round trip; the up-button calls this directly.
- `viewport.scrollTop = viewport.scrollHeight` (`terminal.ts:284`, inside `applySheetInset`) — the existing "jump to bottom" pattern the down-button reuses.
- `.term-viewport` native touch/scroll wiring (`web/src/styles.css:930-952`: `overflow: auto`, `touch-action: pan-x pan-y`, `.xterm { pointer-events: none }`) — already correct; not touched by this feature.

### Established Patterns

- `historyInFlight` / `historyArmed` guard flags (`terminal.ts:172,178`) — the existing pattern for coordinating the history fetch against the poll loop; the new poll-pause state (D4) extends this same coordination point rather than introducing a parallel mechanism.

### Integration Points

- `poll()` (`terminal.ts:213-226`) and the `setInterval` driving it (`terminal.ts:380`) — needs the pause/resume gate (D4).
- `viewport.addEventListener("scroll", …)` (`terminal.ts:250-258`) — the existing top-threshold trigger; needs a symmetric bottom-threshold check to resume polling per D4's discretion note.
- `applyScreen()` (`terminal.ts:180-211`) — the content-shrink comment here (lines 187-196) documents exactly the defect D4 fixes; touched only insofar as the pause gate prevents it from ever being called with live text while history is showing.

## Canonical References

- `docs/history/terminal-scrollback-agent-panes/CONTEXT.md` — the closed feature that built `loadOlder()`/`PaneScroller`; D3 (never branch on agent name), D4 (escape-injection mechanics), and its own Deferred-To-Planning line ("where exactly the escape-injection restore-to-bottom step fires relative to the frontend's own poll loop, so a restored view doesn't fight the next live poll tick") — this feature is the resolution of that deferred item.
- `src/web/screen.rs:35-48` — `read_screen` route: default vs `?history=1` behavior, agent-agnostic.
- `src/herdr/pane_scroller.rs` — backend scroll-strategy selection (native scrollback vs escape injection), untouched by this feature.

## Outstanding Questions

### Resolve Before Planning

(none — all gray areas resolved this session)

### Deferred To Planning

- [ ] Exact idle-hide timeout and bottom-resume-threshold values — see Agent's Discretion.
- [ ] Whether the up-button should be disabled/hidden once already at max history (no further `loadOlder()` benefit) or always tappable (harmless no-op re-fetch) — implementation choice, not product-material.
- [ ] D6's resume mechanism relies on a native `scroll` event firing after a programmatic `scrollTop` assignment — true in real browsers (async, next rendering opportunity) but **not** in jsdom (this repo's existing tests manually `dispatchEvent(new Event("scroll"))` after every scrollTop set, e.g. `web/test/terminal.test.ts:225-227,244-247`). Any test asserting "opening Reply/Keys resumes poll" must dispatch that event manually or assert the resume call directly — a green jsdom test alone is not proof of real-browser behavior here.

## Deferred Ideas

- Fallback for an alt-screen agent that doesn't respond to PageUp escape injection — already tracked as `PBI-057` from the prior feature; unaffected by this one.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked decisions, existing code context, canonical references, and the deferred-to-planning items above.

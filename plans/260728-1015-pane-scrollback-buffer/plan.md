# Pane Scrollback Buffer — Discussion Plan

**Status:** discussion only, no decisions locked, no code started.
**Slug:** pane-scrollback-buffer
**Related:** `terminal-scroll-nudge-buttons` (cells -1..-8), `terminal-scrollback-agent-panes` (PBI-056), deferred idea PBI-057, `docs/history/terminal-scroll-nudge-buttons/CONTEXT.md`.

## Why this exists

Current "load older" mechanism replays the agent's own PageUp keybinding
(`PaneScroller`, escape-injection) to reveal older content. Confirmed via live
testing (2026-07-28): Claude Code's PageUp jumps by a fixed viewport-sized
chunk, not a continuous line-by-line scroll — each hop is a disjoint page, so
even with every timing/comparison bug now fixed, the UX can never feel like
smooth continuous scrollback. That's a ceiling of the mechanism itself, not a
remaining bug.

User asked: what if the gateway itself keeps a growing line-based buffer per
pane, built by listening (polling) from the moment a pane is first opened on
mobile — would that be worth the operational cost, and would it actually feel
smooth?

## Two mechanisms compared

| | PageUp escape-injection (today) | Gateway-side buffer (proposed) |
|---|---|---|
| Feel | Jumps by fixed page, disjoint | Continuous, line-by-line — genuinely smooth |
| Reach | Can reach into the agent's own pre-existing history (before the gateway ever watched it) | Only covers what's been buffered since watching started — no retroactive reach |
| Server cost | Zero extra cost when nobody is scrolling; a few extra round-trips only during an actual "load older" tap | A background poll per watched pane, running continuously (even while nobody is looking at it on mobile), for as long as the gateway process is alive and the pane exists |
| Storage | None (stateless per request) | In-memory buffer per pane, with a retention cap — never durable (this repo's existing never-store-terminal-output rule rules out disk/sqlite) |
| Lifecycle complexity | None — every request is self-contained | Needs explicit start (first time a pane is opened on mobile), stop/GC (pane vanishes from herdr's snapshot), and a memory bound across however many panes are being watched at once |
| Failure mode if gateway restarts | N/A (nothing to lose) | Buffer is gone; scrollback for every watched pane resets to empty until re-watched |

## Cost, concretely

- One background poll loop per watched pane (not per open browser tab —
  should persist even if the operator navigates away, per the "listen from
  when opened" ask). With today's live herdr instance carrying ~13 open
  panes, that's up to 13 concurrent pollers if every pane gets opened once.
- Per-poll cost: one `pane.read` round-trip over the herdr socket. Cheap
  individually; adds up linearly with pane count and poll frequency.
- Memory: bounded only by a retention cap we choose (e.g. last N lines per
  pane) — needs to be picked deliberately, not left open-ended.
- New lifecycle surface: a supervisor that starts/stops these pollers,
  independent of the existing per-request `PaneScroller` port and independent
  of any single browser tab's lifetime.

## Benefit, concretely

- Genuinely smooth scrollback UX (the actual complaint this plan responds
  to) — this is the only mechanism that can deliver it; polishing
  PageUp-injection further cannot, it's a ceiling of the underlying
  mechanism.
- User has explicitly accepted the "no retroactive reach" trade-off (buffer
  starts empty at watch-time, that's fine).

## Open decisions to lock before any code

1. **Retention cap** — how many lines (or how much wall-clock time) of
   buffer to keep per pane before trimming the oldest.
2. **Poll interval** — how often the background poller reads a watched pane.
3. **Watch lifecycle** — exactly when a pane starts being watched (first
   mobile open — confirmed by user) and when it stops (pane vanishes from
   herdr's snapshot — GC). Does watching ever pause (e.g. operator hasn't
   opened the app in days) or run indefinitely as long as the pane exists?
4. **Cross-pane memory bound** — is there a global cap (e.g. total buffered
   lines across all watched panes), or is per-pane cap alone considered
   sufficient?
5. **Coexistence with PageUp escape-injection** — does the buffer fully
   replace `PaneScroller`'s escape-injection path, or does escape-injection
   remain as a fallback for reaching further back than the buffer covers
   (trading smoothness for reach on that one deeper request)?

## Recommendation

Worth building if smoothness matters more than retroactive reach (user has
said this). This is a new subsystem (background lifecycle, memory
management), not a bug fix — scope is closer to `terminal-scrollback-agent-panes`
than to the tiny cells this session just shipped. Lock the 5 decisions above
first (a short exploring-style pass), then shape and validate before writing
code.

## Next step

Not started. Waiting on the 5 decisions above.

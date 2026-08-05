# Transcript Live Tail — Discussion Plan

**Status:** implemented 2026-08-05 (user opted to execute directly, outside the
bee flow). Shipped: `src/transcript/` (resolve + tail + render, 11 unit tests),
`GET /api/panes/:pane/activity` (`src/web/activity.rs`, 3 handler tests), the
"Log" toggle + activity viewport in `web/src/views/terminal.ts` (5 vitest
tests), styles in `web/src/styles.css`. Verified end-to-end against `--demo`
with a fixture transcript: open = EOF (no backfill), incremental append,
sidechain/tool rendering, 200-line client ring. Open decisions below were
resolved as: (1) newest-mtime file, (2) silent re-resolve with a
"— session switched —" divider once the watched file is fully consumed,
(3) 20-line tool-output elision + 400-char line clip + 400-line poll cap,
(4) transcript lines inherit Tier 2 unredacted (same content class as the
terminal), (5) ring is client-only. Drive-by fix: `demo_config()` now builds
its JSON via serde_json — the old `format!` embedded the Windows temp dir's
backslashes as invalid JSON escapes, so `--demo` panicked on Windows.

**Round 2 (same day, user field feedback):** "open = empty" read as broken,
and subagent activity was invisible. Two scope revisions, both shipped:
(a) opening now backfills the tail of the transcript so the 200-line ring
lands full (`OPEN_BACKFILL_BYTES` = 512 KiB window, newest 200 lines win);
(b) the watch set is now multi-file — Claude Code ≥2.1.x writes each
Task/teammate transcript to `<session-id>/subagents/agent-*.jsonl` (verified
live against 2.1.222: the main file's `isSidechain` is always false), so the
tail now follows the main file plus every subagent file, merges records by
their RFC3339 `timestamp`, prefixes subagent lines with `⑂ `, and picks up
subagent files born mid-watch from byte 0. The cursor became a `;`-joined
multi-entry set (v1 single-entry cursors still parse). Per-record caps
loosened: tool output 20→40 lines, text 40→80.
**Slug:** transcript-live-tail (narrowed from transcript-as-history-source)
**Related:** distillery candidate `transcript-as-history-source` (collie, `R2 E1 F2`,
`docs/distillery/porting-log.md:104`), `docs/distillery/sources/collie.md:135-149`,
`plans/260728-1015-pane-scrollback-buffer/plan.md` (PBI-057),
`docs/backlog.md` PBI-057/PBI-058/PBI-061, `docs/specs/terminal-detail.md` "Open Gaps".

## User-locked scope (2026-08-05)

The original draft of this plan proposed full retroactive history (read the
whole transcript, continuation-chain following, pagination). The user cut that:

1. **Respect every existing feature** — terminal view, PageUp history, keys,
   reply sheet all stay exactly as they are. This is purely additive.
2. **The only need:** the displayed log must be *complete from the moment the
   view is opened* — nothing lost between polls, the way the screen-scrape
   path loses whatever scrolls by between 1.5 s ticks.
3. **No looking back.** No backfill of history from before the view was
   opened. Open → start empty (or from "now") → append incrementally, steadily.
4. **Display capped at 200 lines** — a ring: newest appends, oldest beyond
   200 falls off. No pagination, no "load older".

These four points close most of the original plan's open decisions and shrink
the build to a tail, not an archive.

## Why the transcript is still the right source (and screen-diff is not)

The requirement is *completeness from open time*. There are only two candidate
mechanisms, and one cannot meet it:

- **Diffing consecutive `pane.read` grids** cannot guarantee completeness:
  Claude Code runs on the alt screen and redraws freely (spinner, status line,
  in-place updates), so consecutive 80-line snapshots are neither prefix nor
  suffix of each other — a diff yields duplicates and garbage, and anything
  that scrolled fully past between two ticks is simply absent from both
  snapshots. This is the documented ceiling of the pixel mechanism
  (`plans/260728-1015-pane-scrollback-buffer/plan.md`), not a tuning problem.
- **Tailing the agent's own session transcript**
  (`~/.claude/projects/{encoded-cwd}/{session-id}.jsonl`, append-only, one JSON
  object per line) is gap-free by construction: a byte cursor advances over an
  append-only file, so every record between two polls is read, regardless of
  poll interval. Two independent shipping implementations read this same file
  (collie `bridge/transcript.ts`; paseo's Claude adapter history path), so the
  format risk is known and bounded.

The transcript also upgrades *what* a line is: instead of pixels, each record
is a typed turn — assistant text, thinking, `tool_use` (the bash command, the
edit), `tool_result` (its actual output) — so the 200 lines shown are 200 lines
of meaning, not 200 rows of a redrawn TUI frame.

## Proposed shape (sketch, not locked)

Deliberately minimal — one module, one route, one view addition:

1. **`src/transcript/` module:**
   - `resolve(pane) -> Option<PathBuf>`: pane's `foreground_cwd ?? cwd` from
     the snapshot (`src/herdr/wire.rs:138-142`, D5 precedence) → Claude Code's
     encoded project directory under `~/.claude/projects/` → the session file
     that is currently growing (see open decision 1). Resolved once per view
     open; `None` for panes without a transcript (view simply doesn't offer
     the tab — terminal view is unaffected).
   - `tail(path, cursor) -> (Vec<Event>, new_cursor)`: open with read-share
     semantics (the agent is appending), seek to `cursor`, read complete lines
     only — a partial trailing line is left for the next poll, cursor held
     back at the last newline. Parse each line defensively: unknown record
     shapes pass through as a raw one-liner, never an error (paseo's `unknown`
     escape-hatch lesson). Join `tool_use`/`tool_result` by id where both have
     already arrived; render them as separate lines otherwise (a tail must not
     hold lines hostage waiting for a pair).
   - **Open = now:** first call takes `cursor = len(file)` (EOF at open time).
     No backfill, per locked scope. Everything after that lands in order.
2. **Web API:** `GET /api/panes/:pane/activity?cursor=<byte-offset>` →
   `{ lines: [...], cursor }`, stateless per request like every existing
   route; `cursor` absent in the request means "start at EOF now". Polling on
   the same cadence as the screen poll (1.5 s) — no WebSocket, consistent with
   decision 675fc93a. Gateway keeps **no per-pane state**: the cursor lives in
   the browser tab, dies with it. Nothing stored — `src/store/mod.rs`
   never-store rule untouched (the transcript is Claude Code's own artifact;
   we hold a read offset in the client only).
3. **Web UI:** an "Activity" tab beside the terminal in the pane view.
   Append-only `<pre>`/list — no xterm, no reset-rewrite cycle, no
   `preserveScrollTop()` acrobatics. Client keeps a **200-line ring**: trim
   from the top past 200. Stick-to-bottom with release-on-scroll-up, per the
   collie `use-auto-scroll` findings already consulted
   (`docs/distillery/reports/distill-consult-terminal-scroll-2026-07-28.md`) —
   within the 200-line window only; scrolling up never triggers any fetch.
4. **Line rendering:** one transcript record → one or a few display lines:
   `> bash: <command>`, output lines verbatim, `✎ edit: <file>`, assistant
   text, `· thinking` (collapsed to one line), `⑂ subagent: <desc>` for
   sidechain frames (flat lines, no drill-in — nesting is out of scope).
   Per-record output capped (e.g. first N lines + "… +K lines" marker, open
   decision 3) so one huge `tool_result` cannot flush the whole ring.

## What is explicitly out of scope (was in the original draft)

- Retroactive history / reading from session start — cut by user.
- Continuation-chain following with the never-show-less guard — mostly moot
  without backfill; the one residual case (agent resumed *while being watched*
  starts a new file) is open decision 2.
- Pagination, `hasOlder`, cursor persistence across sessions — all cut.
- Subagent drill-in views, per-subagent timelines — flat lines only.
- Any change to the terminal view, PageUp injection, PBI-057's buffer idea,
  keys/reply input paths — untouched. PBI-057 remains the candidate for
  non-Claude panes if smooth pixel scrollback is ever still wanted.

## Cost, concretely

- One parser over an unversioned external format — same mitigation as before:
  defensive per-line parse, raw passthrough for unknown shapes, fixture corpus
  recorded from the installed Claude Code version so drift breaks tests, not
  the view. The tail-only scope shrinks the parsed surface to the handful of
  record types that occur mid-session.
- Pane→file resolution heuristic (cwd + "currently growing") — wrong-file risk
  when two live Claude sessions share a cwd; one-worktree-per-agent makes cwd
  unique in orchestration use, plain use needs the rule in decision 1.
- Windows file-sharing semantics for reading a file being appended by another
  process — read-share open, partial-line handling as above.

## Benefit, concretely

- Meets the stated need exactly: open the view, watch a complete, steadily
  appending log of what the agent is actually doing — every command with its
  real output — with zero loss between polls, bounded at 200 lines.
- Trivial resource profile: no background pollers, no gateway memory, no
  storage; cost is one seek+read per open browser tab per 1.5 s.
- Sidesteps PBI-058's stuck-scroll failure mode entirely for this view (no
  escape injection anywhere in the path).

## Open decisions to lock before any code

1. **Session-file pick at open time** — "the currently growing file": newest
   mtime in the encoded-cwd directory? newest whose size increases across one
   probe interval? On ambiguity (two live sessions, same cwd): pick newest
   with a visible "watching session <short-id>" label, or refuse with an
   explanation?
2. **Mid-watch file switch** — if the watched file stops growing and a newer
   sibling appears (resume/fork while viewing), does the tail silently
   re-resolve and continue (recommended: yes, from the new file's EOF), or
   surface a "session restarted" divider line?
3. **Per-record output cap** — how many lines of one `tool_result` render
   before eliding (candidate: 20 lines + "… +K lines"), so a single huge
   output can't flush the entire 200-line ring.
4. **Privacy tier** — transcript records contain file contents and command
   output. Today "Tier 2 raw terminal streams are NOT redacted (PRD §7)"
   (`src/security/redact.rs:10`). Confirm transcript lines inherit Tier 2
   verbatim (they show the same class of content the terminal already shows),
   or decide otherwise before shipping.
5. **200-line ring ownership** — client-only (gateway fully stateless,
   recommended) vs server-trimmed responses. Client-only means a slow poll
   after a burst may deliver >200 lines in one response and the client trims;
   confirm that's acceptable.

## Recommendation

Build this narrowed version. It is strictly smaller than the original
transcript plan (no chain walking, no pagination, no nesting), strictly more
honest than any pixel-side fix could be about the one thing the user asked for
(completeness from open time), and additive to every existing surface. Scope
now looks like a small lane: one module + one route + one tab.

## Next step

Not started. Route through bee-hive exploring to lock the 5 decisions above
(1 and 4 are the load-bearing ones), recording a fixture transcript from the
locally installed Claude Code as the feasibility check.

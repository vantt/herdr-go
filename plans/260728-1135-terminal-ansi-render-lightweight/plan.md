# Terminal ANSI Render — Lightweight Alternative to xterm.js — Discussion Plan

**Status:** discussion only, no decisions locked, no code started.
**Slug:** terminal-ansi-render-lightweight
**Related:** `terminal-scroll-nudge-buttons`, `terminal-scrollback-agent-panes` (PBI-056),
`plans/260728-1015-pane-scrollback-buffer/` (independent decision — see "Relationship to the
buffer plan" below), `docs/distillery/reports/distill-consult-terminal-scroll-2026-07-28.md`
(source material this plan is based on).

## Why this exists

Discussion (2026-07-28) consulting the `collie` reference project's terminal-rendering approach
surfaced a load-bearing fact, verified independently on BOTH sides:

- **collie** (`web/src/lib/ansi.ts:1-5`): *"Herdr's `pane.read(format:"ansi")` emits ONLY SGR
  color/style sequences — no cursor moves, no OSC — so we don't need a terminal emulator, just
  SGR."*
- **this repo** (`src/herdr/pane_scroller.rs:274-275`): *"there is nothing else in practice that
  `format: ansi` reads emit"* besides CSI sequences. Same herdr API (`format: "ansi"`,
  `src/herdr/socket.rs:469`), same empirical finding, written by two independent codebases.

The host's live terminal view (`web/src/views/terminal.ts`) renders this data through
**xterm.js** — a full terminal emulator (virtual grid, cursor engine, its own scrollback model
fought via `scrollback: 0` + a custom `HISTORY_SCROLL_THRESHOLD`) — even though `disableStdin:
true` (`terminal.ts:176`) confirms it is used purely as a **read-only mirror**, never for typed
input. collie proves a ~250-line hand-rolled SGR parser (`ansi.ts`) rendering plain React text
nodes (`ansi-output.tsx`, a `<pre>` + `<span>` tree) is sufficient for the identical job —
including mirroring the same class of agent (Claude Code's full-screen alt-screen TUI).

## Two approaches compared

| | xterm.js (today) | Hand-rolled SGR parser + `<pre>` (collie-style) |
|---|---|---|
| Bundle/runtime weight | Full terminal emulator: virtual grid, cursor state machine, canvas/DOM renderer | ~250 lines of parsing + plain DOM text nodes |
| Internal scrollback conflict | Has its own scrollback/viewport model; must be suppressed (`scrollback: 0`) and worked around (`HISTORY_SCROLL_THRESHOLD`) to coexist with the app's own scroll/nudge logic | None — no competing internal buffer; a future gateway-side buffer (see below) becomes the single source of truth with nothing to fight |
| Pan for wide content | `term.resize(cols, rows)` fixed grid; user pans/zooms the grid (`terminal.ts:77-78`) | `overflow-x-auto` + `whitespace-pre` on the `<pre>` — collie already does this for wide TUI tables (`ansi-output.tsx:76-81`) |
| Zoom | Grid re-render at new cols/rows | `fontSize` prop, already supported in collie's `AnsiOutput` |
| Spec coverage | Full terminal spec by construction: tab stops, wide (CJK) glyph width, combining characters, exotic CSI | "Minimal, safe" by the source's own description — unhandled escape sequences are **silently skipped**, not handled; no explicit tab (`\t`) case in `parseAnsi` (unverified whether herdr's `format:ansi` output ever contains a literal tab) |
| URL auto-linkify | `WebLinksAddon` (already wired, PBI-030/PBI-047) | Would need an equivalent added by hand |
| Load-older / PageUp feel | Full-replace per poll (`term.reset()` + `term.write()`) | Full-replace per poll (re-run `parseAnsi` on the new text, React reconciles the `<pre>`) — same behavior, same "jumps by page" ceiling, because that ceiling is set by the **PageUp escape-injection mechanism itself** (`pane_scroller.rs`), not by the renderer |

## Relationship to the buffer plan

This is a **separate, independent decision** from `plans/260728-1015-pane-scrollback-buffer/`:

- The render swap changes **how already-fetched content is displayed**. It does not change how
  content is fetched, how much is fetched, or the poll/PageUp interaction model.
- The buffer plan changes **how much history is available** (continuous gateway-side accumulation
  vs. today's page-jump PageUp replay). Its own cost drivers (background poller per pane, watch
  lifecycle, cross-pane memory bound) are unaffected by which renderer eventually displays the
  result.
- They compose cleanly if both are done: a plain-DOM renderer has no competing internal buffer to
  fight, so a future gateway-side line buffer would render through the same
  parse-and-replace path this plan describes, using the same anchor-capture/restore-on-grow
  technique collie's own history view already demonstrates (`web/src/routes/history.tsx`, lines
  ~83-92, 139-146) for inserting older content above the viewport without yanking the read
  position.
- Neither plan depends on the other landing first.

## Cost, concretely

- Reimplementing correctness collie's parser explicitly does not guarantee: tab handling, wide
  (CJK/emoji double-width) glyph alignment, any exotic escape sequence beyond SGR/CSI-final-`m`.
  Risk is silent misrender, not a crash — needs deliberate test coverage before trusting it in
  production, not just porting the ~250 lines as-is.
- `WebLinksAddon` equivalent needs hand-rolling or a small dependency.
- Full migration touches every place `terminal.ts` currently assumes an xterm `Terminal` instance
  (resize/cols/rows math, keyboard-inset interaction, theme wiring) — bounded in size (collie's
  proof is ~250 lines for the parser core) but still a render-engine swap, not a tweak.

## Benefit, concretely

- Removes the `scrollback: 0` workaround and the friction of fighting xterm's own viewport model
  with `HISTORY_SCROLL_THRESHOLD` — the app's own scroll state becomes the only state.
- Smaller bundle, fewer moving parts for a read-only-mirror use case that never needed a full
  terminal emulator's input/cursor machinery in the first place (`disableStdin: true` already
  proves this).
- Directly compatible with (not a prerequisite for) the pane-scrollback-buffer plan, should that
  ever be built.

## Open decisions to lock before any code

> Superseded in part — see "Revised state of the open decisions" at the end of this file
> (2026-08-11). The list below is the original framing, kept for context.

1. **Correctness coverage** — decide what "good enough" means for tab stops, wide-glyph
   alignment, and unhandled escapes before committing to the swap; collie's own scope ("minimal,
   safe" for their observed traffic) may not be sufficient here without verification against this
   repo's actual `format:ansi` output across agent types (Claude/Codex/Agy/shell).
2. **URL auto-linkify equivalent** — hand-roll or find a small dependency; must preserve the
   existing PBI-030/PBI-047 behavior (see open question already on PBI-047 about markdown-link
   rendering).
3. **Hidden xterm-specific dependencies** — full read-through of `terminal.ts` to confirm nothing
   else (visual cursor cue, some other xterm-only API) is silently relied on beyond what's already
   identified (resize math, keyboard inset, theme).
4. **Migration boundary** — one PR/cell replacing the renderer only, keeping today's PageUp/Down
   interaction untouched, so the swap can be verified in isolation from any buffer work.

## Recommendation

Worth a dedicated exploring-style pass given the concrete, cited evidence (both codebases
independently confirmed the same backend data-shape property). Not a tiny cell — same class of
scope as `terminal-scroll-nudge-buttons`, bounded but touching a core rendering path. Lock the 4
decisions above first.

## Next step

Not started. Waiting on the decisions listed under "Revised state of the open decisions"
(2026-08-11) — now 7, not 4. The cheapest next action that is also a real stop condition: capture an
ANSI fixture corpus from live panes of all four kinds and measure DOM render cost at 400/1000 styled
lines against xterm (decisions 1 and 5). A bad measurement kills the swap before any port work.

## Zoom is not a driver for this decision (2026-08-07)

Discussion surfaced "no pinch-to-zoom" as a pain point and asked whether it favors the collie-style
swap. Checked directly: collie has no pinch either (`grep pinch upstreams/collie/web/src` — 0 hits);
its zoom is the same font-size-button pattern this repo already had, over a narrower range (9-16px vs
this repo's 7-22px). Swapping renderers would not have added pinch and would have shrunk the existing
zoom range.

Pinch was implemented directly on the current xterm.js renderer instead (`web/src/views/terminal.ts`,
commit history) — a `touchstart`/`touchmove`/`touchend` handler that previews via CSS `transform:
scale()` (GPU-composited, no per-frame char-cell re-measure) and commits once, on release, through the
same `setFont()` A−/A+ already call. Cost of pinch turned out independent of which renderer is
underneath: the expensive part (xterm's char-cell re-measure) only happens once per gesture, not once
per frame, either way.

Net: the 4 open decisions above are unchanged and renderer-swap evaluation should continue to ignore
zoom/pinch as a factor in either direction.

## New evidence (2026-08-11) — line wrap is the decisive driver, and open decision #1 largely resolves

Session re-examined the swap with a new requirement on the table: **soft-wrapping content that is not
a table/grid** — the one capability the current view lacks. Findings, each independently verified:

### 1. The wrap gap is architectural in xterm, not a missing feature

The current view **never wraps**, by construction:

- `cols` comes from the *content*, not the viewport: `clamp(max(stripAnsiLen(line)) + 1, 20, 400)`
  (`web/src/views/terminal.ts:290-298`). No FitAddon is used — `@xterm/addon-fit` is declared
  (`web/package.json:16`) but imported nowhere, i.e. a dead dependency.
- CSS pins natural width: `width: max-content !important` (`web/src/styles.css:1156-1170`).
- The spec states it as intent: *"terminal lines keep their natural shape rather than wrapping to the
  phone width"* (`docs/specs/terminal-detail.md:58`).

To wrap under xterm, `cols` must become the viewport width in characters — at which point **xterm
applies one wrap policy to the entire buffer**, breaking box-drawing/tables along with prose. xterm.js
exposes no per-line wrap control; wraparound is a property of the buffer, not of a line.

**This is the only decisive argument for the swap.** Bundle size and dropping the `scrollback: 0` /
`HISTORY_SCROLL_THRESHOLD` workarounds remain true but are secondary.

### 2. Open decision #1 (correctness coverage) is much smaller than assumed

The plan's premise rested on two empirical observations ("verified against a live pane" / "nothing
else in practice"). There is a stronger, structural guarantee upstream:

`pane.read(format:"ansi")` is served by ghostty's `FormatterFormat::Vt` serializing **a selection of
the already-rendered cell grid**, with `unwrap`/`trim` flags
(`upstreams/herdr/src/ghostty/mod.rs:898-912`, `1005-1012`; call chain
`upstreams/herdr/src/pane/terminal.rs:400-402` → `src/app/api/panes.rs:1194`). A grid dump has no
cursor, so it **cannot** emit CUP/EL/ED/alt-screen/scroll-region. The escape set is closed by the
producer's construction, not by luck.

Residual correctness risk narrows to: wide-glyph (CJK/emoji) advance width under DOM font metrics
rather than a grid — the one place xterm is genuinely stronger, since browser monospace fonts are
unreliable for emoji — and literal tabs, for which collie's parser has no explicit branch.

### 3. A renderer swap alone does not deliver clean wrap — the backend half is unwired

The pty has already hard-wrapped its output. Soft-wrapping that again yields ragged double-wrapping
at the pty's own column boundary.

herdr already solves this and this repo does not use it: `recent_unwrapped_ansi()` joins soft wraps
while **preserving SGR** (`upstreams/herdr/src/pane/terminal.rs:400-402`), served over the socket
(`upstreams/herdr/src/app/api/panes.rs:1194`), with upstream tests for both line-joining and CJK
(`upstreams/herdr/src/pane/terminal.rs:4141-4160`). Here, `ReadSource` has only `Visible`/`Recent`
(`src/herdr/mod.rs:59-60`, wired at `src/herdr/socket.rs:469-473`).

Note collie does **not** do this — it wraps already-wrapped content. Porting collie alone reaches
"readable", not "clean".

Convenient alignment: `recent_unwrapped` applies to scrollback (primary-screen panes — shell, logs),
while alt-screen panes have no scrollback and fall back to `visible` via `PaneScroller`
(`src/herdr/pane_scroller.rs:95-99`). Alt-screen content is grid art that must *not* wrap. The
capability lands exactly where it is wanted.

### 4. Cost correction — this repo has no React

The plan describes the port as "~250 lines" citing collie's `ansi.ts`. Collie is React; **this repo's
`web/` is vanilla TS** (no react in `web/package.json`). Consequences:

- `lib/ansi.ts` (245 lines, framework-free) — portable near-verbatim.
- `ansi-output.tsx` (289 lines, React + Tailwind + `React.memo`/`useMemo`) — **must be rewritten in
  vanilla DOM**, including hand-rolled memoisation. Not a copy.
- `blocks.ts` (165) — only `splitLines` is needed; the Block AST exists to lift Claude dialogs, out of
  scope here.
- `find.ts` / `use-auto-scroll.ts` / `chat-message-list.tsx` (~244) — **not needed**; this repo has its
  own scroll/nudge machinery (R16-R19).

Offsetting this, the coupling surface is small: exactly **one** non-test module imports `@xterm/*`
(`web/src/views/terminal.ts`, 813 lines) plus `web/test/terminal.test.ts`.

### 5. Counter-risk: a plain `<pre>` may be *slower* than xterm, against the "smooth" goal

xterm's DOM renderer paints only the rows inside its grid viewport. A flat `<pre>` paints every line
it holds. Collie does **not** virtualize (`chat-message-list.tsx` is an ordinary scrolling div),
relying on herdr's 1000-line clamp plus `React.memo`/`useMemo`. This repo caps at
`HISTORY_ROW_CEILING = 1000` (`web/src/views/terminal.ts:42`) and re-renders whenever the polled text
changes — i.e. every 1.5s while an agent is active.

**This must be measured before committing.** If DOM render at 400-1000 styled lines is slower than
xterm, the swap damages the very goal (smoothness) that motivates it, and virtualization enters
scope — at which point "port ~250 lines" no longer describes the work.

### 6. Relationship to PR #5 (Log view) — orthogonal, but decide together

PR #5 (`vantt/herdr-go`, OPEN, CONFLICTING, head `de71998`, local ref `pr5`) adds a **second
viewport** beside xterm: a `<pre class="activity-pre">` with `white-space: pre-wrap; overflow-wrap:
anywhere` tailing Claude Code's on-disk JSONL transcript via a new `GET /api/panes/:pane/activity`.
Its own CSS comment states this plan's thesis independently: *"Wrap long lines instead of panning —
the tail is prose-and-commands, not a fixed-width grid."*

It obtains wrap by **separating the source**, not by per-line heuristics: the transcript knows
semantically what is text vs tool output vs thinking, so nothing is guessed. But its scope is
narrower and its trade-offs real:

- Claude Code panes only; shell/Codex/Agy panes get no wrap anywhere.
- The terminal *mirror* still cannot wrap — the operator switches views rather than wrapping the
  mirror.
- ANSI is **stripped server-side** (`clip()`, `src/transcript/mod.rs` ~line 570 on `pr5`) → no colour.
- Hard caps: tool output 40 lines, text 80 lines, 400 chars/line, 400 lines/poll, 200-line client ring.

The two changes are technically orthogonal (PR #5 does not touch the renderer). The decision is not
"which one" but **how many render paths the app should carry**: landing PR #5 and later doing this
swap yields three unless the `<pre>` container, stick-to-bottom logic, and ring buffer are unified
deliberately. Decide both together.

Supporting datum from the same period: `e8947e5` (*"tolerate pty line-wrap in the reply-guard's
landed-text match"*) is a bug caused by pty hard-wrap — evidence the hard-wrap is already costing
elsewhere, independent of rendering.

### Revised state of the open decisions

1. **Correctness coverage** — largely resolved by the grid-dump guarantee (§2). Remaining: wide-glyph
   advance width and literal tabs. Needs a fixture corpus from real panes of all four kinds
   (Claude/Codex/Agy/shell), not reasoning.
2. **URL auto-linkify** — unchanged, still must be hand-rolled; note collie has **no** link detection
   to port from (OSC-8 is consumed and dropped).
3. **Hidden xterm dependencies** — bounded and now enumerated: one source module + one test module;
   the test asserts against `.xterm-rows`/`.xterm` DOM and stubs `matchMedia`/canvas.
4. **Migration boundary** — unchanged, and now must also state its relationship to PR #5 (§6).
5. **NEW — render performance** — ~~stop condition, measure first~~ **MEASURED 2026-08-11, green.**
   Full numbers: `plans/reports/terminal-render-swap-dom-cost-measurement-260811-1219-report.md`.
   Real captures give **2.8-3.2 spans/line** (Claude panes) and **5.7** (colour-heavy shell:
   `git log --graph` + `ls --color` + `cargo tree`), not the dozens feared — span count is bounded
   by the SGR-sequence count, which the fixtures show is ~3/line. Parse is ~1 ms per 1000 lines.
   Decisive: the **live poll only requests 80 lines** (`src/web/screen.rs:58`), so the recurring
   1.5 s repaint is ~256-456 spans; the 400/1000-line case (~2 300-5 700 spans) is a one-off on
   operator-triggered history load (`RECENT_LINES_CAP = 1000`, `src/herdr/pane_scroller.rs:22`).
   For comparison the current renderer already does a whole-text `term.reset() + term.write()` on
   every changed poll. Remaining gaps: browser layout/paint not timed (no system chromium here) and
   nothing measured on a real phone; no Codex/Agy fixture (no such pane was running).
   §5 above stands as the original concern, now answered.
6. **NEW — wrap policy** — **LOCKED 2026-08-11 (user decision).** Fully automatic, per block,
   **no UI control of any kind** — no toggle, no per-block override, no affordance. The operator
   explicitly ruled out buttons. When the classifier is unsure it **errs toward pan**.

   Rationale — the cost of being wrong is asymmetric, and with no override that asymmetry becomes
   the architecture:

   | Misclassification | Consequence |
   |---|---|
   | structured block treated as prose → wrapped | **Destructive** — alignment lost, unreadable, no way to recover |
   | prose block treated as structured → panned | **Exactly today's behavior** — invisible non-event |

   So pan is the default and wrap is the path that must prove itself. A classifier failure degrades
   silently to the current UX rather than breaking anything. Accepted cost: some prose will not wrap
   and the feature will silently not fire on it.

   This matches the fail-closed detector contract the distillery already recorded from collie
   (`docs/distillery/sources/collie.md`, `harness-fail-closed-detector-contract`: return nothing
   rather than a best guess).

   **Signal separation, measured on real captures** (probe:
   `.bee/spikes/terminal-render-bench/probe-wrap-signals.mjs`; fixtures uncommitted):

   | Signal | Claude panes | Colour-heavy shell | Verdict |
   |---|---|---|---|
   | 5-line window with >=2 stable column gutters | 7-11% | 66% | **strongest** (6-9x) |
   | inner gutter (2+ spaces mid-line) | 2-8% | 52% | strong (~10x) |
   | alphaRatio >= 0.6 | 82-91% | 41% | moderate |
   | sentence punctuation | 58-64% | 38% | weak |
   | box-drawing character | 20-22% | 41% | **weak alone** — Claude's own TUI frame trips it |
   | aligned pipes / horizontal rules | 0% | 0-20% | markdown tables only |

   Two corrections this measurement forced: box-drawing is **not** sufficient as the primary signal
   (an earlier assumption), and stable-gutter analysis over a window is what catches the
   whitespace-aligned tables (`ls -la`, `ps aux`, `df -h`) that carry no drawing characters at all.

   **Proposed rule** (thresholds to be tuned on a corpus, not fixed by argument):

   ```
   per block (segmented at blank lines):
     pan   if  >=2 stable gutters | >=30% box-drawing lines | aligned pipes
     wrap  if  every line is either shorter than the viewport OR was soft-wrapped by the pty
     pan   otherwise                                        <- default
   ```

   The second clause uses a signal from the producer rather than from line shape: a line the pty
   soft-wrapped is one the program emitted **without choosing its own break**, which is what prose
   does and what a table (one newline per row) does not. Verified live — see decision 7.

   Also in scope, and button-free: for a block only mildly too wide (roughly 1-2x viewport), scale
   that block's font down to fit instead of demanding a horizontal drag. Monospace plus a uniform
   scale preserves column alignment exactly, so nothing is lost; below a minimum size it falls back
   to pan.

   **Known weak spot:** alt-screen panes (Claude Code) expose no pty soft-wrap signal at all — only
   `visible`, a grid the TUI hard-wrapped itself — so the second clause is unavailable there and the
   decision rests on gutters/box-drawing alone. The measured numbers suggest that still works
   (prose blocks wrap, the TUI frame pans), but breaks land on the TUI's own column boundary rather
   than cleanly.

   **Stability hazard, easy to miss:** the poll rewrites content every 1.5 s. If block boundaries or
   classifications flip between polls the layout jumps — which damages the very smoothness this
   feature exists to improve, and would look like a bug rather than a heuristic miss. Mitigations to
   design in from the start: anchor blocks to blank lines (far more stable than content-derived
   boundaries), add hysteresis so leaving the structured classification needs stronger evidence than
   entering it, and skip re-classification when a block's content hash is unchanged. The
   classification must also depend only on content, never on viewport width, so rotating the phone
   or raising the keyboard cannot re-classify anything — width may only affect how a decision is
   applied.

   **Stop condition:** if tuning cannot reach a miss rate the operator trusts, fall back to a manual
   toggle. A mechanism that cannot be trusted is worse than an honest switch — but note the operator
   has ruled that out for now, so reaching this point means reopening the decision with them.
7. **NEW — backend half** — wire `ReadSource::RecentUnwrapped` (§3); without it the wrap is ragged.
   **Verified live 2026-08-11** on a throwaway pane: a 1587-char logical line comes back from
   `--source recent` as **7 physical lines of ~231 chars** (the pane width) and from
   `--source recent-unwrapped` as **one 1587-char line**. When no line exceeds the pane width the
   two sources return byte-identical output — `recent-unwrapped` is not a different format, only a
   rejoin of what the pty broke.

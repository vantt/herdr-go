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

Not started. Waiting on the 4 decisions above.

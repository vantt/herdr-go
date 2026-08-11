# Phase 2 — Vanilla DOM renderer replaces xterm.js

**Status: code complete and browser-verified 2026-08-11. Real-device checks outstanding.**

Shipped: `web/src/terminal-render.ts`, `web/src/linkify.ts` (+ 11 tests), the nine call-site changes in
`web/src/views/terminal.ts`, the CSS swap, and the test rewrite. 188 web tests green, `tsc` clean,
full Rust chain unaffected and green.

**Bundle:** JS 325.71 kB -> **37.19 kB** raw, 83.44 kB -> **11.54 kB** gzipped (-86%). CSS 27.17 ->
23.10 kB (xterm.css gone). `@xterm/addon-fit` and `@xterm/addon-web-links` removed outright;
`@xterm/xterm` moved to devDependencies because the phase-1 equivalence harness still drives it as
the reference implementation.

**Deliberate behaviour changes, both trivial but real** — recorded rather than quietly absorbed:

- One fewer trailing blank line. The old code sized the grid to `lines.length + 1`; the extra row was
  a spare the fixed grid needed and has no counterpart here. Three test expectations moved by one
  (601->600, 501->500, 11->10).
- `HISTORY_ROW_CEILING` is gone. It existed because a fixed grid discards rows past its own height —
  the constant's own comment says the previous value "silently discarded" history. With no grid,
  every line herdr returns is rendered, and herdr's own 1000-line cap is the real bound.

Also removed: `stripAnsiLen` (existed only to size the grid) and its tests, and the `matchMedia` and
canvas stubs that existed only so xterm could be constructed under jsdom.

**Browser-verified 2026-08-11.** Rendered in chromium at 390px against real captures: colours, bold,
italic and dim survive, the URL is a real anchor, and the side-by-side against xterm on the same
capture shows the same content — with xterm cut off at the right edge where the new renderer wraps it
into view.

Phase 1's carried-over glyph question is answered, and then fixed. Vietnamese diacritics were
displaying displaced ("nêú" for "nếu"), and the cause was neither the parser nor the renderer: the
capture is entirely precomposed with zero combining marks, xterm displaced them identically on the
same data, and a bare `<pre>` with the same font stack did too while serif was correct.

Rendering the same text in each installed monospace family isolated it: **DejaVu Sans Mono** has no
precomposed Vietnamese glyphs and the browser synthesises them onto the following character — and it
is what the generic `monospace` keyword resolves to on many Linux systems. Noto Sans Mono, Liberation
Mono, Ubuntu Mono and Noto Mono all render correctly.

Fixed by naming Vietnamese-capable families ahead of the generic keyword in `--font-mono`, and
pointing `.term-screen` at that variable instead of carrying its own copy of the stack. Costs
nothing — no webfont, no bytes. Re-verified: the same capture now renders every tone mark correctly.
A self-hosted subset webfont remains the option if a real device ever turns out to lack all of them.

**Still unverified:** the R1-R20 checklist against the running app (this exercised the renderer in
isolation, not the view's poll/scroll/sheet machinery), pinch-to-zoom, and the keyboard inset —
all of which need real touch or a real device.

**Goal:** remove xterm.js from the terminal view with **zero visible change**. Same pan-not-wrap
behaviour, same PageUp/Down history, same pinch and zoom, same keyboard inset. The wrap feature does
not appear here — it is phase 4.

**Why isolate it:** this is open decision 4 in the decision record. A swap that changes nothing is
verifiable against the existing spec; a swap bundled with new behaviour is not.

**Stop condition:** any rule R1-R20 that cannot be preserved fails acceptance criterion 2 — revert.

## The complete xterm surface to replace

Eight call sites, one module, enumerated from `web/src/views/terminal.ts`:

| Line | Use | Replacement |
|---|---|---|
| 1-3 | `Terminal`, `ITheme`, `WebLinksAddon`, `xterm.css` imports | own modules |
| 210-218 | `new Terminal({...})` | none — no emulator to construct |
| 219 | `term.open(viewport)` | build DOM into the viewport directly |
| 220 | `loadAddon(new WebLinksAddon())` | hand-rolled linkifier |
| 290-298 | `term.cols/rows`, `term.resize()` | drop entirely — no grid to size |
| 299-300 | `term.reset()` + `term.write()` | parse + rebuild the line DOM |
| 421 | `term.options.fontSize` | CSS font-size on the container |
| 437 | `term.element` (pinch transform target) | the container element |
| 707 | `term.dispose()` | remove listeners, drop the node |

`@xterm/addon-fit` is already dead (declared at `web/package.json:16`, imported nowhere) and goes
with the rest.

## Requirements

- **Behaviour identical.** Long lines keep their natural shape and the operator pans, exactly as
  `docs/specs/terminal-detail.md:58` records. No wrapping in this phase.
- **Linkify restored.** The Data Dictionary (row 3) promises "any URL in the output renders as a
  clickable link"; PBI-030 delivered it via `WebLinksAddon`. Collie has **no** link detection to port
  — this must be written. Bare `http(s)://` URLs only, matching today's behaviour; markdown links
  stay out of scope (PBI-047).
- **Text nodes only, never `innerHTML`.** Content is untrusted terminal output.
- Preserve the dedupe guard (`text === lastText`, `terminal.ts:272`) so unchanged content is not
  rebuilt.
- Preserve `preserveScrollTop()`'s distance-from-bottom anchoring across redraws.

## Files

**Create**
- `web/src/terminal-render.ts` — StyledLine[] → DOM, plus mount/update/teardown.
- `web/src/linkify.ts` — URL detection over styled segments, splitting a segment around a match
  without disturbing its styling.
- `web/test/linkify.test.ts`.

**Modify**
- `web/src/views/terminal.ts` — replace the nine sites above.
- `web/src/styles.css` — remove the xterm-specific overrides (`.term-viewport .xterm`,
  `.xterm-viewport`, `width: max-content !important`, `pointer-events: none`,
  `height: max-content` at 1096-1100 and 1156-1178) and give the new container equivalent geometry.
- `web/test/terminal.test.ts` — assertions currently read `.xterm-rows`/`.xterm` DOM directly
  (e.g. 282-298, 649, 672, 742) and stub `matchMedia` plus canvas for xterm's renderer (15-32).
  Rewrite against the new DOM; **delete the stubs rather than keeping them working**, since their
  reason for existing disappears.
- `web/package.json` — drop `@xterm/xterm`, `@xterm/addon-fit`, `@xterm/addon-web-links`.

## Steps

1. Build `terminal-render.ts` against phase 1's model, mounting into `.term-viewport`.
2. Port `TERMINAL_THEME` from xterm's `ITheme` shape to the renderer's own colour resolution, keeping
   the same values so the view is pixel-comparable.
3. Write the linkifier and its tests before wiring it, so the parity gate is objective.
4. Swap the call sites in one pass. Keep every surrounding behaviour — poll guard flags, nudge
   buttons, history load, reply guard, keyboard inset — untouched.
5. Repoint pinch-to-zoom at the new container. The gesture already previews via CSS `transform:
   scale()` and commits once through `setFont()`, and that design was shown renderer-independent on
   2026-08-07 — it should need only a new element reference.
6. Rewrite the tests, then remove the dependencies.

## Validation

- **R1-R20 checklist**, rule by rule, against `docs/specs/terminal-detail.md`. Every rule is either
  demonstrably preserved or the phase stops. This is the phase's primary gate.
- Data Dictionary row 3: URLs clickable.
- Side-by-side comparison against the pre-swap build on the phase-1 corpus — colours and layout.
- Full verify chain green.
- Bundle size recorded before and after (a secondary benefit, not a justification). **Baseline
  measured 2026-08-11, before any swap work: 325.71 kB raw / 83.44 kB gzipped.**
- Answer phase 1's carried-over question here: how CJK and emoji measure under DOM font metrics
  versus xterm's grid. It needs a real renderer, so this is the first phase that can settle it.

## Risks and rollback

- **Linkify is the one genuine capability loss** if the rewrite is weaker than `WebLinksAddon`. Gate
  it with tests written first.
- **Hidden xterm dependency** beyond the nine sites — the enumeration above is from grep and should
  be re-confirmed by a full read of `terminal.ts` before starting (open decision 3).
- Rollback: the phase is one commit touching one view plus its test. Reverting restores xterm
  wholesale, since no other module imports it.

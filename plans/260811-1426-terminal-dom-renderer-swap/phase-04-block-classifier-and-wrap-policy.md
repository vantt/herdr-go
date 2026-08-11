# Phase 4 — Block classifier + automatic wrap policy

**Status: code complete and browser-verified 2026-08-11. Real-device checks outstanding.**

Shipped: `web/src/block-classify.ts` (+19 tests), per-block rendering in `terminal-render.ts`
(+17 tests), the CSS, the live read switched to `RecentUnwrapped` with a route test that fails if it
regresses, and **R21-R28** written into `docs/specs/terminal-detail.md`. 224 web tests, 347 Rust
tests, `tsc`, clippy, fmt and rename-contract all green.

## Departures from the plan above, and why

**The pty soft-wrap signal was dropped.** The sketched rule used per-line "did the pty break this"
flags as positive evidence for wrapping. Two problems surfaced while building it:

1. Obtaining those flags needs **two** backend reads per poll — `recent` diffed against
   `recent_unwrapped` — a cost the plan never priced.
2. The first implementation had the logic backwards: it treated a line the pty did *not* wrap as
   evidence *against* wrapping. That is wrong. A line that was not broken merely fit; it says nothing
   about whether the program chose its shape.

Since the structure signal alone already separates the measured cases (7-11% of Claude windows
versus 66% of shell windows), the parameter was removed rather than shipped half-right. It stays a
candidate refinement if the structure detector proves insufficient in use.

**Arrows and geometric shapes were removed from the box-drawing class.** A test caught the exact
false positive the plan warned about: one `→` in a three-line paragraph is 33% of its lines, over the
30% threshold, so ordinary prose panned. Arrows and bullets are prose decoration, not layout, so the
class narrowed to U+2500-259F — box drawing and block elements, which is what `cargo tree` and
`git log --graph` actually draw with. Note this makes the shipped detector stricter than the
exploratory probe that produced the measurements above, which used the wider class.

**Panning moved from the page to the block.** A wrapping block has to wrap to what the reader can
see, which means the screen container is viewport-width; inside a `max-content` container it would
have wrapped to whichever block happened to be widest. So each pan block carries its own horizontal
scroller instead of the whole page panning as one unit. This is a real interaction change from the
pre-swap behaviour, and it is also what keeps a table's rows moving together.

**Copy fidelity needed explicit work.** A pan block must be a block-level box to have a scroller, and
`textContent` concatenates across block boundaries without a newline — so a copied selection would
have run the last line of one block into the first of the next. Zero-height separator spans carry
those newlines. `terminal-render.test.ts` asserts the screen text round-trips exactly, which is the
guard on all of this.

## Browser verification (2026-08-11)

Run at 390px through the app's own modules and stylesheet: prose wraps and fits, laid-out blocks
occupy exactly as many rows as they have lines, a wide table scrolls on its own, the page never pans
sideways, blank lines keep a line's height, blocks sit flush, and copied text round-trips. Green on
the synthetic screen and all seven real captures.

**One defect only looking could find:** blocks rendered a full line apart. The separator spans
measured 0 high, but a zero-height inline between two block boxes sits in an anonymous block whose
line box keeps a strut from the screen's font — so the measurement said 0 while the layout said
otherwise. Fixed by making the separator a zero-height block with hidden overflow. The check now
measures the gap *between* blocks rather than the separator's own height, which is the property that
actually mattered.

## Still outstanding

Anything needing real touch or a real device: the per-block horizontal scroll's feel, pinch-to-zoom,
the keyboard inset, and R21-R28 seen in the live app rather than in the renderer in isolation.

**Goal:** the actual feature. Prose wraps to the viewport; structured content keeps its shape.
Fully automatic, **no UI control of any kind**, erring toward pan when unsure.

**Depends on:** phase 2 (per-line rendering must exist) and phase 3 (detection degrades badly on
wrapped data).

## The locked design

The operator ruled out buttons — no toggle, no per-block override, no affordance. That constraint is
what makes the default direction load-bearing:

| Misclassification | Consequence |
|---|---|
| structured treated as prose → wrapped | **Destructive** — alignment lost, unreadable, unrecoverable |
| prose treated as structured → panned | **Exactly today's behaviour** — an invisible non-event |

So pan is the default and wrap is the path that must prove itself. A classifier failure degrades
silently to the current UX. Accepted cost, stated plainly: **some prose will not wrap and the feature
will silently not fire on it.**

This is the same fail-closed contract the distillery already recorded from collie
(`harness-fail-closed-detector-contract`): return nothing rather than a best guess.

## Measured signal separation

From `.bee/spikes/terminal-render-bench/probe-wrap-signals.mjs` on real captures:

| Signal | Claude panes | Colour-heavy shell | Verdict |
|---|---|---|---|
| 5-line window with >=2 stable column gutters | 7-11% | 66% | **strongest** (6-9x) |
| inner gutter (2+ spaces mid-line) | 2-8% | 52% | strong (~10x) |
| alphaRatio >= 0.6 | 82-91% | 41% | moderate |
| sentence punctuation | 58-64% | 38% | weak |
| box-drawing character | 20-22% | 41% | **weak alone** — Claude's own TUI frame trips it |
| aligned pipes / horizontal rules | 0% | 0-20% | markdown tables only |

Two corrections this forced, both against earlier assumptions in this work: box-drawing is **not**
sufficient as the primary signal, and stable-gutter analysis over a window is what catches the
whitespace-aligned tables (`ls -la`, `ps aux`, `df -h`) that carry no drawing characters at all.

## Proposed rule

Thresholds below are starting points to tune on a corpus, not values to defend by argument.

```
per block (segmented at blank lines):
  pan   if  >=2 stable gutters | >=30% box-drawing lines | aligned pipes
  wrap  if  every line is either shorter than the viewport OR was soft-wrapped by the pty
  pan   otherwise                                        <- default
```

The second clause uses evidence from the producer rather than from line shape: a line the pty
soft-wrapped is one the program emitted **without choosing its own break** — which is what prose does
and what a table (one newline per row) does not.

## Also in scope, and button-free: fit-by-shrink

For a block only mildly too wide (roughly 1-2x viewport), scale that block's font down to fit instead
of demanding a horizontal drag. Monospace plus a uniform scale preserves column alignment exactly, so
nothing is lost. Below a minimum readable size it falls back to pan. This handles the most common
real irritation — a table that overflows slightly yet still forces a sideways gesture.

## The hazard that matters more than accuracy

**Classification flipping between polls.** Content is rewritten every 1.5 s. If block boundaries or
verdicts change between ticks the layout jumps — which damages the very smoothness this feature
exists to improve, and reads as a bug rather than a heuristic miss. A block the operator is mid-drag
could re-wrap under their finger.

Design in from the start, not as a later fix:

- Anchor blocks to blank lines — far more stable than content-derived boundaries.
- Hysteresis: leaving the structured classification requires stronger evidence than entering it.
- Skip re-classification when a block's content hash is unchanged.
- **Classification depends only on content, never on viewport width.** Rotating the phone or raising
  the keyboard must not re-classify anything; width may only affect how a decision is applied.

## Files

**Create**
- `web/src/block-classify.ts` — segmentation + signals + verdict, pure and independently testable.
- `web/test/block-classify.test.ts`.

**Modify**
- `web/src/terminal-render.ts` (phase 2) — apply the per-block verdict.
- `web/src/styles.css` — per-block wrap versus pan containers. A structured block needs its own
  `overflow-x` scroller; note collie's recorded CSS trap (`ansi-output.tsx:76-81`): `overflow-x:auto`
  forces `overflow-y` to compute to `auto`, and a flex item with non-visible overflow may shrink
  below its content height, stealing vertical scroll from the outer container.
- `src/web/screen.rs` — switch the live read to unwrapped once wrapping exists (phase 3 gates this).

**Factor out**
- The distance-from-bottom scroll anchor. Two implementations already exist with different thresholds
  — `HISTORY_SCROLL_THRESHOLD = 4` for the screen view and, in PR #5, `ACTIVITY_STICK_THRESHOLD = 24`
  for its Log viewport. Extract one helper here so a future merge adopts it rather than inventing a
  third.

## Validation

- Classifier unit tests over the corpus, reported as a confusion matrix. The number that matters is
  **structured-wrapped-as-prose**, which must be zero; prose-panned is tolerated by design.
- Jitter test: feed successive polls of a growing buffer and assert verdicts do not oscillate.
- Width-independence test: same content at several viewport widths yields identical verdicts.
- R1-R20 still hold — wrapping is new behaviour and needs its own rules written into
  `docs/specs/terminal-detail.md`, per repo convention that behaviour changes land with spec rules.
- Manual check on a real phone. This cannot be automated, and the mobile-first structural-avoidance
  learning already in `docs/history/learnings/` applies.

## Risks and rollback

- **Tuning may not converge.** If the miss rate is not trustworthy, the only fallback left is a
  manual toggle — which the operator ruled out. Reaching that point means reopening the decision with
  them, not shipping a mechanism they cannot trust.
- **Alt-screen panes expose no pty soft-wrap signal** — only `visible`, a grid the TUI hard-wrapped
  itself — so the second clause is unavailable there and the verdict rests on gutters and
  box-drawing alone. Measured numbers suggest that still works (prose blocks wrap, the TUI frame
  pans), but breaks land on the TUI's own column boundary rather than cleanly. No mechanism fixes
  this; it is a property of the source.
- Rollback: the classifier is one module and one call site. Forcing its verdict to "pan" everywhere
  restores phase 2 behaviour exactly.

# Terminal renderer swap — xterm.js → SGR parser + DOM, with automatic line wrap

**Status: all four phases complete and browser-verified 2026-08-11; deployed to the dev service. Real-device checks outstanding.**

| # | Phase | State |
|---|---|---|
| 1 | SGR parser + StyledLine model | done — xterm equivalence 7/7 fixtures |
| 2 | Vanilla DOM renderer swap | done — R1-R20 checklist not yet run against the live app |
| 3 | `RecentUnwrapped` read source | done |
| 4 | Block classifier + wrap policy | done — R21-R28 written and rendering verified |

Totals: 224 web tests, 347 Rust tests, `tsc`, clippy, fmt and rename-contract green. Bundle JS
83.44 kB -> **11.98 kB** gzipped.

**Verified in a real browser 2026-08-11** (chromium via playwright, 390px viewport, real pane
captures rendered through the app's own modules and stylesheet on the vite dev server). Harness:
`.bee/spikes/terminal-render-bench/visual-check.mjs` and `side-by-side.mjs`.

Measured and seen: prose wraps and fits, laid-out blocks occupy exactly as many rows as they have
lines (nothing broken), a wide table scrolls on its own, the page never pans sideways, blank lines
keep a line's height, blocks sit flush, colours/bold/italic survive, the URL is a real anchor, and
copied text round-trips. All green across the synthetic screen and all seven captures.

**Looking found one real defect the unit tests could not.** Blocks were rendering a full line apart.
The newline separators measured 0 high, but a zero-height *inline* between two block boxes sits in an
anonymous block whose line box keeps a strut from the screen's own font — so the span measured 0
while still pushing every block apart. Fixed by making the separator a zero-height block with hidden
overflow. Two other "failures" were bugs in the check script, not the product: it compared rendered
text against raw ANSI bytes (escapes must not survive), and expected a narrow drawn box to overflow.

**Vietnamese rendering: found, diagnosed, fixed.** Diacritics were displaced ("nêú" for "nếu"). Not
the parser and not the renderer — the capture is entirely precomposed, xterm displaced them
identically, and a bare `<pre>` with the same font stack did too while serif was correct. Testing each
installed monospace family isolated **DejaVu Sans Mono**, which lacks precomposed Vietnamese glyphs
and is what the generic `monospace` keyword resolves to on many Linux systems. Fixed by naming
Vietnamese-capable families ahead of that keyword in `--font-mono` and pointing `.term-screen` at the
variable rather than a second copy of the stack. Zero bytes added; re-verified correct.

**Still unverified:** anything needing real touch or a real device — the per-block horizontal scroll's
feel, pinch-to-zoom, the on-screen keyboard inset, and the R1-R20 checklist run against the live app
rather than against the renderer in isolation.
**Backlog:** PBI-060 (`docs/backlog.md`).
**Supersedes as the execution plan for:** `plans/260728-1135-terminal-ansi-render-lightweight/plan.md`
(that file stays the decision record — all 7 open decisions and their evidence live there; this file
is how the accepted ones get built).

## Why

The live terminal view cannot wrap long lines and never has: `cols` is derived from the longest
content line, not the viewport (`web/src/views/terminal.ts:290-298`), CSS pins natural width
(`web/src/styles.css:1156-1170`), and the spec records it as intent
(`docs/specs/terminal-detail.md:58`). Wrapping under xterm.js means one wrap policy for the entire
buffer, which breaks tables along with prose — xterm exposes no per-line wrap control. Wrapping some
content but not tables is therefore impossible without leaving the fixed-grid model.

## Operator's acceptance criteria (stated 2026-08-11)

1. Colours preserved.
2. No existing capability lost.
3. Line wrap added.
4. Tables not broken.

Plus, locked the same day: the wrap decision is **fully automatic with no UI control of any kind**,
and errs toward pan when unsure.

## Phases

| # | Phase | Depends on | Independently shippable |
|---|---|---|---|
| 1 | [SGR parser + StyledLine model](phase-01-sgr-parser-and-styled-line-model.md) | — | yes (pure module, no UI change) |
| 2 | [Vanilla DOM renderer swap](phase-02-vanilla-dom-renderer-swap.md) | 1 | yes (behaviour identical to today) |
| 3 | [Backend `RecentUnwrapped` read source](phase-03-backend-unwrapped-read-source.md) | — | yes (additive, unused until 4) |
| 4 | [Block classifier + wrap policy](phase-04-block-classifier-and-wrap-policy.md) | 2, 3 | yes (the actual feature) |

Phase 3 is independent of 1 and 2 and may land at any time. Phase 2 deliberately ships **zero
visible change** so the renderer swap is verifiable in isolation from the new behaviour — this is
open decision 4 ("migration boundary") in the decision record.

## Acceptance criteria for the whole change

- **Colour fidelity:** for every fixture in the corpus, the DOM render and xterm produce the same
  per-character foreground/background/weight/style. Checked mechanically, not by eye.
- **No capability lost:** every rule R1-R20 in `docs/specs/terminal-detail.md` still holds, plus the
  Data Dictionary's row 3 promise that URLs render clickable. Phase 2 carries the checklist.
- **Wrap works:** prose blocks wrap to the viewport instead of requiring a horizontal drag.
- **Tables intact:** no block classified as structured is ever wrapped.
- **Performance:** the recurring 1.5 s repaint stays within the measured budget (~250-460 spans for
  the 80-line live poll — `plans/reports/terminal-render-swap-dom-cost-measurement-260811-1219-report.md`).
- Full verify chain green: `cargo test && cargo fmt --all --check && cargo clippy --all-targets -D
  warnings && bash tests/rename_contract.sh && cd web && npm run bundle && npm run test -- --run`.

## Stop conditions

- **Phase 1:** if colour equivalence against xterm cannot be reached on the corpus, the swap fails
  criterion 1 — stop, do not proceed to phase 2.
- **Phase 2:** if any R1-R20 rule cannot be preserved, stop and revert; the swap fails criterion 2.
- **Phase 4:** if classifier tuning cannot reach a miss rate the operator trusts, the only fallback
  left is a manual toggle — which the operator has ruled out. Reaching this point means reopening
  the decision with them, not shipping a mechanism they cannot trust.

## Explicitly out of scope

- **PR #5 (Log view / transcript tail)** — orthogonal, solves data completeness rather than
  rendering, and carries its own unresolved P1s. Decided separately. Note it introduces a second
  `<pre>` viewport; phase 4 should factor the scroll-anchor helper so a future merge has one
  implementation to adopt rather than a third to invent.
- **PBI-047** (markdown `[text](url)` links) — stays open; phase 2 only restores parity with today's
  bare-URL linkifying.
- **PBI-057** (gateway-side scrollback buffer) — independent; composes cleanly if both land.
- Pinch/zoom behaviour — already renderer-independent (proven 2026-08-07); phase 2 preserves it
  unchanged rather than redesigning it.

## Known risks carried into execution

| Risk | Where handled |
|---|---|
| Wide glyph (CJK/emoji) advance width under DOM font metrics vs xterm's grid | Phase 1 |
| Linkify must be hand-rolled — collie has no link detection to port | Phase 2 |
| Classifier flipping between polls makes the layout jump | Phase 4 |
| Alt-screen panes expose no pty soft-wrap signal | Phase 4 |
| No Codex/Agy fixture exists — none were running when the corpus was captured | Phase 1 |

## Open questions

- Which table-shaped output does the operator actually read most (`docker ps`? `kubectl`? test
  runners?) — needed to tune phase 4's thresholds on representative data.
- Can any fixture be committed? Real captures contain live work content; the corpus needs either a
  synthesised clean set or a hand-reviewed one before it enters the repo.
- Alt-screen Claude panes can only ever re-wrap already-wrapped text. Accept that, or treat Claude
  panes as the Log view's job?

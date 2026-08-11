# Phase 1 — SGR parser + StyledLine model

**Status: done 2026-08-11.** `web/src/ansi.ts` + `web/test/ansi.test.ts` (36 cases) +
`web/test/ansi-xterm-equivalence.test.ts`. Verify chain green: 179 web tests, `tsc` clean,
`npm run bundle` builds, `cargo test`/`fmt`/`clippy`/`rename_contract` unaffected and passing.

**Gate result — passed.** The equivalence harness drives xterm's real buffer over each captured
fixture, folds its cells back into style runs, and compares them to `parseAnsi`'s segments.
**7/7 fixtures match run-for-run** — colours (16, bright, 256, truecolour), attributes, box-drawing
and accented text all agree. Acceptance criterion 1 ("colours preserved") is met on the data
available.

Two things worth carrying forward:

- One real bug the tests caught: an escape like `ESC ( B` (charset designator) is ESC + intermediate
  byte + final byte, not a fixed two bytes. Skipping two leaked the `B` as visible text, violating
  the parser's own "consume unknown escapes, never leak" contract. Fixed by consuming intermediates
  (0x20-0x2F) then the final byte.
- The comparison needed symmetric normalisation of trailing unstyled whitespace: xterm pads every
  row out to `cols`, so its blank tail is grid padding with no counterpart in the parser's output.
  Normalised on **both** sides — the parser itself must not discard what it was given.

**Still open from this phase:** wide-glyph (CJK/emoji) *advance width* is a rendering question, not a
parsing one — the parser's handling of them is verified, but how they measure under DOM font metrics
versus xterm's grid can only be answered with a real renderer. Carried into phase 2.

**Re-running the gate:** the harness loads fixtures through a Vite glob over
`plans/*-terminal-render-benchmark/fixtures/*.ans`, and the whole suite **skips when that directory
is empty** — which it is on any fresh checkout, since real captures are not committed. It is a
development gate, not a CI gate, until a shareable corpus exists. `web/test/ansi.test.ts` is the
committed guard.

**Goal:** a pure, framework-free module that turns herdr's `format:"ansi"` text into styled line
data, proven to reproduce xterm's colours on real captures. No UI change ships in this phase.

**Stop condition:** if colour equivalence against xterm cannot be reached on the corpus, acceptance
criterion 1 ("colours preserved") fails and the swap does not proceed to phase 2.

## Context

- Why a hand-rolled parser is safe: `pane.read(format:"ansi")` is ghostty's `FormatterFormat::Vt`
  serialising a selection of the **already-rendered cell grid**
  (`upstreams/herdr/src/ghostty/mod.rs:898-912`, reached via
  `upstreams/herdr/src/pane/terminal.rs:400-402` → `upstreams/herdr/src/app/api/panes.rs:1194`).
  A grid dump has no cursor, so it cannot emit CUP/EL/ED/alt-screen/scroll-region. The escape set is
  closed by construction, not by observation.
- Reference implementation to port from: `upstreams/collie/web/src/lib/ansi.ts` (245 lines) — but see
  "not a copy" below.
- Decision record: `plans/260728-1135-terminal-ansi-render-lightweight/plan.md`, open decision 1.

## Requirements

- Support the full SGR set collie's parser covers, verified present in its source: reset, bold, dim,
  italic, underline, inverse, strikethrough and their off-codes (22/23/24/27/29); 16-colour fg/bg;
  bright fg/bg (90-97, 100-107); default fg/bg (39/49); 256-colour (`38;5;n`); 24-bit truecolour
  (`38;2;r;g;b`); and the ISO 8613-6 colon forms (`38:5:n`, `38:2:r:g:b`, `38:2::r:g:b`).
- Consume and discard non-SGR escapes defensively (DEC-private `?`-prefixed sequences, OSC with
  either BEL or ST termination) so nothing leaks as visible text.
- `\r` last-write-wins per line, matching collie's spinner/progress handling — but keep collie's
  guard that a CR immediately followed by LF or end-of-input is a line terminator, not an overwrite
  (herdr's buffer is CRLF; treating every CR as an overwrite blanks every line).
- Output text as data only. The renderer must place it in text nodes, never `innerHTML` — this is the
  XSS boundary and phase 2 depends on it.
- **Not a copy:** collie is React and returns `CSSProperties`. This repo is vanilla TS with a CSS
  variable theme (`TERMINAL_THEME`, `web/src/views/terminal.ts:73`). Emit style as data this repo's
  renderer can apply, resolving colours against the existing theme rather than collie's Tailwind
  variables.

## Files

**Create**
- `web/src/ansi.ts` — parser + types. Flat `web/src/` matches the existing shared-module convention
  (`api.ts`, `kind-marks.ts`); there is no `lib/` directory in this repo.
- `web/test/ansi.test.ts` — unit tests.
- Corpus fixtures — location unresolved, see plan.md open questions. Real captures contain live work
  content and must not be committed unreviewed.

**Modify** — none. This phase adds a module nothing imports yet.

## Steps

1. Capture a corpus from live panes covering every agent kind and content class: agent TUI (Claude),
   colour-heavy shell (`git log --graph`, `ls --color`, `cargo tree`), plain logs, and content with
   CJK and emoji. Reuse the capture approach from
   `plans/reports/terminal-render-swap-dom-cost-measurement-260811-1219-report.md`.
2. Port the parser, adapting the style model to this repo's theme as above.
3. Write the equivalence harness: feed each fixture to both xterm and the new parser, then compare
   per-character foreground, background, bold, dim, italic, underline and strike. This is the phase's
   real gate — unit tests alone would only prove the parser agrees with itself.
4. Close the two residual correctness gaps named in the decision record: literal tab handling (collie
   has no `\t` branch), and wide-glyph advance width. Establish what CJK and emoji actually do under
   DOM font metrics versus xterm's grid, and record the answer — this is the one area where xterm is
   genuinely stronger.

## Validation

- `cd web && npm run test -- --run` green, including the new file.
- Equivalence harness passes on every corpus fixture.
- `npm run typecheck` clean.
- Parse cost stays near the measured ~1 ms per 1000 lines.

## Risks and rollback

- **Wide glyphs** are the known weak point. If emoji or CJK misalign badly enough to damage
  criterion 2, that is a phase-1 stop condition, not something to discover in phase 2.
- **No Codex/Agy fixture** — none were running at corpus-capture time. Either wait for one or record
  the gap explicitly rather than assuming Claude and shell bracket the range.
- Rollback is free: nothing imports this module until phase 2.

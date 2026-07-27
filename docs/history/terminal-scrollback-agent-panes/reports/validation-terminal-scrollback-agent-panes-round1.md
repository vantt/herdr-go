# Validation report — terminal-scrollback-agent-panes, round 1

**Verdict: NOT READY.** Full plan-checker + cold-pickup cell review, dispatched
against the round-1 `plan.md` and its 3 cells, with direct file reads and
command runs (not model recall).

## BLOCKER

1. Cell 1's `files` list omitted `src/web/create.rs`, which contains a third
   `impl Herdr` (`RecordingHerdr`, test-only stub, `read_pane` at `:337`) that
   fails to compile once the trait signature changes. Neither this file nor
   the one production call site (`src/web/screen.rs:27`) was in cell 1's
   scope — the latter is cell 2's file, creating an implicit cross-cell
   compile dependency.
2. D11 (spec updates: `docs/specs/terminal-detail.md`, `web-api.md`,
   `herdr-port.md`) had no owning cell — none of the 3 cells' actions,
   `must_haves`, or `files` mention any of the three spec files.
3. Cell 3's verify command (`cd web && npm run test -- --run terminal`)
   failed outright: `sh: 1: vitest: not found` — dependencies were not
   installed in this fresh worktree. (Fixed for round 2: `npm install` has
   been run in `web/`.)

## CRITICAL

4. Cell 3's verify, even once runnable, proves none of its `must_haves`:
   `web/test/terminal.test.ts` (88 lines) exercises only
   `computeKeyboardInset`/`stripAnsiLen`/`terminalHead` — no test touches
   `renderTerminal`'s DOM behavior at all. A green run is compatible with
   zero implementation of the actual feature.
5. `FakeHerdr` (`src/herdr/fake.rs:372-382`) stores one `(text, revision)`
   per pane with no `lines`-aware slicing, so it cannot express "recent
   richer than visible" for any test — cell 1's own truths were untestable
   as written.
6. The core mechanism had a real logic bug: comparing `recent(1000)` against
   an *approximated* visible (`recent(<small lines>)`) causes false-positive
   escalation for any genuinely short primary-screen pane (identical short
   text from both reads → escalates → sends raw PageUp/Ctrl+End into a plain
   shell for nothing), contradicting D3's own "no escalation needed" intent
   for that case.

## WARNING

7. `terminal.ts:168`'s `rows = clamp(lines.length+1, 4, 400)` render clamp,
   combined with `scrollback: 0` (`:148`), silently discards content beyond
   ~400 rows — below D2's 1000-line herdr cap — and was unowned by any cell.
   Round-1 plan.md also mischaracterized `applyScreen` as "prepending";
   it's a wholesale replace (`:157-172`), no prepend path exists.
8. Round-1 plan.md's citation of `web/src/styles.css:748-750` for the scroll
   container was wrong — the actual rule is `.term-viewport { overflow: auto
   }` at `:930-939`.

## Answered during validating (carried into round 2, not re-derived)

**A — scroll-position preservation:** under append-only growth within both
the 400-row clamp and the 1000-line cap, already-rendered rows keep their
pixel Y by accident (content is written top-down, growth only appends
below), so no explicit preservation is needed for that common case. It
breaks past either ceiling (oldest visible rows age out of the window under
a now-stale `scrollTop`) or on a content shrink (`term.resize()` runs before
`term.reset()` at `:169-170`, clamping `scrollTop` irrecoverably).

**B — poll/history race:** the existing 1500ms poll loop
(`terminal.ts:174-187`, interval at `:309`) has zero overlap guard — only
`if (disposed) return` and a `text === lastText` dedupe, no in-flight flag,
no `AbortController`, no revision-ordering check. Cell 3 must build
sequencing from scratch.

## Disposition

All 8 findings addressed in round 2 of `plan.md` and the revised cells — see
that file's "Revision Note (round 2)" section for the point-by-point fix.

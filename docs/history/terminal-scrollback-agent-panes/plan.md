---
artifact_contract: bee-plan/v1
mode: standard
approved_gate2: 2026-07-27 (round 3, superseding rounds 1-2 after NOT READY validating verdicts)
---

# Terminal scrollback for agent panes — Plan

## Revision Note (round 2)

Round 1 of this plan was validated and came back **NOT READY** with 3
BLOCKERs, 3 CRITICALs, and 2 WARNINGs, all backed by real file reads and
command runs (not guesses) — see
`docs/history/terminal-scrollback-agent-panes/reports/validation-terminal-scrollback-agent-panes-round1.md`.
This is not a silent edit of a frozen plan (per the plan-freeze rule) — it is
a genuine return-to-planning after a failed reality gate, so this revision
supersedes round 1's Discovery/Approach/Risk Map/Cells sections outright.
Nothing here shrinks a locked CONTEXT.md decision; D3 and D9 are refined with
newly-found evidence, per the "verified decisions reverse only on new
evidence" rule — the refinement is cited inline below, not silently swapped.

## Revision Note (round 3)

Round 2 was validated again and came back **NOT READY** with 2 new findings
(round 1's 8 findings were confirmed fixed): the round-2 mechanism required
(a) requesting herdr's real `source: "visible"`, but nothing in the plan
actually added a `source` selector to `Herdr::read_pane` — it still
hardcoded `"recent"` (`src/herdr/socket.rs:466`) — and (b) an "at-capacity"
gate that needs the pane's viewport row count, which the gateway's own wire
types do not carry (`src/herdr/wire.rs:132-143`'s `Pane` has no `scroll`
field) and which cross-referencing herdr's own snapshot for was exactly what
D3 already rejected as unreliable/stale.

Fix: (a) add an explicit `source` parameter to `Herdr::read_pane` so
`"visible"` can actually be requested — a real, evidence-grounded
mechanism fix, kept. (b) **Drop the at-capacity gate entirely** rather than
invent a new way to obtain viewport dimensions the gateway doesn't have.
Reasoning: the problem the gate solved — a short primary-screen pane
harmlessly escalating once, sending an unbound PageUp/Ctrl+End into a plain
shell that generically ignores unbound escape sequences — is a low-cost
wasted round trip, not a scrollback-correctness bug (round 1's own risk map
already labeled the double-read cost LOW and accepted it before round 2
over-corrected it into a gate that needed data that doesn't exist). This is
not a scope reduction of any locked decision: D3 still requires comparing
actual read results, never the `scroll.max_offset_from_bottom` field or the
agent name — dropping the capacity pre-check keeps that requirement intact,
it just removes a mechanism-level embellishment that introduced two new
blockers to solve a low-severity problem already accepted once.

## Mode Gate Record

Unchanged from round 1: 3 flags (external systems, public contracts,
multi-domain) → **standard**. No hard-gate flag.

## Discovery

L1, revised with round-1 validating's findings:

- Confirmed the existing live-view call path: `GET /api/panes/:pane/screen`
  (`src/web/screen.rs::read_screen`) → `Herdr::read_pane(&self, pane_id)` →
  `SocketHerdr` sends `{"source":"recent","format":"ansi"}` with no `lines`
  key (`src/herdr/socket.rs:466`, D2's fix site). The trait method takes only
  `pane_id` today.
- **New: `Herdr::read_pane`'s signature change has a wider blast radius than
  round 1 accounted for.** There are **two** implementors, not one — the
  production `SocketHerdr`/test `FakeHerdr` pair (`src/herdr/{socket,fake}.rs`)
  plus a **third**, test-only `RecordingHerdr` stub inside
  `src/web/create.rs:323-337` (used to stub the trait for create-endpoint
  tests). All three must compile against the new signature. Separately, the
  **one production call site**, `src/web/screen.rs:27`, must also be updated
  to pass an explicit `lines` value matching today's behavior — this is a
  compile-fix, not new logic, and is intentionally scoped into cell 1 rather
  than left implicit and picked up accidentally by cell 2.
- Confirmed the frontend does **not** rely on xterm.js's own scrollback at
  all (`web/src/views/terminal.ts:148`, `scrollback: 0`). `applyScreen()`
  (`:157-172`) does a **wholesale replace** each poll: `term.reset()` then
  `term.write(text)` on a grid resized to `rows = clamp(lines.length + 1, 4,
  400)` (`:168`) — **not** a prepend/append of a delta; round 1's plan
  mischaracterized this as "prepending", corrected here. The scroll container
  is `.term-viewport { overflow: auto }` at `web/src/styles.css:930-939`
  (round 1 cited `:748-750`, which is a comment inside an unrelated
  `overflow: hidden` rule — wrong citation, corrected here).
- **New: the render clamp at `:168` silently discards history beyond ~400
  rows**, independent of and below D2's 1000-line herdr-side cap — a pane
  with more than 400 lines of returned text loses its oldest ~600+ lines to
  this clamp before the operator ever sees them, regardless of what the
  backend returns. Unowned by any round-1 cell; owned by cell 3 in this
  revision.
- **Scroll-position risk, resolved (validating Answer A):** under append-only
  growth *within* both the 400-row clamp and herdr's 1000-line cap,
  already-rendered rows keep their pixel Y (content is written top-down,
  growth only ever appends below what's already shown), so `viewport.scrollTop`
  stays correct **by accident** — no explicit preservation logic is needed
  for that common case. It breaks only in two situations: (a) total content
  exceeds the 400-row clamp or the 1000-line herdr cap, so the *oldest*
  visible rows silently age out of the window (the clamp/cap slide their
  view forward, shifting everything the operator was reading upward under a
  now-stale `scrollTop`); (b) a content *shrink* — `term.resize()` runs
  *before* `term.reset()` (`:169-170`), so a smaller row count clamps
  `scrollTop` irrecoverably. Cell 3 must handle (a) and (b) explicitly rather
  than assume append-only growth is the only case.
- **Poll/history race, resolved (validating Answer B):** the existing
  1500ms poll loop (`terminal.ts:174-187`, interval at `:309`) has **zero**
  existing overlap guard — only `if (disposed) return` and a `text ===
  lastText` dedupe (`:158`), no in-flight flag, no `AbortController`, no
  revision-ordering check (`screen.revision` is fetched and ignored). Cell 3
  must build this sequencing from scratch; there is nothing to reuse.

## Approach

**Mechanism (round 3 — drops round 2's at-capacity gate, per the Revision
Note above; the `visible`-vs-`recent` comparison itself is unchanged and
still fixes round 1's approximation bug):**

1. Read **herdr's actual `visible` source** (`source: "visible"`, via the
   new `source` parameter on `Herdr::read_pane` — no need to approximate it
   via `recent` with a small `lines` value; herdr exposes it directly).
2. Read `recent(1000)` (`NativeScrollback`) and compare against the `visible`
   read from step 1.
3. If `recent(1000)` returns more than `visible`, use it — the
   primary-screen case.
4. If they're identical, escalate: send raw PageUp (`\x1b[5~`) via
   `Herdr::send_text`, re-read `visible`, then always send raw Ctrl+End
   (`\x1b[1;5F`) via `send_text` to restore the live bottom before returning
   (`EscapeInjection` — the alt-screen case, D4/D5). For a genuinely short
   primary-screen pane this step 4 fires once, harmlessly (an unbound
   escape sequence a plain shell generically ignores) — accepted as a
   low-cost round trip, not a correctness bug (see Risk Map).

The strategy-selection code carries D10's WHY-comment explaining why
selection compares actual read results (steps 2-4) rather than
`scroll.max_offset_from_bottom` or the pane's agent name.

**For primary-screen panes:** once `NativeScrollback` returns more than
`visible`, the frontend's existing wholesale-replace render path already
displays it (up to the two ceilings above) — no new rendering path, only the
scroll-preservation and 400-row-ceiling handling from Discovery.

**For alt-screen panes:** retrieval requires an explicit action, so the
frontend needs an explicit "load older" trigger (scrolling/swiping past the
top of `.term-viewport`) that calls the history endpoint and re-renders with
the escalated result — a full re-render (per the corrected wholesale-replace
mechanism above), not a prepend.

**Trait/endpoint shape (unchanged from round 1 except where noted):**

- `Herdr::read_pane` gains an explicit `lines: usize` argument **and** a
  `source` argument (a small enum, e.g. `ReadSource::{Visible, Recent}` in
  `src/herdr/mod.rs`, matching herdr's own `source` vocabulary) — round 3
  fix: round 2's plan referenced requesting `source: "visible"` without ever
  adding a way to actually select it; `SocketHerdr` currently hardcodes
  `"source":"recent"` unconditionally (`src/herdr/socket.rs:466`). `lines` is
  ignored by herdr when `source` is `Visible` (matches herdr's own behavior),
  still required as a parameter for the `Recent` case. All three
  implementors — `SocketHerdr`, `FakeHerdr`, and the test-only
  `RecordingHerdr` (`src/web/create.rs:323-337`) — and the one production
  call site (`src/web/screen.rs:27`) must be updated to compile; cell 1 owns
  all of these as a minimal signature-only fix (existing callers pass
  `ReadSource::Recent` to match today's behavior exactly), cell 2 then adds
  new logic on top of the already-updated `src/web/screen.rs`.
- `Herdr::send_text(&self, pane_id: &str, bytes: &str) -> Result<()>` calling
  herdr's `pane.send_text` raw-byte channel (D5), implemented on
  `SocketHerdr` and `FakeHerdr` (not needed on `RecordingHerdr`, which only
  stubs `read_pane` for create-endpoint tests unrelated to this feature).
- **New in this revision:** `FakeHerdr` needs a history-aware shape — a
  longer stored per-pane history string it slices by the requested `lines`
  and by a fixed viewport-row count for `visible` — so cell 1's own
  `NativeScrollback`/`EscapeInjection` truths are actually testable rather
  than merely asserted (validating CRITICAL 5).
- Extend `GET /api/panes/:pane/screen` with an optional `?history=1` query
  param: absent → unchanged live-view behavior; present → routes through the
  revised `PaneScroller` mechanism instead. Same response shape
  (`ScreenBody { text, revision }`), no new endpoint.

**Spec updates (D11) — ownership clarified (BLOCKER 2 from validating):**
`docs/specs/terminal-detail.md`, `web-api.md`, and `herdr-port.md` are **not**
covered by a dedicated code cell in this slice. Per this repo's established
convention, spec sync for a behavior-changing feature happens through the
`bee-scribing` chain step after cell execution, not as its own planning cell
— stated here explicitly so a future reader does not reintroduce a "missing
cell" for D11.

## Risk Map

| Component | Risk | Proof needed |
|---|---|---|
| Scroll-position preservation past the 400-row/1000-line ceilings, and on content shrink | MEDIUM | Cell 3 must implement explicit handling for both cases identified in Discovery — not assume append-only growth is the only case. Verify via a DOM-level test asserting `scrollTop` behavior across a simulated ceiling-crossing and a simulated shrink. |
| `EscapeInjection`'s restore-to-bottom vs. the next live poll tick | MEDIUM | Cell 3 must add an in-flight guard (the existing poll loop has none, confirmed) so a live poll tick cannot interleave with an in-flight history request's read-then-restore round trip. Verify via a test that fires both concurrently and asserts no torn/interleaved frame. |
| A short primary-screen pane (Recent not richer than Visible, nothing more to show) escalates once to `EscapeInjection`, sending an unbound PageUp/Ctrl+End into whatever foreground process holds the pane (a plain shell, or any other stdin reader not yet exercised by this feature's evidence) | LOW | **Assumption, not proven** (round 4 correction): this feature's live evidence for how a foreground process reacts to these exact escape bytes covers only Claude Code (an alt-screen pane, `\x1b[5~` confirmed to trigger its own scroll — see D4). No primary-screen/shell pane was ever live-tested with these bytes. The claim that terminals/shells generically ignore an unbound escape sequence harmlessly is a reasonable but unverified assumption, accepted as LOW severity because escalation is operator-gestured (one "load older" swipe) and never sends a trailing Enter — not because it has been proven harmless. Round 2's attempt to prevent this via an at-capacity gate required viewport-dimension data the gateway doesn't have (see Revision Note round 3) and was reverted; if this assumption is ever contradicted by a live report of stray input appearing in a shell, revisit with a real fallback (e.g. skip escalation for panes not already known to be alt-screen from prior observation), not by reinstating the at-capacity gate. |
| `send_text`/`FakeHerdr` history shape/`source` param are new surface | LOW | Standard trait-method additions + fake extension, unit tests in cell 1. |

## Cells (current slice)

1. `terminal-scrollback-agent-panes-1` — backend port: `Herdr::read_pane`
   gains explicit `lines` and `source` (`ReadSource::{Visible,Recent}`)
   arguments across all three implementors (`SocketHerdr`, `FakeHerdr`,
   `RecordingHerdr` in `src/web/create.rs`) and the one production call site
   (`src/web/screen.rs`, compile-fix only, passing `ReadSource::Recent` to
   match today's default explicitly); `Herdr::send_text` added + implemented
   on `SocketHerdr`/`FakeHerdr`; `FakeHerdr` gains a history-aware per-pane
   shape sliceable by `lines` for `Recent`, plus a separate fixed slice for
   `Visible`; new `PaneScroller` port (own file, e.g.
   `src/herdr/pane_scroller.rs`) implementing the compare-and-escalate
   mechanism from Approach (visible vs recent(1000), no capacity gate), with
   D10's WHY-comment. Unit tests against the extended `FakeHerdr` proving:
   short-pane escalates harmlessly and returns correct (empty-extra) result,
   primary-screen pane returns `NativeScrollback`, alt-screen pane returns
   `EscapeInjection` result (including the always-restore-to-bottom
   behavior).
2. `terminal-scrollback-agent-panes-2` — backend wiring: extend
   `GET /api/panes/:pane/screen` (`src/web/screen.rs`, building on cell 1's
   compile-fix) with `?history=1` routing through `PaneScroller`; integration
   test for both the unchanged default path and the history path.
3. `terminal-scrollback-agent-panes-3` — web: a "load older" trigger when the
   operator scrolls/swipes past the top of `.term-viewport`, calling the
   `?history=1` endpoint and doing a full wholesale re-render with the
   result (not a prepend — `applyScreen`'s actual design); raising or
   otherwise accounting for the 400-row render clamp so "load older" can
   show more than 400 rows; explicit scroll-position handling for the
   ceiling-crossing and shrink cases from the risk map; an in-flight guard
   sequencing the history-load request against the existing 1500ms poll loop
   so they never interleave. Must include a real DOM-level test exercising
   `renderTerminal`'s actual behavior for these paths — the pre-existing
   `web/test/terminal.test.ts` only covers helper functions
   (`computeKeyboardInset`/`stripAnsiLen`/`terminalHead`), which round 1
   wrongly treated as sufficient coverage (validating CRITICAL 4/BLOCKER 3).

## Verify

`cargo test --quiet && cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && bash tests/rename_contract.sh && cd web && npm run bundle && npm run test -- --run` (this repo's recorded `commands.verify`). Note: this worktree needed `npm install` in `web/` once before this command's web half could run (fresh `git worktree add` does not carry installed dependencies) — already done, not a per-cell step.

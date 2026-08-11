# Phase 3 — `ReadSource::RecentUnwrapped`

**Status: done 2026-08-11.** `ReadSource::RecentUnwrapped` (wire string `recent_unwrapped`), a
`takes_line_count()` predicate replacing the `== Recent` check that gated the `lines` parameter, and
fake support. 346 Rust tests green, fmt and clippy clean.

**Both reads switched (2026-08-11, after a live table defect).** The live poll went first; the
history path (`PaneScroller`) was left on `Recent` and that was wrong. A controlled experiment —
print a table wider than the pane, read it both ways — showed the wrapped source returns each row
split at the wrap column with the pieces at unrelated indents, while the unwrapped source returns
every row intact and in column. Leaving history on the wrapped source meant the same table looked
whole scrolling forward and shattered scrolling back. `history_reads_the_unwrapped_scrollback`
guards it, and was checked to actually fail when the source is reverted.

**Originally deliberately not done:** no read was switched over at first. The new source is added and unused. Switching
the live poll before wrapping exists would make things *worse* — unwrapped lines are longer, so under
today's pan-only rendering the operator would have to drag further. Phase 4 gates that switch.

**Fake design note:** `recent_unwrapped` is `Option<String>` defaulting to `None`, meaning "same as
`recent`". That is the honest default — herdr returns identical text for both sources whenever no
line was long enough to wrap — and it matters for test integrity: a fake that always diverged would
let a caller pass while asking for the wrong source. `seed_wrapped_pane` sets the two independently
for the tests that need a real divergence, and there is a test for each direction.

**Goal:** let the gateway ask herdr for scrollback with the pty's soft wraps rejoined. Small,
additive, independent of phases 1 and 2 — but phase 4 does not work properly without it.

## Why this is not optional

Two separate reasons, both verified live on 2026-08-11:

1. **Wrap quality.** The pty has already hard-wrapped its output. Soft-wrapping that again yields
   ragged double-wrapping at the pty's own column boundary. Measured: a 1587-character logical line
   comes back from `--source recent` as **7 physical lines of ~231 characters** (the pane width), and
   from `--source recent-unwrapped` as **one 1587-character line**.
2. **Detection quality** — the less obvious one. On wrapped data a prose paragraph arrives as
   fragments that often contain no sentence punctuation at all, and a table wider than the pane has
   its column alignment destroyed (row 1 fragment A, row 1 fragment B, row 2 fragment A…). Both of
   phase 4's strongest signals degrade. On unwrapped data prose becomes one long line rich in
   punctuation, and wide table rows stay aligned with each other.

Note: when no line exceeds the pane width the two sources return byte-identical output. This is not a
different format, only a rejoin of what the pty broke.

## Upstream support already exists

- `recent_unwrapped_ansi()` preserves SGR — it is the same `ghostty_recent_ansi` with the unwrap flag
  set (`upstreams/herdr/src/pane/terminal.rs:400-402`).
- Served over the socket for `pane.read` (`upstreams/herdr/src/app/api/panes.rs:1194`).
- Wire value `recent_unwrapped`; the CLI accepts both spellings
  (`upstreams/herdr/src/cli.rs:1038`).
- Upstream tests cover soft-wrap joining and CJK
  (`upstreams/herdr/src/pane/terminal.rs:4141-4160`).

Locally, `ReadSource` has only `Visible` and `Recent` (`src/herdr/mod.rs:59-60`), wired at
`src/herdr/socket.rs:469-473`.

## Files

**Modify**
- `src/herdr/mod.rs` — add the enum variant and its wire string.
- `src/herdr/socket.rs` — the `lines` parameter is currently sent only for `Recent`
  (`socket.rs:472`); the new variant needs the same treatment, capped at herdr's 1000-line limit.
- `src/herdr/fake.rs` — the fake must answer the new variant, and its unwrapped answer must actually
  differ from `Recent` for a wrapped fixture, or tests will pass against a fake that proves nothing.
- `src/web/screen.rs` — choose the source. See "decision to make" below.
- `src/herdr/pane_scroller.rs` — only if history reads should use it too.

## Decision to make during this phase

Which reads switch to unwrapped, and whether the frontend chooses:

- The live poll currently asks for `Recent, 80` (`src/web/screen.rs:58`).
- `PaneScroller` compares `Visible` against `Recent(1000)` and returns the richer
  (`src/herdr/pane_scroller.rs:95-99`). Alt-screen panes have no scrollback and fall back to
  `Visible`, which cannot be unwrapped at all.

The convenient part: unwrapping only applies to primary-screen panes (shell, logs) — exactly where
prose wrapping is wanted — while alt-screen panes keep grid content that must **not** be wrapped.
The capability lands where it is needed without a per-pane switch.

Keep the response shape unchanged (`ScreenBody { text, revision }`) so this phase stays additive.

## Validation

- Rust unit tests for the wire mapping and the `lines` parameter.
- A fake-backed test asserting unwrapped and wrapped reads differ for a fixture containing a
  soft-wrapped line.
- `cargo test && cargo fmt --all --check && cargo clippy --all-targets -D warnings`.
- Live check against the running herdr, reproducing the 1587-character result above.

## Risks and rollback

- Rollback is trivial while nothing consumes the new source.
- Behaviour risk if the live poll is switched before phase 4 lands: unwrapped lines are **longer**,
  so under today's pan-only rendering the operator would have to drag further. Do not switch the live
  read until phase 4 can wrap, or gate it behind phase 4.

//! `PaneScroller` — a hexagonal port over [`Herdr`] (matches this repo's
//! `Herdr`/`Store` one-trait-swappable-adapters convention, CONTEXT.md D9)
//! answering "give me this pane's older content" without the caller needing
//! to know which of the two mechanisms actually served it:
//!
//! - `NativeScrollback` — herdr's own `recent` source (free, CONTEXT.md D2).
//! - `EscapeInjection` — replay the agent's own scroll keybinding via raw
//!   bytes for an alt-screen pane that holds no native scrollback at all
//!   (CONTEXT.md D4/D5).

use std::time::Duration;

use super::{Herdr, ReadSource, Result, ScreenRead};

/// Raw PageUp — the VT escape sequence a live Claude Code pane was verified
/// to interpret as "scroll its own transcript up" (CONTEXT.md D4).
pub(crate) const PAGE_UP: &str = "\x1b[5~";
/// Raw Ctrl+End — verified to restore the live bottom view afterward (D4).
pub(crate) const RESTORE_BOTTOM: &str = "\x1b[1;5F";

/// Herdr's own hard cap on a `recent` read (CONTEXT.md D2).
const RECENT_LINES_CAP: usize = 1000;

/// How many times to re-read `Visible` after sending PageUp before giving up
/// and using whatever the last read returned. `send_text` only confirms the
/// bytes reached the pty, not that the agent's TUI has redrawn in response --
/// live reproduction against Claude Code 2.1.220/herdr 0.7.4 showed an
/// immediate read right after `send_text` reliably races ahead of the
/// redraw and returns the stale, pre-scroll screen (root cause of "load
/// older" appearing to do nothing). A short poll loop closes that race
/// without a single guessed sleep length; a pane that never responds (the
/// harmless no-op case) simply exhausts every attempt.
const ESCALATION_READ_ATTEMPTS: usize = 5;
/// Delay between escalation re-reads (see `ESCALATION_READ_ATTEMPTS`).
const ESCALATION_READ_INTERVAL: Duration = Duration::from_millis(40);

/// Which mechanism actually served a `read_history` result (CONTEXT.md D9).
/// Not part of `read_history`'s public return (the cell's own contract pins
/// that to a bare `ScreenRead` so a caller builds `ScreenBody` from it
/// directly) — used internally, and exposed to tests via
/// `read_history_with_strategy`, so a test can assert which path fired
/// without inferring it from timing or side effects alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollStrategy {
    NativeScrollback,
    EscapeInjection,
}

/// A hexagonal port over [`Herdr`] answering "give me this pane's older
/// content" — the trait's own implementation runs the try-NativeScrollback-
/// then-compare logic internally (D3), so a caller never branches on
/// strategy itself.
pub struct PaneScroller<'a> {
    herdr: &'a dyn Herdr,
}

impl<'a> PaneScroller<'a> {
    pub fn new(herdr: &'a dyn Herdr) -> Self {
        PaneScroller { herdr }
    }

    /// Read `pane_id`'s older content. Always returns the existing
    /// [`ScreenRead`] wire type (`text` + `revision`, never a bare
    /// `Vec<String>`/`String`) so a caller can build a `ScreenBody` straight
    /// off it, matching the existing non-history response shape — and always
    /// carries the read's own `revision` through unchanged, never fabricated.
    pub async fn read_history(&self, pane_id: &str) -> Result<ScreenRead> {
        self.read_history_with_strategy(pane_id)
            .await
            .map(|(read, _strategy)| read)
    }

    /// Same as [`PaneScroller::read_history`], but also names which strategy
    /// served the result — used by this module's own tests to assert
    /// `NativeScrollback` vs. `EscapeInjection` directly instead of inferring
    /// it from `send_text` call logs alone.
    async fn read_history_with_strategy(
        &self,
        pane_id: &str,
    ) -> Result<(ScreenRead, ScrollStrategy)> {
        let visible = self
            .herdr
            .read_pane(pane_id, ReadSource::Visible, 0)
            .await?;
        let recent = self
            .herdr
            .read_pane(pane_id, ReadSource::Recent, RECENT_LINES_CAP)
            .await?;

        // WHY, not WHAT (D10): alt-screen panes run in the terminal's
        // alternate screen buffer, which by VT100/xterm semantics never
        // accumulates scrollback at all (D1) -- so herdr's own `recent` read
        // comes back no richer than `visible` for them, not merely smaller.
        // Selection therefore compares the two ACTUAL reads (richer or not),
        // never `scroll.max_offset_from_bottom`: that field lives only on a
        // separately-fetched, possibly-stale snapshot (the pane.read
        // response itself carries no scroll field at all), and it reads
        // `0`/absent for both a permanently-alt-screen pane AND a genuinely-
        // empty fresh primary-screen pane -- indistinguishable by that field
        // alone (D1/D3). Also never the pane's `agent` name: a plain shell
        // pane carries none, an unknown/future agent would be silently
        // mishandled by a hardcoded list, and an agent's own alt-screen
        // usage can change release to release while its name stays the same
        // (D3). "Not richer than" is deliberately ONE case covering exact
        // equality and any other non-richer outcome identically -- there is
        // no partial-credit branch, and no at-capacity/viewport-dimension
        // gate before escalating (round 3 dropped that: the gateway's wire
        // types carry no viewport data to gate on, and the risk it guarded
        // against -- a short primary-screen pane escalating once, harmlessly
        // -- is accepted below, not solved by inventing missing data).
        if is_richer(&recent, &visible) {
            return Ok((recent, ScrollStrategy::NativeScrollback));
        }

        // Escalate: replay the agent's own scroll keybinding via raw bytes
        // (D4/D5) -- never send_keys/send_input, see Herdr::send_text's own
        // doc for why. For a genuinely short primary-screen pane this fires
        // once, harmlessly: an ASSUMED-but-not-independently-verified no-op
        // (only Claude Code/alt-screen was live-tested with these exact
        // bytes) accepted as a low-cost round trip, not a correctness bug --
        // see plan.md's Risk Map.
        self.herdr.send_text(pane_id, PAGE_UP).await?;
        // Poll for the redraw rather than trusting a single immediate read
        // (see ESCALATION_READ_ATTEMPTS) -- stop as soon as a read is richer
        // than the pre-scroll `visible` baseline, i.e. the redraw landed.
        let mut escalated = self
            .herdr
            .read_pane(pane_id, ReadSource::Visible, 0)
            .await?;
        for _ in 1..ESCALATION_READ_ATTEMPTS {
            if is_richer(&escalated, &visible) {
                break;
            }
            tokio::time::sleep(ESCALATION_READ_INTERVAL).await;
            escalated = self
                .herdr
                .read_pane(pane_id, ReadSource::Visible, 0)
                .await?;
        }
        // ALWAYS restore the live bottom afterward, regardless of what the
        // escalated read returned -- an operator's "load older" swipe must
        // never leave the pane scrolled away from its live tail.
        self.herdr.send_text(pane_id, RESTORE_BOTTOM).await?;

        Ok((escalated, ScrollStrategy::EscapeInjection))
    }
}

/// `recent` is "richer" than `visible` when it strictly contains more text --
/// a length comparison, not equality: D2's live evidence showed a primary-
/// screen pane's `recent` read always tails-match `visible` exactly and adds
/// content before it, so a strict length win is exactly "has more to show".
/// Equal length (even with different bytes) is deliberately NOT richer --
/// see the WHY-comment in `read_history_with_strategy` for why "not richer"
/// must stay one case, not just exact equality.
fn is_richer(recent: &ScreenRead, visible: &ScreenRead) -> bool {
    recent.text.len() > visible.text.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::herdr::fake::FakeHerdr;

    #[tokio::test(start_paused = true)]
    async fn shortpane_escalates_harmlessly_and_returns_carried_revision() {
        // Recent(1000) == Visible (no extra native scrollback) and the pane
        // does not respond to PageUp (escape_reveal: None) -- the harmless
        // no-op case: PaneScroller still escalates (D3's "not richer" branch
        // covers this identically to an alt-screen pane), but the content
        // comes back unchanged, and the revision is the pane's own, carried
        // through, never fabricated.
        let herdr = FakeHerdr::new();
        herdr.seed_scroll_pane("w1:p1", "❯ ", "❯ ", None);

        let scroller = PaneScroller::new(&herdr);
        let (read, strategy) = scroller.read_history_with_strategy("w1:p1").await.unwrap();

        assert_eq!(strategy, ScrollStrategy::EscapeInjection);
        assert_eq!(read.text, "❯ ", "short pane's content is unchanged");
        assert_eq!(
            read.revision, 1,
            "revision is the pane's own, carried through"
        );

        // Escalation still ran the full send_text sequence (PageUp then
        // Ctrl+End) even though it changed nothing -- proving the restore
        // step is unconditional, not skipped when the pane didn't respond.
        assert_eq!(
            herdr.sent_text_log("w1:p1").await,
            vec![PAGE_UP.to_string(), RESTORE_BOTTOM.to_string()]
        );
    }

    #[tokio::test]
    async fn primaryscreen_pane_returns_native_scrollback_without_escalating() {
        // Recent(1000) is strictly richer than Visible -- herdr already
        // holds real scrollback for this pane (D2), so PaneScroller must use
        // it directly and never touch send_text at all.
        let herdr = FakeHerdr::new();
        let history = "line 1\nline 2\nline 3\n❯ ";
        let visible = "line 3\n❯ ";
        herdr.seed_scroll_pane("w1:p1", visible, history, None);

        let scroller = PaneScroller::new(&herdr);
        let (read, strategy) = scroller.read_history_with_strategy("w1:p1").await.unwrap();

        assert_eq!(strategy, ScrollStrategy::NativeScrollback);
        assert_eq!(read.text, history);
        assert!(
            herdr.sent_text_log("w1:p1").await.is_empty(),
            "NativeScrollback must never call send_text"
        );
    }

    #[tokio::test]
    async fn altscreen_pane_escalates_and_always_restores_the_bottom() {
        // Recent(1000) == Visible (alt-screen VT semantics never accumulate
        // scrollback, D1) but the pane DOES respond to PageUp -- PaneScroller
        // must escalate, return the revealed older content, and always send
        // the restore-to-bottom Ctrl+End afterward so a later Visible read
        // is back to the live tail.
        let herdr = FakeHerdr::new();
        let live_bottom = "Jump to bottom (ctrl+End)\n❯ ";
        let revealed = "...earlier transcript...\nJump to bottom (ctrl+End)\n❯ ";
        herdr.seed_scroll_pane("w1:p1", live_bottom, live_bottom, Some(revealed));

        let scroller = PaneScroller::new(&herdr);
        let (read, strategy) = scroller.read_history_with_strategy("w1:p1").await.unwrap();

        assert_eq!(strategy, ScrollStrategy::EscapeInjection);
        assert_eq!(read.text, revealed, "the escalated read is what's returned");
        assert_eq!(
            herdr.sent_text_log("w1:p1").await,
            vec![PAGE_UP.to_string(), RESTORE_BOTTOM.to_string()],
            "PageUp then Ctrl+End, in that order"
        );

        // The restore actually took effect: a fresh Visible read afterward
        // is back to the live bottom, not stuck showing the escalated read.
        let after = herdr
            .read_pane("w1:p1", ReadSource::Visible, 0)
            .await
            .unwrap();
        assert_eq!(after.text, live_bottom);
    }

    #[tokio::test(start_paused = true)]
    async fn equal_length_different_content_is_not_richer() {
        // "Not richer than" must stay ONE case covering exact equality AND
        // any other non-richer outcome identically -- same length, different
        // bytes, still escalates rather than trusting recent as-is.
        let herdr = FakeHerdr::new();
        herdr.seed_scroll_pane("w1:p1", "abc", "xyz", None);

        let scroller = PaneScroller::new(&herdr);
        let (_read, strategy) = scroller.read_history_with_strategy("w1:p1").await.unwrap();

        assert_eq!(strategy, ScrollStrategy::EscapeInjection);
    }

    #[tokio::test(start_paused = true)]
    async fn escalated_reveal_that_lags_behind_the_redraw_is_still_returned() {
        // Regression, reproduced live against a real Claude Code pane
        // (herdr 0.7.4, Claude Code 2.1.220): the agent's TUI redraw is not
        // synchronous with the send_text() call that triggers it. A single
        // immediate read right after PageUp raced ahead of the redraw and
        // returned the stale, pre-scroll screen every time -- root cause of
        // "load older" appearing to do nothing. The retry loop must wait out
        // a short redraw lag instead of giving up after one read.
        let herdr = FakeHerdr::new();
        let live_bottom = "Jump to bottom (ctrl+End)\n❯ ";
        let revealed = "...earlier transcript...\nJump to bottom (ctrl+End)\n❯ ";
        herdr.seed_scroll_pane("w1:p1", live_bottom, live_bottom, Some(revealed));
        herdr.set_reveal_delay("w1:p1", 2); // stale for the first 2 reads

        let scroller = PaneScroller::new(&herdr);
        let (read, strategy) = scroller.read_history_with_strategy("w1:p1").await.unwrap();

        assert_eq!(strategy, ScrollStrategy::EscapeInjection);
        assert_eq!(
            read.text, revealed,
            "retries wait out the redraw lag instead of giving up after one read"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn escalated_reveal_gives_up_after_exhausting_all_read_attempts() {
        // If the redraw never lands within the retry budget, the last
        // (stale) read is used rather than looping forever or erroring --
        // restore-to-bottom still always runs regardless.
        let herdr = FakeHerdr::new();
        let live_bottom = "Jump to bottom (ctrl+End)\n❯ ";
        let revealed = "...earlier transcript...\nJump to bottom (ctrl+End)\n❯ ";
        herdr.seed_scroll_pane("w1:p1", live_bottom, live_bottom, Some(revealed));
        herdr.set_reveal_delay("w1:p1", 999); // never lands within the budget

        let scroller = PaneScroller::new(&herdr);
        let (read, strategy) = scroller.read_history_with_strategy("w1:p1").await.unwrap();

        assert_eq!(strategy, ScrollStrategy::EscapeInjection);
        assert_eq!(
            read.text, live_bottom,
            "exhausts attempts and returns the last (stale) read, not an error"
        );
        assert_eq!(
            herdr.sent_text_log("w1:p1").await,
            vec![PAGE_UP.to_string(), RESTORE_BOTTOM.to_string()],
            "restore-to-bottom still runs even when the redraw never landed"
        );
    }
}

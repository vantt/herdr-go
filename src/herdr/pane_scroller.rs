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
/// Settle delay before firing the NEXT hop's PageUp once this hop's own
/// redraw has already landed (multi-page scroll-back, CONTEXT.md
/// D-multi-page). Live-reproduced (2026-07-28): back-to-back hops separated
/// only by `ESCALATION_READ_INTERVAL` sometimes caught the agent still
/// mid-transition from THIS hop, so the next one landed on a stale or
/// shorter frame instead of a genuinely deeper one -- multi-page requests
/// plateaued after 2 hops no matter how many were asked for. Deliberately
/// longer than `ESCALATION_READ_INTERVAL`: this settles an entire hop
/// transition, not just one re-read poll.
const INTER_HOP_SETTLE: Duration = Duration::from_millis(150);

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

    /// Read `pane_id`'s older content, `pages` PageUp-hops back from the live
    /// bottom (clamped to at least 1 — every call is a fresh, self-contained
    /// round trip that always ends restored to live, never a continuation of
    /// a previous call's state: the gateway keeps no durable per-pane scroll
    /// state between requests, matching this repo's existing never-store
    /// convention. A caller wanting to go further back than its last request
    /// simply asks for one more page next time (CONTEXT.md D-multi-page);
    /// the whole `pages`-hop journey happens inside this one call before
    /// restoring). Always returns the existing [`ScreenRead`] wire type
    /// (`text` + `revision`, never a bare `Vec<String>`/`String`) so a caller
    /// can build a `ScreenBody` straight off it, matching the existing
    /// non-history response shape — and always carries the read's own
    /// `revision` through unchanged, never fabricated.
    pub async fn read_history(&self, pane_id: &str, pages: usize) -> Result<ScreenRead> {
        self.read_history_with_strategy(pane_id, pages)
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
        pages: usize,
    ) -> Result<(ScreenRead, ScrollStrategy)> {
        let pages = pages.max(1);
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
        // Each hop's completion is judged against the PREVIOUS hop's own
        // content (starting from the pre-scroll `visible`), never a fixed
        // "original" baseline -- that's what lets `pages > 1` reveal
        // successively older content instead of re-landing on the same page
        // (CONTEXT.md D-multi-page). A page that turns out not richer than
        // the one before it (agent has no more to reveal) just stops early;
        // later pages are not attempted.
        let mut escalated = visible.clone();
        for hop in 0..pages {
            self.herdr.send_text(pane_id, PAGE_UP).await?;
            // Poll for the redraw rather than trusting a single immediate
            // read (see ESCALATION_READ_ATTEMPTS) -- stop as soon as a read
            // is richer than this hop's own baseline, i.e. the redraw
            // landed.
            let before = escalated.clone();
            let mut next = self
                .herdr
                .read_pane(pane_id, ReadSource::Visible, 0)
                .await?;
            let mut landed = is_richer(&next, &before);
            for _ in 1..ESCALATION_READ_ATTEMPTS {
                if landed {
                    break;
                }
                tokio::time::sleep(ESCALATION_READ_INTERVAL).await;
                next = self
                    .herdr
                    .read_pane(pane_id, ReadSource::Visible, 0)
                    .await?;
                landed = is_richer(&next, &before);
            }
            if !landed {
                // No further content this hop revealed -- keep the previous
                // (already-confirmed-richer) content rather than overwriting
                // it with this hop's non-richer read, and stop; later hops
                // would be no-ops too.
                break;
            }
            escalated = next;
            // Let this hop's transition fully settle before firing the next
            // PageUp (see INTER_HOP_SETTLE) -- skipped after the last hop,
            // nothing more to fire.
            if hop + 1 < pages {
                tokio::time::sleep(INTER_HOP_SETTLE).await;
            }
        }
        // ALWAYS restore the live bottom afterward, regardless of what the
        // escalated read returned -- an operator's "load older" swipe must
        // never leave the pane scrolled away from its live tail.
        self.herdr.send_text(pane_id, RESTORE_BOTTOM).await?;
        // Wait for the restore to actually land, same reason as the
        // escalation poll above: the agent's own exit from its scroll view
        // is not synchronous with the Ctrl+End bytes landing in the pty.
        // Returning too early left the pane still mid-transition when the
        // very next real keystroke arrived (e.g. a Reply-sheet Send tapped
        // right after "load older") -- live-reproduced as the agent
        // swallowing that Enter as "dismiss scroll view" instead of
        // "submit", so typed text landed with no Enter.
        let mut restored = self
            .herdr
            .read_pane(pane_id, ReadSource::Visible, 0)
            .await?;
        for _ in 1..ESCALATION_READ_ATTEMPTS {
            if restored.text == visible.text {
                break;
            }
            tokio::time::sleep(ESCALATION_READ_INTERVAL).await;
            restored = self
                .herdr
                .read_pane(pane_id, ReadSource::Visible, 0)
                .await?;
        }

        Ok((escalated, ScrollStrategy::EscapeInjection))
    }
}

/// `recent` is "richer" than `visible` when it strictly contains more text --
/// a length comparison, not equality: D2's live evidence showed a primary-
/// screen pane's `recent` read always tails-match `visible` exactly and adds
/// content before it, so a strict length win is exactly "has more to show".
/// Equal length (even with different bytes) is deliberately NOT richer --
/// see the WHY-comment in `read_history_with_strategy` for why "not richer"
/// must stay one case, not just exact equality. Compares `visible_len`
/// (CONTEXT.md D-multi-page), not raw byte length: live testing against
/// multiple real Claude Code panes found its own footer/status bar
/// (elapsed-time and token-percentage counters) re-renders on every single
/// read regardless of scroll position, and ANSI color codes plus
/// right-padding to the terminal's column width both inflate raw length
/// independent of actual content -- together enough noise on some panes to
/// make a strictly-longer escalation register as "not richer" (multi-page
/// scroll-back plateaued after the first hop on one real pane even though
/// PageUp was doing something every time).
fn is_richer(recent: &ScreenRead, visible: &ScreenRead) -> bool {
    visible_len(&recent.text) > visible_len(&visible.text)
}

/// A read's length with ANSI CSI escape sequences stripped and each line's
/// trailing whitespace trimmed -- both are rendering noise (color codes,
/// padding to the terminal's column width) that a raw byte-length
/// comparison would otherwise count as "more content" (see `is_richer`).
fn visible_len(text: &str) -> usize {
    let mut stripped = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            stripped.push(c);
            continue;
        }
        // CSI sequence: ESC '[' ... final byte in 0x40..=0x7E. A bare ESC
        // (no following '[') is simply dropped -- there is nothing else in
        // practice that `format: ansi` reads emit.
        if chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if ('\u{40}'..='\u{7e}').contains(&next) {
                    break;
                }
            }
        }
    }
    stripped.lines().map(|line| line.trim_end().len()).sum()
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
        let (read, strategy) = scroller
            .read_history_with_strategy("w1:p1", 1)
            .await
            .unwrap();

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
        let (read, strategy) = scroller
            .read_history_with_strategy("w1:p1", 1)
            .await
            .unwrap();

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
        let (read, strategy) = scroller
            .read_history_with_strategy("w1:p1", 1)
            .await
            .unwrap();

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
        let (_read, strategy) = scroller
            .read_history_with_strategy("w1:p1", 1)
            .await
            .unwrap();

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
        let (read, strategy) = scroller
            .read_history_with_strategy("w1:p1", 1)
            .await
            .unwrap();

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
        let (read, strategy) = scroller
            .read_history_with_strategy("w1:p1", 1)
            .await
            .unwrap();

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

    #[tokio::test(start_paused = true)]
    async fn restore_wait_lets_a_delayed_exit_from_scroll_view_land_before_returning() {
        // Regression, reproduced live: a real keystroke (e.g. a Reply-sheet
        // Send) arriving right after read_history returns got swallowed by
        // Claude Code as "dismiss scroll view" instead of reaching the
        // composer, because the agent's own exit from its scroll view had
        // not actually landed yet even though Ctrl+End's bytes had. The
        // restore-wait must consume that lag internally so a caller reading
        // Visible immediately afterward already sees the real, settled
        // state, not a still-scrolled one.
        let herdr = FakeHerdr::new();
        let live_bottom = "Jump to bottom (ctrl+End)\n❯ ";
        let revealed = "...earlier transcript...\nJump to bottom (ctrl+End)\n❯ ";
        herdr.seed_scroll_pane("w1:p1", live_bottom, live_bottom, Some(revealed));
        herdr.set_restore_delay("w1:p1", 2); // still looks scrolled for 2 reads after Ctrl+End

        let scroller = PaneScroller::new(&herdr);
        let (_read, strategy) = scroller
            .read_history_with_strategy("w1:p1", 1)
            .await
            .unwrap();
        assert_eq!(strategy, ScrollStrategy::EscapeInjection);

        let after = herdr
            .read_pane("w1:p1", ReadSource::Visible, 0)
            .await
            .unwrap();
        assert_eq!(
            after.text, live_bottom,
            "restore-wait consumed the lag internally -- an immediate read afterward is already settled"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn restore_wait_gives_up_after_exhausting_all_read_attempts() {
        // If the exit-from-scroll-view lag never resolves within the retry
        // budget, read_history_with_strategy must still return (not hang)
        // -- the escalated read itself is unaffected by the restore-side
        // wait either way.
        let herdr = FakeHerdr::new();
        let live_bottom = "Jump to bottom (ctrl+End)\n❯ ";
        let revealed = "...earlier transcript...\nJump to bottom (ctrl+End)\n❯ ";
        herdr.seed_scroll_pane("w1:p1", live_bottom, live_bottom, Some(revealed));
        herdr.set_restore_delay("w1:p1", 999); // never lands within the budget

        let scroller = PaneScroller::new(&herdr);
        let (read, strategy) = scroller
            .read_history_with_strategy("w1:p1", 1)
            .await
            .unwrap();

        assert_eq!(strategy, ScrollStrategy::EscapeInjection);
        assert_eq!(
            read.text, revealed,
            "the escalated read itself is unaffected by the restore-side wait"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn pages_gt_1_hops_back_further_than_a_single_pageup() {
        // User field-test finding (2026-07-28): a single "load older" always
        // revealed the same one page back, no matter how many times it was
        // repeated, because every request restored to live before the next
        // one began -- there was no way to ask for "further back than last
        // time". `pages` lets one call hop back N times before restoring.
        let herdr = FakeHerdr::new();
        let live_bottom = "page0 (live)\n❯ ";
        herdr.seed_scroll_pane(
            "w1:p1",
            live_bottom,
            live_bottom,
            Some("page1 further back\npage0 (live)\n❯ "),
        );
        herdr.push_escape_page(
            "w1:p1",
            "page2 even further back\npage1 further back\npage0 (live)\n❯ ",
        );
        herdr.push_escape_page(
            "w1:p1",
            "page3 furthest back\npage2 even further back\npage1 further back\npage0 (live)\n❯ ",
        );

        let scroller = PaneScroller::new(&herdr);
        let (read, strategy) = scroller
            .read_history_with_strategy("w1:p1", 3)
            .await
            .unwrap();

        assert_eq!(strategy, ScrollStrategy::EscapeInjection);
        assert!(
            read.text.contains("page3 furthest back"),
            "3 pages requested in one call should land on the 3rd page, not just the 1st: got {:?}",
            read.text
        );

        // Always restores afterward regardless of how many pages were hopped.
        let after = herdr
            .read_pane("w1:p1", ReadSource::Visible, 0)
            .await
            .unwrap();
        assert_eq!(after.text, live_bottom);
    }

    #[tokio::test(start_paused = true)]
    async fn pages_beyond_what_the_agent_has_stops_early_without_erroring() {
        // Only 1 page exists; asking for 5 must not error or loop forever --
        // it lands on the 1 available page and stops (later hops aren't
        // richer than the one before, so they're skipped).
        let herdr = FakeHerdr::new();
        let live_bottom = "page0 (live)\n❯ ";
        let only_page = "page1 further back\npage0 (live)\n❯ ";
        herdr.seed_scroll_pane("w1:p1", live_bottom, live_bottom, Some(only_page));

        let scroller = PaneScroller::new(&herdr);
        let (read, strategy) = scroller
            .read_history_with_strategy("w1:p1", 5)
            .await
            .unwrap();

        assert_eq!(strategy, ScrollStrategy::EscapeInjection);
        assert_eq!(read.text, only_page);
    }

    #[test]
    fn visible_len_strips_ansi_color_codes() {
        let plain = "hi";
        let colored = "\x1b[38;2;80;80;80mhi\x1b[0m";
        assert_eq!(visible_len(colored), visible_len(plain));
    }

    #[test]
    fn visible_len_ignores_trailing_padding_on_each_line() {
        // `format: ansi` reads right-pad every line to the terminal's column
        // width -- that padding is layout, not content, and must not count.
        let padded = "line one                    \nline two          \n";
        let unpadded = "line one\nline two\n";
        assert_eq!(visible_len(padded), visible_len(unpadded));
    }

    #[test]
    fn is_richer_ignores_ansi_and_padding_noise_that_a_raw_length_check_would_not() {
        // Live finding (2026-07-28): a real Claude Code pane's footer
        // (elapsed-time/token-percentage counters) re-renders on every read
        // regardless of scroll position, and its ANSI color codes plus
        // right-padding could make a genuinely richer (more actual lines)
        // read register as "not richer" under a naive `.text.len()`
        // comparison -- multi-page scroll-back plateaued after one hop as a
        // result. Heavier noise on the SHORTER-in-content read must not
        // outweigh genuinely more content on the other side.
        let heavily_padded_but_less_content =
            "\x1b[38;2;80;80;80m❯ \x1b[0m                                                  \n";
        let plain_but_more_content = "older line revealed\n❯ \n";
        let a = ScreenRead {
            text: plain_but_more_content.to_string(),
            revision: 1,
        };
        let b = ScreenRead {
            text: heavily_padded_but_less_content.to_string(),
            revision: 1,
        };
        assert!(is_richer(&a, &b));
        assert!(!is_richer(&b, &a));
    }
}

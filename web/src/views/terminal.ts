import { Terminal, type ITheme } from "@xterm/xterm";
import { WebLinksAddon } from "@xterm/addon-web-links";
import "@xterm/xterm/css/xterm.css";
import { fetchScreen, sendReply, sendKeys, type AgentRow } from "../api";
import type { NewPaneRef } from "../main";

export interface TerminalProps {
  agent: AgentRow | NewPaneRef;
  onBack: () => void;
}

/**
 * The header fields terminal detail reads from whatever reference opened it.
 * An AgentRow already carries them; a freshly created NewPaneRef does not, so
 * derive a sane kind/display (never 'undefined') from the data in hand: the
 * preset name as the kind when set, else "shell"; the destination label as the
 * display.
 */
export function terminalHead(agent: AgentRow | NewPaneRef): {
  kind: string;
  display: string;
} {
  if ("workspace_id" in agent) {
    return { kind: agent.name ?? "shell", display: agent.label };
  }
  return { kind: agent.kind, display: agent.display };
}

const POLL_MS = 1500;
const FONT_MIN = 7;
const FONT_MAX = 22;
const FONT_DEFAULT = 12;
// The render grid's row ceiling -- matches herdr's own hard scrollback cap
// (CONTEXT.md D2's `lines: 1000`). The previous 400 silently discarded any
// returned history beyond it (with `scrollback: 0`, xterm drops rows past
// the grid's row count instead of retaining them), before the operator ever
// had a chance to scroll up and see it.
const HISTORY_ROW_CEILING = 1000;
// px tolerance for ".term-viewport is scrolled to its very top" (the "load
// older" trigger point) and, symmetrically, "scrolled to its very bottom"
// (the poll-resume trigger point, CONTEXT.md D4).
const HISTORY_SCROLL_THRESHOLD = 4;
// How long the floating scroll-nudge buttons stay visible after the last
// touch/scroll interaction with the viewport before fading out (CONTEXT.md
// D1's idle auto-hide; exact value is agent's discretion).
const NUDGE_IDLE_MS = 3000;
// Gap between the scroll-nudge buttons and the term-bar footer they float
// above.
const NUDGE_GAP = 8;

const TERMINAL_THEME: ITheme = {
  background: "#0b0e14",
  foreground: "#e4e8f1",
  cursor: "#0b0e14", // hide the cursor block — this is a static snapshot view
  cursorAccent: "#0b0e14",
  selectionBackground: "#2b3550",
  black: "#12161f",
  red: "#f2545b",
  green: "#34d399",
  yellow: "#f5b544",
  blue: "#4f8cff",
  magenta: "#c678dd",
  cyan: "#56b6c2",
  white: "#d7dce6",
  brightBlack: "#5f6b82",
  brightRed: "#f87171",
  brightGreen: "#4ade80",
  brightYellow: "#fbbf24",
  brightBlue: "#7aa8ff",
  brightMagenta: "#d291e4",
  brightCyan: "#6fd3dd",
  brightWhite: "#f4f6fb",
};

/**
 * Observe + reply view. herdr's request API can't size the PTY, so the (wide)
 * screen is polled and rendered at its natural width inside a scroll/zoom
 * viewport — the user pans by scrolling and zooms with A−/A+. Replying opens a
 * textarea and posts the text (decision 675fc93a).
 */
export function renderTerminal(root: HTMLElement, props: TerminalProps): void {
  const { kind, display } = terminalHead(props.agent);
  root.innerHTML = `
    <div class="view view-terminal">
      <div class="term-viewport" id="term-viewport"></div>
      <div class="reply-sheet" id="reply-sheet" hidden>
        <div class="sheet-head">
          <span class="reply-label" id="reply-label">Reply to ${escapeHtml(kind)}</span>
          <button type="button" class="sheet-x" id="reply-close" aria-label="Close">✕</button>
        </div>
        <textarea id="reply-text" class="reply-text" rows="3" placeholder="Type your reply…" autocomplete="off"></textarea>
        <div class="reply-actions">
          <label class="reply-submit-toggle">
            <input type="checkbox" id="reply-enter" checked /> Press Enter (submit)
          </label>
          <div class="reply-buttons">
            <button type="button" class="btn-ghost sheet-switch" id="to-keys">▤ Keys</button>
            <button type="button" class="btn-primary" id="reply-send">Send</button>
          </div>
        </div>
      </div>
      <div class="keys-pad" id="keys-pad" hidden>
        <div class="sheet-head">
          <span class="reply-label">Navigate the agent's menu</span>
          <button type="button" class="sheet-x" id="keys-close" aria-label="Close">✕</button>
        </div>
        <div class="keys-primary" role="group" aria-label="Primary keys">
          <button type="button" class="key-btn key-lg" data-key="up" aria-label="Up">↑</button>
          <button type="button" class="key-btn key-lg" data-key="down" aria-label="Down">↓</button>
          <button type="button" class="key-btn key-lg key-enter" data-key="enter">Enter</button>
        </div>
        <div class="keys-secondary" role="group" aria-label="More keys">
          <button type="button" class="key-btn key-sm" data-key="left" aria-label="Left">←</button>
          <button type="button" class="key-btn key-sm" data-key="right" aria-label="Right">→</button>
          <button type="button" class="key-btn key-sm" data-key="space" aria-label="Space (toggle)">␣</button>
          <button type="button" class="key-btn key-sm" data-key="escape">Esc</button>
          <button type="button" class="btn-ghost sheet-switch" id="to-reply">⌨ Type</button>
        </div>
      </div>
      <div class="scroll-nudge" id="scroll-nudge">
        <button type="button" class="scroll-nudge-btn" id="nudge-up" aria-label="Load older output">▲</button>
        <button type="button" class="scroll-nudge-btn" id="nudge-down" aria-label="Return to live">▼</button>
      </div>
      <footer class="term-bar">
        <button type="button" class="icon-btn" id="back-btn" aria-label="Back to agent list">
          <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
            <path d="M15 18l-6-6 6-6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
        <div class="term-title">
          <span class="term-name">${escapeHtml(display)}</span>
          <span class="term-conn" id="term-conn" data-state="connecting">Loading&hellip;</span>
        </div>
        <div class="term-zoom" role="group" aria-label="Zoom">
          <button type="button" class="icon-btn" id="zoom-out" aria-label="Zoom out">A−</button>
          <button type="button" class="icon-btn" id="zoom-in" aria-label="Zoom in">A+</button>
        </div>
        <button type="button" class="btn btn-ghost term-keys-btn" id="keys-open">Keys</button>
        <button type="button" class="btn btn-primary term-reply-btn" id="reply-open">Type</button>
      </footer>
    </div>
  `;

  const viewport = root.querySelector<HTMLDivElement>("#term-viewport")!;
  const connEl = root.querySelector<HTMLSpanElement>("#term-conn")!;
  const backBtn = root.querySelector<HTMLButtonElement>("#back-btn")!;
  const termBar = root.querySelector<HTMLElement>(".term-bar")!;
  const zoomIn = root.querySelector<HTMLButtonElement>("#zoom-in")!;
  const zoomOut = root.querySelector<HTMLButtonElement>("#zoom-out")!;
  const replyOpen = root.querySelector<HTMLButtonElement>("#reply-open")!;
  const replySheet = root.querySelector<HTMLDivElement>("#reply-sheet")!;
  const keysOpen = root.querySelector<HTMLButtonElement>("#keys-open")!;
  const keysPad = root.querySelector<HTMLDivElement>("#keys-pad")!;
  const keysClose = root.querySelector<HTMLButtonElement>("#keys-close")!;
  const replyText = root.querySelector<HTMLTextAreaElement>("#reply-text")!;
  const replyEnter = root.querySelector<HTMLInputElement>("#reply-enter")!;
  const replySend = root.querySelector<HTMLButtonElement>("#reply-send")!;
  const replyClose = root.querySelector<HTMLButtonElement>("#reply-close")!;
  const toKeys = root.querySelector<HTMLButtonElement>("#to-keys")!;
  const toReply = root.querySelector<HTMLButtonElement>("#to-reply")!;
  const scrollNudge = root.querySelector<HTMLDivElement>("#scroll-nudge")!;
  const nudgeUp = root.querySelector<HTMLButtonElement>("#nudge-up")!;
  const nudgeDown = root.querySelector<HTMLButtonElement>("#nudge-down")!;

  // Float the nudge buttons above the always-visible footer bar, anchored to
  // .view-terminal (position:relative) rather than .term-viewport, which is
  // itself the scroll container and can't be the positioning parent for its
  // own children (CONTEXT.md D1).
  scrollNudge.style.bottom = `${termBar.offsetHeight + NUDGE_GAP}px`;

  let fontSize = FONT_DEFAULT;
  const term = new Terminal({
    fontFamily: "ui-monospace, 'JetBrains Mono', 'SF Mono', Menlo, Consolas, monospace",
    fontSize,
    cursorBlink: false,
    disableStdin: true,
    theme: TERMINAL_THEME,
    scrollback: 0,
    convertEol: true,
  });
  term.open(viewport);
  term.loadAddon(new WebLinksAddon());

  let lastText: string | null = null;
  let disposed = false;
  // Guards the history-load request against the poll loop below: neither the
  // existing `text === lastText` dedupe nor the poll interval had any
  // overlap protection before this feature, so a live poll tick could
  // otherwise interleave with an in-flight "load older" read/restore round
  // trip (CONTEXT.md D4's escape-injection path sends real keys into the
  // pane and always restores the live bottom afterward -- a concurrent poll
  // reading mid-restore would show a torn frame).
  let historyInFlight = false;
  // Arms the "load older" trigger; cleared the instant it fires so a
  // momentum scroll that keeps `.term-viewport` pinned at the top (which
  // fires many 'scroll' events) sends exactly one history request, not one
  // per event. Re-armed only once the operator scrolls away from the top
  // again (see the listener below).
  let historyArmed = true;
  // True while the viewport is scrolled away from the live bottom (entered
  // via the loadOlder() trigger below); pauses poll() so a regular tick
  // doesn't overwrite the history view before the operator scrolls back
  // (CONTEXT.md D4). Cleared by the symmetric bottom-threshold check in the
  // scroll listener below, which also resumes poll() immediately (D7).
  let viewingHistory = false;
  // How many PageUp-hops back from the live bottom the last successful
  // "load older" reached (0 = live). The gateway keeps no scroll depth of
  // its own between requests (every call restores to live before
  // returning), so going further back than last time means asking for one
  // more hop on the NEXT request -- this is that running count. Reset to 0
  // wherever viewingHistory clears back to live.
  let historyDepth = 0;

  function applyScreen(text: string): void {
    if (text === lastText) return;
    lastText = text;
    // Preserve the operator's reading position -- its pixel distance from
    // the bottom of the content -- across the wholesale re-render below,
    // rather than leaving scrollTop untouched. Anchoring on distance from
    // the bottom (instead of distance from the top) is what survives the
    // two cases a plain "leave it alone" breaks: (a) the content crossing
    // the row ceiling above or herdr's own 1000-line cap, where the oldest
    // rows age out from under a stale scrollTop, and (b) a content shrink --
    // e.g. the ordinary poll tick right after a "load older" escalation,
    // since a history read is a one-shot expansion, not a persistent poll
    // mode (CONTEXT.md D4) -- where resize() running before reset() below
    // can otherwise clamp scrollTop irrecoverably. When the prior position
    // no longer fits (content shrank past it), preserveScrollTop degrades to
    // "scroll as far up as the new content allows" rather than an invalid
    // value.
    const distanceFromBottom = viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight;
    // Size the grid to the content so wide TUI lines are not wrapped; the
    // viewport scrolls to pan.
    const lines = text.split("\n");
    const cols = clamp(
      lines.reduce((m, l) => Math.max(m, stripAnsiLen(l)), 0) + 1,
      20,
      400,
    );
    const rows = clamp(lines.length + 1, 4, HISTORY_ROW_CEILING);
    if (term.cols !== cols || term.rows !== rows) term.resize(cols, rows);
    term.reset();
    term.write(text.replace(/\n/g, "\r\n"));
    viewport.scrollTop = preserveScrollTop(distanceFromBottom, viewport.scrollHeight, viewport.clientHeight);
  }

  async function poll(): Promise<void> {
    if (disposed || historyInFlight || viewingHistory) return;
    try {
      const screen = await fetchScreen(props.agent.pane_id);
      if (screen === null) {
        setState("closed", "Pane gone");
        return;
      }
      applyScreen(screen.text);
      setState("open", "Live");
    } catch {
      setState("closed", "Disconnected");
    }
  }

  /**
   * Fired when the operator scrolls/swipes .term-viewport to its very top:
   * fetches older pane content (the backend's `PaneScroller` escalation via
   * `?history=<n>`, CONTEXT.md D9/D-multi-page) and does a full wholesale
   * re-render with the result via the existing applyScreen() path above --
   * not a prepend, matching applyScreen's actual wholesale-replace design
   * (plan.md Discovery: applyScreen has no prepend path). Each call asks for
   * one hop further back than the last successful one (historyDepth), since
   * the gateway restores to live and forgets depth between requests -- this
   * running count is what lets repeated "load older" triggers keep going
   * further back instead of re-landing on the same page.
   */
  async function loadOlder(): Promise<void> {
    historyInFlight = true;
    const requestedDepth = historyDepth + 1;
    try {
      const screen = await fetchScreen(props.agent.pane_id, requestedDepth);
      if (disposed || screen === null) return;
      applyScreen(screen.text);
      historyDepth = requestedDepth;
    } catch {
      // Leave the current view (and historyDepth) as-is; scrolling away
      // from the top and back re-arms the trigger below for a retry.
    } finally {
      historyInFlight = false;
    }
  }

  // Explicit "operator asked to return to live" reset -- used by nudge-down
  // and by opening the Reply/Keys sheet (D6), never inferred from a scroll
  // position alone there: a short page (real Claude Code pages vary in
  // length hop to hop, CONTEXT.md D-multi-page) can leave scrollTop stuck
  // near 0 even after `scrollTop = scrollHeight` (nothing to scroll to), so
  // relying on the scroll listener's heuristic for an explicit user action
  // risked never actually resuming poll(). The passive drag-to-bottom case
  // below still needs a heuristic (there is no explicit action to hook),
  // hence the separate, narrower guard there.
  function returnToLive(): void {
    viewingHistory = false;
    historyDepth = 0;
    void poll();
  }

  viewport.addEventListener("scroll", () => {
    if (viewport.scrollTop > HISTORY_SCROLL_THRESHOLD) {
      historyArmed = true;
    } else if (historyArmed && !historyInFlight && !disposed) {
      historyArmed = false;
      viewingHistory = true;
      void loadOlder();
    }

    // Symmetric bottom-threshold resume (CONTEXT.md D4's discretion note):
    // fires on any scroll event that lands the viewport back near the live
    // bottom, whether a drag-scroll or a programmatic scrollTop=scrollHeight
    // assignment (e.g. applySheetInset, D6) -- both dispatch a native
    // 'scroll' event. Reflects promptly (D7) rather than waiting up to
    // POLL_MS for the next tick.
    // Bottom-threshold resume requires scrollTop to be meaningfully away
    // from the top, not just "distance from bottom" alone: after loading a
    // page short enough to barely overflow the viewport (real Claude Code
    // pages vary in length hop to hop, CONTEXT.md D-multi-page), nudge-up's
    // own scrollTop=0 jump-to-top can ALSO read as "near the bottom" (there
    // is nothing to scroll), which used to be mistaken for a genuine
    // return-to-live drag -- live field-test finding (2026-07-28):
    // "occasionally jumps back to live on its own, not felt as caused by
    // the down button". A real return-to-live (drag or nudge-down's
    // scrollTop=scrollHeight jump) always leaves scrollTop meaningfully
    // above 0, so this guard costs nothing there.
    const distanceFromBottom = viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight;
    if (
      viewingHistory &&
      viewport.scrollTop > HISTORY_SCROLL_THRESHOLD &&
      distanceFromBottom <= HISTORY_SCROLL_THRESHOLD
    ) {
      viewingHistory = false;
      historyDepth = 0;
      void poll();
    }
  });

  function setState(state: "connecting" | "open" | "closed", label: string): void {
    connEl.dataset.state = state;
    connEl.textContent = label;
  }

  function setFont(next: number): void {
    fontSize = clamp(next, FONT_MIN, FONT_MAX);
    term.options.fontSize = fontSize;
  }

  zoomIn.addEventListener("click", () => setFont(fontSize + 1));
  zoomOut.addEventListener("click", () => setFont(fontSize - 1));

  // Scroll-nudge buttons (CONTEXT.md D1/D2/D3): a discoverable, always-present
  // affordance for the same load-older/return-to-live actions the viewport's
  // own scroll thresholds already trigger. Fades out after a period with no
  // touch/scroll interaction on the viewport, and is hidden outright whenever
  // the Reply or Keys sheet covers the same corner.
  let nudgeIdleTimer: number | null = null;
  // Never let the idle-hide timer fire while the operator is off the live
  // bottom (viewingHistory): reading revealed history is itself a period of
  // no touch/scroll interaction, and hiding the only affordance back to live
  // (or to load further) exactly then defeats the buttons' purpose. Re-check
  // every NUDGE_IDLE_MS instead of hiding outright, so hiding still resumes
  // promptly once the operator is back live and stays idle.
  function scheduleNudgeIdle(): void {
    if (nudgeIdleTimer !== null) window.clearTimeout(nudgeIdleTimer);
    nudgeIdleTimer = window.setTimeout(() => {
      if (viewingHistory) {
        scheduleNudgeIdle();
      } else {
        scrollNudge.classList.add("is-idle");
      }
    }, NUDGE_IDLE_MS);
  }
  function showNudge(): void {
    scrollNudge.classList.remove("is-idle");
    scheduleNudgeIdle();
  }
  function updateNudgeVisibility(): void {
    scrollNudge.hidden = !replySheet.hidden || !keysPad.hidden;
  }
  viewport.addEventListener("scroll", showNudge);
  viewport.addEventListener("touchstart", showNudge);
  showNudge();

  nudgeUp.addEventListener("click", () => {
    if (historyInFlight || disposed) return;
    // Unlike the drag-scroll-to-top trigger below, a button tap usually fires
    // from the live bottom, where applyScreen()'s preserveScrollTop (distance-
    // from-bottom preservation) lands the freshly-expanded history right back
    // at the bottom -- a full re-render with no visible scroll. Jump to the
    // top explicitly, mirroring nudge-down's explicit jump to the bottom.
    // Consume historyArmed first so this programmatic scrollTop=0 (which
    // fires a native 'scroll' event) doesn't re-trigger the listener below
    // into a second, redundant loadOlder() call.
    historyArmed = false;
    viewingHistory = true;
    void loadOlder().then(() => {
      viewport.scrollTop = 0;
    });
  });
  nudgeDown.addEventListener("click", () => {
    viewport.scrollTop = viewport.scrollHeight;
    returnToLive();
  });

  // The Keys pad and Reply sheet are mutually-exclusive bottom sheets. Opening
  // one closes the other; a one-tap switch button on each jumps to the other so
  // the "navigate, then type" (option ending in a free-text prompt) flow needs
  // no close-then-open dance.
  // Keep the agent's prompt (usually at the bottom of the screen) visible when a
  // bottom sheet covers the lower terminal: reserve viewport space equal to how
  // far the sheet rises above the footer bar, and scroll to the newest content.
  const SHEET_GAP = 8;
  function applySheetInset(sheet: HTMLElement): void {
    const overlap = sheet.offsetHeight - termBar.offsetHeight;
    viewport.style.paddingBottom = overlap > 0 ? `${overlap + SHEET_GAP}px` : "";
    viewport.scrollTop = viewport.scrollHeight;
    returnToLive();
  }
  function clearSheetInset(): void {
    viewport.style.paddingBottom = "";
  }

  // Reserve space for the OS keyboard on top of the sheet/term-bar overlap
  // above: some browsers (e.g. Chrome's default interactive-widget=resizes-visual)
  // shrink window.visualViewport without shrinking the layout viewport (and
  // thus 100dvh), so the reply-sheet and terminal content need their own
  // keyboard-aware inset. Reply-sheet only (D1) — keys-pad has no focusable
  // input, so no keyboard ever opens for it.
  let keyboardInsetHandler: (() => void) | null = null;
  function applyKeyboardInset(): void {
    const vv = window.visualViewport;
    if (!vv) return;
    const inset = computeKeyboardInset(window.innerHeight, vv.height, vv.offsetTop);
    replySheet.style.bottom = inset > 0 ? `${inset}px` : "";
    const overlap = replySheet.offsetHeight - termBar.offsetHeight;
    const basePadding = overlap > 0 ? overlap + SHEET_GAP : 0;
    const totalPadding = basePadding + inset;
    viewport.style.paddingBottom = totalPadding > 0 ? `${totalPadding}px` : "";
  }

  function openReply(): void {
    keysPad.hidden = true;
    replySheet.hidden = false;
    applySheetInset(replySheet);
    replyText.focus();
    if (window.visualViewport && !keyboardInsetHandler) {
      keyboardInsetHandler = applyKeyboardInset;
      window.visualViewport.addEventListener("resize", keyboardInsetHandler);
    }
    updateNudgeVisibility();
  }
  function openKeys(): void {
    closeReply();
    keysPad.hidden = false;
    applySheetInset(keysPad);
    updateNudgeVisibility();
  }

  replyOpen.addEventListener("click", openReply);
  replyClose.addEventListener("click", closeReply);
  toKeys.addEventListener("click", openKeys);

  // Key pad: press arrow/Enter/Esc/Space keys to drive a TUI option menu. The
  // pad stays open across presses so you can navigate then confirm; each press
  // re-polls so the screen reflects the move.
  keysOpen.addEventListener("click", openKeys);
  keysClose.addEventListener("click", () => {
    keysPad.hidden = true;
    clearSheetInset();
    updateNudgeVisibility();
  });
  toReply.addEventListener("click", openReply);
  keysPad.querySelectorAll<HTMLButtonElement>(".key-btn").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const key = btn.dataset.key;
      if (!key) return;
      await sendKeys(props.agent.pane_id, [key]);
      void poll(); // reflect the navigation promptly
    });
  });

  replySend.addEventListener("click", async () => {
    const text = replyText.value;
    if (text.length === 0) return;
    replySend.disabled = true;
    const ok = await sendReply(props.agent.pane_id, text, replyEnter.checked);
    replySend.disabled = false;
    if (ok) {
      replyText.value = "";
      closeReply();
      void poll(); // reflect the reply promptly
    } else {
      replyText.setAttribute("aria-invalid", "true");
    }
  });

  function closeReply(): void {
    replySheet.hidden = true;
    replyText.removeAttribute("aria-invalid");
    clearSheetInset();
    replySheet.style.bottom = "";
    if (keyboardInsetHandler && window.visualViewport) {
      window.visualViewport.removeEventListener("resize", keyboardInsetHandler);
    }
    keyboardInsetHandler = null;
    updateNudgeVisibility();
  }

  backBtn.addEventListener("click", () => {
    disposed = true;
    clearInterval(timer);
    if (nudgeIdleTimer !== null) window.clearTimeout(nudgeIdleTimer);
    term.dispose();
    props.onBack();
  });

  void poll();
  const timer = window.setInterval(() => void poll(), POLL_MS);
}

/**
 * The gap between the layout viewport and the visual viewport, nonzero
 * exactly when an OS keyboard is showing and the browser did not already
 * shrink the layout viewport (e.g. 100dvh) to compensate.
 */
export function computeKeyboardInset(
  innerHeight: number,
  viewportHeight: number,
  offsetTop: number,
): number {
  return Math.max(0, innerHeight - viewportHeight - offsetTop);
}

/** Length of a line ignoring ANSI escape sequences (for grid sizing). */
export function stripAnsiLen(line: string): number {
  // eslint-disable-next-line no-control-regex
  return line.replace(/\x1b\[[0-9;?]*[a-zA-Z]/g, "").length;
}

/**
 * The scrollTop to apply after a wholesale re-render so the operator's
 * distance from the bottom of the content survives, clamped to the new
 * scrollable range. Anchoring on distance-from-bottom (rather than leaving
 * scrollTop untouched) is what lands the operator as close as possible to
 * their prior reading position even when the rendered content's height
 * changed from the front -- a row-ceiling crossing/sliding history window,
 * or a shrink -- instead of a stale scrollTop that clips off-screen or
 * points at content that no longer exists.
 */
export function preserveScrollTop(
  distanceFromBottom: number,
  newScrollHeight: number,
  newClientHeight: number,
): number {
  const maxScrollTop = Math.max(0, newScrollHeight - newClientHeight);
  return clamp(maxScrollTop - distanceFromBottom, 0, maxScrollTop);
}

function clamp(n: number, lo: number, hi: number): number {
  return Math.max(lo, Math.min(hi, n));
}

function escapeHtml(value: string): string {
  const div = document.createElement("div");
  div.textContent = value;
  return div.innerHTML;
}

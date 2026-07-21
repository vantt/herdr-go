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

  function applyScreen(text: string): void {
    if (text === lastText) return;
    lastText = text;
    // Size the grid to the content so wide TUI lines are not wrapped; the
    // viewport scrolls to pan.
    const lines = text.split("\n");
    const cols = clamp(
      lines.reduce((m, l) => Math.max(m, stripAnsiLen(l)), 0) + 1,
      20,
      400,
    );
    const rows = clamp(lines.length + 1, 4, 400);
    if (term.cols !== cols || term.rows !== rows) term.resize(cols, rows);
    term.reset();
    term.write(text.replace(/\n/g, "\r\n"));
  }

  async function poll(): Promise<void> {
    if (disposed) return;
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
  }
  function clearSheetInset(): void {
    viewport.style.paddingBottom = "";
  }

  function openReply(): void {
    keysPad.hidden = true;
    replySheet.hidden = false;
    applySheetInset(replySheet);
    replyText.focus();
  }
  function openKeys(): void {
    closeReply();
    keysPad.hidden = false;
    applySheetInset(keysPad);
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
  }

  backBtn.addEventListener("click", () => {
    disposed = true;
    clearInterval(timer);
    term.dispose();
    props.onBack();
  });

  void poll();
  const timer = window.setInterval(() => void poll(), POLL_MS);
}

/** Length of a line ignoring ANSI escape sequences (for grid sizing). */
export function stripAnsiLen(line: string): number {
  // eslint-disable-next-line no-control-regex
  return line.replace(/\x1b\[[0-9;?]*[a-zA-Z]/g, "").length;
}

function clamp(n: number, lo: number, hi: number): number {
  return Math.max(lo, Math.min(hi, n));
}

function escapeHtml(value: string): string {
  const div = document.createElement("div");
  div.textContent = value;
  return div.innerHTML;
}

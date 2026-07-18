import { Terminal, type ITheme } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { fetchScreen, sendReply, type AgentRow } from "../api";

export interface TerminalProps {
  agent: AgentRow;
  onBack: () => void;
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
  root.innerHTML = `
    <div class="view view-terminal">
      <header class="term-bar">
        <button type="button" class="icon-btn" id="back-btn" aria-label="Back to agent list">
          <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
            <path d="M15 18l-6-6 6-6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
        <div class="term-title">
          <span class="term-name">${escapeHtml(props.agent.display)}</span>
          <span class="term-conn" id="term-conn" data-state="connecting">Loading&hellip;</span>
        </div>
        <div class="term-zoom" role="group" aria-label="Zoom">
          <button type="button" class="icon-btn" id="zoom-out" aria-label="Zoom out">A−</button>
          <button type="button" class="icon-btn" id="zoom-in" aria-label="Zoom in">A+</button>
        </div>
      </header>
      <div class="term-viewport" id="term-viewport"></div>
      <button type="button" class="reply-fab" id="reply-fab" aria-label="Reply to this agent">
        Reply
      </button>
      <div class="reply-sheet" id="reply-sheet" hidden>
        <label class="reply-label" for="reply-text">Reply to ${escapeHtml(props.agent.kind)}</label>
        <textarea id="reply-text" class="reply-text" rows="3" placeholder="Type your reply…" autocomplete="off"></textarea>
        <div class="reply-actions">
          <label class="reply-submit-toggle">
            <input type="checkbox" id="reply-enter" checked /> Press Enter (submit)
          </label>
          <div class="reply-buttons">
            <button type="button" class="btn-ghost" id="reply-cancel">Cancel</button>
            <button type="button" class="btn-primary" id="reply-send">Send</button>
          </div>
        </div>
      </div>
    </div>
  `;

  const viewport = root.querySelector<HTMLDivElement>("#term-viewport")!;
  const connEl = root.querySelector<HTMLSpanElement>("#term-conn")!;
  const backBtn = root.querySelector<HTMLButtonElement>("#back-btn")!;
  const zoomIn = root.querySelector<HTMLButtonElement>("#zoom-in")!;
  const zoomOut = root.querySelector<HTMLButtonElement>("#zoom-out")!;
  const replyFab = root.querySelector<HTMLButtonElement>("#reply-fab")!;
  const replySheet = root.querySelector<HTMLDivElement>("#reply-sheet")!;
  const replyText = root.querySelector<HTMLTextAreaElement>("#reply-text")!;
  const replyEnter = root.querySelector<HTMLInputElement>("#reply-enter")!;
  const replySend = root.querySelector<HTMLButtonElement>("#reply-send")!;
  const replyCancel = root.querySelector<HTMLButtonElement>("#reply-cancel")!;

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

  replyFab.addEventListener("click", () => {
    replySheet.hidden = false;
    replyText.focus();
  });
  replyCancel.addEventListener("click", closeReply);

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

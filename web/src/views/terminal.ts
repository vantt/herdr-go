import { Terminal, type ITheme } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import "@xterm/xterm/css/xterm.css";
import { TerminalSocket, type ConnectionState } from "../ws";
import type { AgentRow } from "../api";

export interface TerminalProps {
  agent: AgentRow;
  onBack: () => void;
}

type PaneMode = "control" | "observe";

const CONN_LABEL: Record<ConnectionState, string> = {
  connecting: "Connecting…",
  open: "Connected",
  closed: "Disconnected",
};

const TERMINAL_THEME: ITheme = {
  background: "#0b0e14",
  foreground: "#e4e8f1",
  cursor: "#4f8cff",
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
          <span class="term-conn" id="term-conn" data-state="connecting">Connecting&hellip;</span>
        </div>
        <button type="button" class="mode-toggle" id="mode-toggle" data-mode="control" aria-label="Control or observe mode">
          <span class="mode-option" data-value="control">Control</span>
          <span class="mode-option" data-value="observe">Observe</span>
        </button>
      </header>
      <div class="term-viewport" id="term-viewport"></div>
      <button type="button" class="reconnect-btn" id="reconnect-btn" hidden>Reconnect</button>
    </div>
  `;

  const viewport = root.querySelector<HTMLDivElement>("#term-viewport")!;
  const connEl = root.querySelector<HTMLSpanElement>("#term-conn")!;
  const reconnectBtn = root.querySelector<HTMLButtonElement>("#reconnect-btn")!;
  const modeToggle = root.querySelector<HTMLButtonElement>("#mode-toggle")!;
  const backBtn = root.querySelector<HTMLButtonElement>("#back-btn")!;

  const term = new Terminal({
    fontFamily: "ui-monospace, 'JetBrains Mono', 'SF Mono', Menlo, Consolas, monospace",
    fontSize: 14,
    cursorBlink: true,
    theme: TERMINAL_THEME,
    scrollback: 5000,
  });
  const fit = new FitAddon();
  term.loadAddon(fit);
  term.loadAddon(new WebLinksAddon());
  term.open(viewport);
  fit.fit();

  let mode: PaneMode = "control";
  let socket: TerminalSocket | null = null;

  // Registered once: TerminalSocket itself no-ops send*() outside control
  // mode, so this never needs to branch on `mode` explicitly.
  term.onData((data) => socket?.sendInput(data));

  function connect(): void {
    reconnectBtn.hidden = true;
    fit.fit();
    socket?.close();
    socket = new TerminalSocket(term, {
      pane: props.agent.pane_id,
      mode,
      takeover: mode === "control",
      cols: term.cols,
      rows: term.rows,
      onState: setConnState,
      onError: (reason) => {
        setConnState("closed");
        connEl.textContent = reason || CONN_LABEL.closed;
      },
    });
    socket.connect();
  }

  function setConnState(state: ConnectionState): void {
    connEl.dataset.state = state;
    connEl.textContent = CONN_LABEL[state];
    reconnectBtn.hidden = state !== "closed";
  }

  function fitAndReflow(): void {
    fit.fit();
    socket?.sendResize(term.cols, term.rows);
  }

  const handleResize = () => fitAndReflow();
  const handleOrientation = () => setTimeout(fitAndReflow, 200);
  window.addEventListener("resize", handleResize);
  window.addEventListener("orientationchange", handleOrientation);

  modeToggle.addEventListener("click", () => {
    mode = mode === "control" ? "observe" : "control";
    term.options.disableStdin = mode !== "control";
    modeToggle.dataset.mode = mode;
    connect();
  });

  reconnectBtn.addEventListener("click", () => connect());

  backBtn.addEventListener("click", () => {
    window.removeEventListener("resize", handleResize);
    window.removeEventListener("orientationchange", handleOrientation);
    socket?.close();
    term.dispose();
    props.onBack();
  });

  connect();
}

function escapeHtml(value: string): string {
  const div = document.createElement("div");
  div.textContent = value;
  return div.innerHTML;
}

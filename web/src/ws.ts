// Terminal WebSocket relay client. Decodes server frames onto an xterm-like
// terminal and encodes local input/resize back to the gateway, matching the
// wire shapes in src/web/relay.rs and src/herdr/wire.rs exactly.

/** The subset of xterm.js's Terminal API the socket needs — kept minimal so
 * logic can be unit tested against a plain fake object, no real xterm/DOM. */
export interface TerminalLike {
  reset(): void;
  write(data: Uint8Array | string): void;
}

export interface TerminalFrameMsg {
  type: "terminal.frame";
  seq: number;
  encoding: string;
  width: number;
  height: number;
  full: boolean;
  bytes: string;
}

export interface GatewayErrorMsg {
  type: "gateway.error";
  reason: string;
}

export type ServerFrame = TerminalFrameMsg | GatewayErrorMsg;

export type ConnectionState = "connecting" | "open" | "closed";

/** Decode a base64 string into raw bytes (browser-native, no Buffer). */
export function base64ToBytes(b64: string): Uint8Array {
  if (b64.length === 0) return new Uint8Array(0);
  const binary = atob(b64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

/**
 * Apply one server frame to a terminal. `full=true` is a whole-screen redraw
 * and must reset the terminal before writing; a diff frame just writes.
 * `gateway.error` messages carry no terminal payload and are ignored here —
 * callers surface `reason` through their own error handler instead.
 */
export function applyFrame(term: TerminalLike, frame: ServerFrame): void {
  if (frame.type !== "terminal.frame") return;
  if (frame.full) term.reset();
  term.write(base64ToBytes(frame.bytes));
}

export interface TerminalSocketOptions {
  pane: string;
  mode: "control" | "observe";
  takeover?: boolean;
  cols: number;
  rows: number;
  onState?: (state: ConnectionState) => void;
  onError?: (reason: string) => void;
}

/**
 * Owns the WebSocket connection for one terminal pane: opens `/ws/terminal`,
 * applies incoming frames to the given terminal, and forwards local
 * input/resize back to the server (control mode only — in observe mode
 * `send*` calls are silently no-ops, matching the backend's read-only pane).
 */
export class TerminalSocket {
  private ws: WebSocket | null = null;
  private readonly term: TerminalLike;
  private readonly opts: TerminalSocketOptions;

  constructor(term: TerminalLike, opts: TerminalSocketOptions) {
    this.term = term;
    this.opts = opts;
  }

  connect(): void {
    const { pane, mode, takeover, cols, rows } = this.opts;
    const proto = location.protocol === "https:" ? "wss" : "ws";
    const params = new URLSearchParams({
      pane,
      mode,
      cols: String(cols),
      rows: String(rows),
    });
    if (takeover) params.set("takeover", "true");

    this.opts.onState?.("connecting");
    const ws = new WebSocket(`${proto}://${location.host}/ws/terminal?${params.toString()}`);
    this.ws = ws;

    ws.onopen = () => this.opts.onState?.("open");
    ws.onclose = () => this.opts.onState?.("closed");
    ws.onerror = () => this.opts.onState?.("closed");
    ws.onmessage = (ev) => this.handleMessage(ev.data as string);
  }

  private handleMessage(raw: string): void {
    let msg: ServerFrame;
    try {
      msg = JSON.parse(raw) as ServerFrame;
    } catch {
      return;
    }
    if (msg.type === "gateway.error") {
      this.opts.onError?.(msg.reason);
      return;
    }
    applyFrame(this.term, msg);
  }

  sendInput(data: string): void {
    this.send({ t: "input", data });
  }

  sendResize(cols: number, rows: number): void {
    this.send({ t: "resize", cols, rows });
  }

  private send(payload: unknown): void {
    if (this.opts.mode !== "control") return;
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(payload));
    }
  }

  close(): void {
    this.ws?.close();
    this.ws = null;
  }
}

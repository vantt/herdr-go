// SGR parser for herdr pane reads. Turns `pane.read(format:"ansi")` text into
// styled lines the view can render as plain DOM.
//
// A full terminal emulator is not needed here, and that is a property of the
// producer rather than an observation that might stop holding: herdr serves
// this text through ghostty's VT formatter over an *already-rendered cell
// grid* (upstreams/herdr/src/ghostty/mod.rs, read_ansi_screen -> the Vt
// formatter). A grid dump has no cursor, so it cannot carry cursor motion,
// erase-in-line, scroll regions or alternate-screen switches -- only SGR
// colour/style runs and text. Anything else that does turn up is consumed and
// dropped rather than rendered.
//
// Text is returned as data. The renderer must place it in text nodes, never
// innerHTML: pane output is untrusted, and this module deriving only colours
// and weights is what keeps it off the XSS surface.

/** One run of text sharing a single style. */
export interface AnsiSegment {
  text: string;
  /** Resolved CSS colour, or undefined for the viewport default. */
  fg?: string;
  bg?: string;
  bold?: boolean;
  dim?: boolean;
  italic?: boolean;
  underline?: boolean;
  strike?: boolean;
}

export interface StyledLine {
  segments: AnsiSegment[];
}

/**
 * The 16 ANSI colours, matching the values the terminal view already renders
 * with. Kept here because they belong with the ANSI code that resolves them;
 * `views/terminal.ts` still carries its own xterm `ITheme` copy until the
 * renderer swap retires it.
 */
export const ANSI_PALETTE = {
  background: "#0b0e14",
  foreground: "#e4e8f1",
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
} as const;

const BASE_16 = [
  ANSI_PALETTE.black,
  ANSI_PALETTE.red,
  ANSI_PALETTE.green,
  ANSI_PALETTE.yellow,
  ANSI_PALETTE.blue,
  ANSI_PALETTE.magenta,
  ANSI_PALETTE.cyan,
  ANSI_PALETTE.white,
  ANSI_PALETTE.brightBlack,
  ANSI_PALETTE.brightRed,
  ANSI_PALETTE.brightGreen,
  ANSI_PALETTE.brightYellow,
  ANSI_PALETTE.brightBlue,
  ANSI_PALETTE.brightMagenta,
  ANSI_PALETTE.brightCyan,
  ANSI_PALETTE.brightWhite,
];

/** Terminals advance to the next multiple of 8 columns on a tab. */
const TAB_WIDTH = 8;

interface State {
  fg?: string;
  bg?: string;
  bold?: boolean;
  dim?: boolean;
  italic?: boolean;
  underline?: boolean;
  strike?: boolean;
  inverse?: boolean;
}

/**
 * Resolve an xterm-256 index to a hex colour: 0-15 are the named palette,
 * 16-231 a 6x6x6 RGB cube, 232-255 a 24-step grey ramp.
 */
function color256(n: number): string | undefined {
  if (!Number.isInteger(n) || n < 0 || n > 255) return undefined;
  if (n < 16) return BASE_16[n];
  if (n < 232) {
    const i = n - 16;
    const steps = [0, 95, 135, 175, 215, 255];
    const r = steps[Math.floor(i / 36) % 6];
    const g = steps[Math.floor(i / 6) % 6];
    const b = steps[i % 6];
    return rgb(r, g, b);
  }
  const level = 8 + (n - 232) * 10;
  return rgb(level, level, level);
}

function rgb(r: number, g: number, b: number): string | undefined {
  const ok = (v: number) => Number.isInteger(v) && v >= 0 && v <= 255;
  if (!ok(r) || !ok(g) || !ok(b)) return undefined;
  const hex = (v: number) => v.toString(16).padStart(2, "0");
  return `#${hex(r)}${hex(g)}${hex(b)}`;
}

/**
 * Read one extended-colour selector (`38`/`48`) starting at `i`, returning the
 * colour and how many parameters it consumed.
 *
 * Both separator conventions occur in the wild and mean the same thing: the
 * common `38;5;n` / `38;2;r;g;b` semicolon form, and the ISO 8613-6 colon form
 * `38:5:n` / `38:2:r:g:b`, including the variant `38:2::r:g:b` whose empty
 * field is a colour-space id. Splitting on both separators flattens all of
 * them into one list, so the only extra handling needed is skipping an empty
 * colour-space field.
 */
function readExtendedColor(parts: string[], i: number): { color?: string; used: number } {
  const kind = parts[i + 1];
  if (kind === "5") {
    return { color: color256(Number(parts[i + 2])), used: 3 };
  }
  if (kind === "2") {
    // `2::r:g:b` -- an empty colour-space field sits between the selector and
    // the components.
    const off = parts[i + 2] === "" ? 3 : 2;
    return {
      color: rgb(Number(parts[i + off]), Number(parts[i + off + 1]), Number(parts[i + off + 2])),
      used: off + 3,
    };
  }
  // Unknown extended form: drop the selector alone rather than guessing how
  // many parameters belong to it.
  return { used: 1 };
}

/** Apply one SGR parameter string to the running style state. */
function applySgr(state: State, params: string): void {
  // An empty parameter list (`ESC[m`) means reset, same as `ESC[0m`.
  const parts = params === "" ? ["0"] : params.split(/[;:]/);

  for (let i = 0; i < parts.length; i++) {
    const raw = parts[i];
    const code = raw === "" ? 0 : Number(raw);
    if (!Number.isFinite(code)) continue;

    switch (code) {
      case 0:
        state.fg = undefined;
        state.bg = undefined;
        state.bold = undefined;
        state.dim = undefined;
        state.italic = undefined;
        state.underline = undefined;
        state.strike = undefined;
        state.inverse = undefined;
        break;
      case 1:
        state.bold = true;
        break;
      case 2:
        state.dim = true;
        break;
      case 3:
        state.italic = true;
        break;
      case 4:
        state.underline = true;
        break;
      case 7:
        state.inverse = true;
        break;
      case 9:
        state.strike = true;
        break;
      case 22:
        state.bold = undefined;
        state.dim = undefined;
        break;
      case 23:
        state.italic = undefined;
        break;
      case 24:
        state.underline = undefined;
        break;
      case 27:
        state.inverse = undefined;
        break;
      case 29:
        state.strike = undefined;
        break;
      case 38: {
        const { color, used } = readExtendedColor(parts, i);
        state.fg = color;
        i += used - 1;
        break;
      }
      case 39:
        state.fg = undefined;
        break;
      case 48: {
        const { color, used } = readExtendedColor(parts, i);
        state.bg = color;
        i += used - 1;
        break;
      }
      case 49:
        state.bg = undefined;
        break;
      default:
        if (code >= 30 && code <= 37) state.fg = BASE_16[code - 30];
        else if (code >= 40 && code <= 47) state.bg = BASE_16[code - 40];
        else if (code >= 90 && code <= 97) state.fg = BASE_16[code - 90 + 8];
        else if (code >= 100 && code <= 107) state.bg = BASE_16[code - 100 + 8];
        // Anything else is a style this view does not render; ignore it
        // rather than failing the whole line.
        break;
    }
  }
}

/** Snapshot the style state into a segment, resolving inverse video. */
function segmentOf(state: State, text: string): AnsiSegment {
  const fg = state.inverse ? (state.bg ?? ANSI_PALETTE.background) : state.fg;
  const bg = state.inverse ? (state.fg ?? ANSI_PALETTE.foreground) : state.bg;
  const seg: AnsiSegment = { text };
  if (fg) seg.fg = fg;
  if (bg) seg.bg = bg;
  if (state.bold) seg.bold = true;
  if (state.dim) seg.dim = true;
  if (state.italic) seg.italic = true;
  if (state.underline) seg.underline = true;
  if (state.strike) seg.strike = true;
  return seg;
}

/**
 * Parse ANSI-SGR text into styled lines.
 *
 * `\r` is last-write-wins within a line: a carriage return with more text after
 * it on the same line discards what the line held and restarts at column 0,
 * keeping the style state, which collapses progress-bar and spinner redraws to
 * their final frame. A `\r` that is merely part of a CRLF terminator, or that
 * ends the input, overwrites nothing -- herdr's buffer is CRLF, so treating
 * every `\r` as a redraw would blank every line.
 *
 * `\t` advances to the next multiple of 8 columns. The grid dumps this parser
 * normally sees have already expanded tabs into cells, so this only matters for
 * text that reaches the view by some other path.
 */
export function parseAnsi(input: string): StyledLine[] {
  const lines: StyledLine[] = [];
  const state: State = {};

  let segments: AnsiSegment[] = [];
  let buf = "";
  let column = 0;

  const flush = (): void => {
    if (!buf) return;
    segments.push(segmentOf(state, buf));
    buf = "";
  };

  const endLine = (): void => {
    flush();
    lines.push({ segments });
    segments = [];
    column = 0;
  };

  for (let i = 0; i < input.length; i++) {
    const ch = input[i];

    if (ch === "\n") {
      endLine();
      continue;
    }

    if (ch === "\r") {
      const next = input[i + 1];
      if (next === undefined || next === "\n") continue;
      buf = "";
      segments = [];
      column = 0;
      continue;
    }

    if (ch === "\t") {
      const width = TAB_WIDTH - (column % TAB_WIDTH);
      buf += " ".repeat(width);
      column += width;
      continue;
    }

    if (ch === "\x1b") {
      const next = input[i + 1];

      if (next === "[") {
        // CSI: parameter bytes, then a final byte in @-~. Private-marker
        // bytes (<=>?) are part of the parameter field, so a DEC-private
        // sequence is consumed whole and never leaks as text.
        let j = i + 2;
        while (j < input.length && !(input[j] >= "@" && input[j] <= "~")) j++;
        const final = input[j];
        const params = input.slice(i + 2, j);
        // Only a plain `m` is SGR. A private-marker `m` (e.g. `ESC[?...m`) is
        // a different sequence that happens to share the final byte.
        if (final === "m" && !/[<=>?]/.test(params)) {
          flush();
          applySgr(state, params);
        }
        i = j < input.length ? j : input.length;
        continue;
      }

      if (next === "]") {
        // OSC: terminated by BEL or by ST (ESC \).
        let j = i + 2;
        while (j < input.length) {
          if (input[j] === "\x07") break;
          if (input[j] === "\x1b" && input[j + 1] === "\\") break;
          j++;
        }
        i = input[j] === "\x07" ? j : j + 1;
        continue;
      }

      // Any other escape: ESC, then zero or more intermediate bytes (0x20-0x2F),
      // then one final byte (0x30-0x7E). Counting a fixed two bytes would leak
      // the final byte of a three-byte form such as the charset designator
      // `ESC ( B` as visible text.
      let j = i + 1;
      while (j < input.length && input[j] >= "\x20" && input[j] <= "\x2f") j++;
      i = j;
      continue;
    }

    // A control character with no meaning here would render as a stray glyph;
    // drop it rather than showing it.
    if (ch < " " || ch === "\x7f") continue;

    buf += ch;
    column++;
  }

  endLine();
  return lines;
}

/// <reference types="vite/client" />
import { describe, it, expect } from "vitest";
import { Terminal, type IBufferLine } from "@xterm/xterm";
import { parseAnsi, ANSI_PALETTE, type AnsiSegment } from "../src/ansi";

// Equivalence gate for the renderer swap: does the hand-rolled SGR parser read
// the same bytes the same way xterm.js does?
//
// The unit tests in ansi.test.ts only prove the parser agrees with itself. This
// drives xterm's real buffer over captured pane output, folds its cells back
// into style runs, and compares them against parseAnsi's segments — so any
// colour, attribute or escape-handling divergence surfaces as a concrete
// mismatch instead of as a rendering surprise after the swap.
//
// Fixtures are real pane captures and are not committed (they contain live work
// content), so this suite skips when the directory is absent or empty. Point it
// at a corpus with ANSI_FIXTURE_DIR, or drop *.ans files in the default path.

// Loaded through Vite's own glob rather than node's fs so the suite needs no
// node type definitions, which this package deliberately does not carry.
const FIXTURES = import.meta.glob("../../plans/*-terminal-render-benchmark/fixtures/*.ans", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

interface Style {
  fg?: string;
  bg?: string;
  bold?: true;
  dim?: true;
  italic?: true;
  underline?: true;
  strike?: true;
}

interface Run {
  text: string;
  style: Style;
}

const BASE_16 = [
  ANSI_PALETTE.black, ANSI_PALETTE.red, ANSI_PALETTE.green, ANSI_PALETTE.yellow,
  ANSI_PALETTE.blue, ANSI_PALETTE.magenta, ANSI_PALETTE.cyan, ANSI_PALETTE.white,
  ANSI_PALETTE.brightBlack, ANSI_PALETTE.brightRed, ANSI_PALETTE.brightGreen,
  ANSI_PALETTE.brightYellow, ANSI_PALETTE.brightBlue, ANSI_PALETTE.brightMagenta,
  ANSI_PALETTE.brightCyan, ANSI_PALETTE.brightWhite,
];

/** Resolve an xterm palette index the same way the parser does. */
function paletteColor(n: number): string {
  if (n < 16) return BASE_16[n];
  if (n < 232) {
    const i = n - 16;
    const steps = [0, 95, 135, 175, 215, 255];
    const c = (v: number) => v.toString(16).padStart(2, "0");
    return `#${c(steps[Math.floor(i / 36) % 6])}${c(steps[Math.floor(i / 6) % 6])}${c(steps[i % 6])}`;
  }
  const l = 8 + (n - 232) * 10;
  return `#${l.toString(16).padStart(2, "0").repeat(3)}`;
}

const rgbHex = (v: number) => `#${v.toString(16).padStart(6, "0")}`;

/**
 * Fold one xterm buffer row into the shape parseAnsi produces: runs of text
 * sharing a style, with the row's trailing default-styled padding dropped —
 * herdr trims line ends, so the parser never sees that padding.
 */
function runsFromRow(row: IBufferLine, cols: number): Run[] {
  const cells: Run[] = [];
  const cell = row.getCell(0)!;

  for (let x = 0; x < cols; x++) {
    row.getCell(x, cell);
    if (cell.getWidth() === 0) continue; // spacer half of a wide glyph

    const fgSet = !cell.isFgDefault();
    const bgSet = !cell.isBgDefault();
    const fgColor = cell.isFgRGB() ? rgbHex(cell.getFgColor()) : paletteColor(cell.getFgColor());
    const bgColor = cell.isBgRGB() ? rgbHex(cell.getBgColor()) : paletteColor(cell.getBgColor());

    // Inverse swaps the two, each side falling back to the viewport colour —
    // the same rule the parser applies.
    const inverse = cell.isInverse();
    const fg = inverse ? (bgSet ? bgColor : ANSI_PALETTE.background) : fgSet ? fgColor : undefined;
    const bg = inverse ? (fgSet ? fgColor : ANSI_PALETTE.foreground) : bgSet ? bgColor : undefined;

    const style: Style = {};
    if (fg) style.fg = fg;
    if (bg) style.bg = bg;
    if (cell.isBold()) style.bold = true;
    if (cell.isDim()) style.dim = true;
    if (cell.isItalic()) style.italic = true;
    if (cell.isUnderline()) style.underline = true;
    if (cell.isStrikethrough()) style.strike = true;

    const chars = cell.getChars();
    cells.push({ text: chars === "" ? " " : chars, style });
  }

  const key = (s: Style) => JSON.stringify(s);
  const runs: Run[] = [];
  for (const c of cells) {
    const prev = runs[runs.length - 1];
    if (prev && key(prev.style) === key(c.style)) prev.text += c.text;
    else runs.push({ text: c.text, style: { ...c.style } });
  }
  return trimTrailingBlank(runs);
}

/**
 * Drop trailing unstyled whitespace. xterm pads every row out to `cols`, so its
 * blank tail is grid padding rather than content and has no counterpart in the
 * parser's output. Applied to both sides so the comparison stays symmetric —
 * the parser itself must not discard what it was given.
 */
function trimTrailingBlank(runs: Run[]): Run[] {
  const out = runs.slice();
  while (out.length) {
    const last = out[out.length - 1];
    if (Object.keys(last.style).length !== 0) break;
    const trimmed = last.text.replace(/\s+$/, "");
    if (trimmed === last.text) break;
    if (trimmed === "") out.pop();
    else {
      out[out.length - 1] = { text: trimmed, style: last.style };
      break;
    }
  }
  return out;
}

function runsFromSegments(segments: AnsiSegment[]): Run[] {
  const runs = segments
    .filter((s) => s.text !== "")
    .map((s) => {
      const style: Style = {};
      if (s.fg) style.fg = s.fg;
      if (s.bg) style.bg = s.bg;
      if (s.bold) style.bold = true;
      if (s.dim) style.dim = true;
      if (s.italic) style.italic = true;
      if (s.underline) style.underline = true;
      if (s.strike) style.strike = true;
      return { text: s.text, style };
    });
  return trimTrailingBlank(runs);
}

const visibleWidth = (s: string) => s.replace(/\x1b\[[0-9;:?]*[a-zA-Z]/g, "").length;

const files = Object.keys(FIXTURES).sort();

describe.skipIf(files.length === 0)("parseAnsi matches xterm on real pane captures", () => {
  for (const file of files) {
    it(`agrees with xterm for ${file.split("/").pop()}`, async () => {
      const text = FIXTURES[file];
      const logical = text.split("\n");
      // Wide enough that xterm never reflows: a wrap would shift cells and the
      // comparison would be measuring xterm's grid, not the parse.
      const cols = Math.max(20, ...logical.map(visibleWidth)) + 2;

      const term = new Terminal({ cols, rows: logical.length + 2, scrollback: 0 });
      try {
        // xterm parses written data asynchronously; reading the buffer before
        // its callback fires sees an empty grid.
        await new Promise<void>((resolve) => term.write(text.replace(/\n/g, "\r\n"), resolve));

        const mine = parseAnsi(text);
        const buf = term.buffer.active;
        const diffs: string[] = [];

        for (let y = 0; y < mine.length; y++) {
          const row = buf.getLine(y);
          if (!row) break;
          const expected = runsFromRow(row, cols);
          const actual = runsFromSegments(mine[y].segments);
          if (JSON.stringify(expected) === JSON.stringify(actual)) continue;

          // Report only the first run that differs. A whole-line dump gets
          // truncated by the runner and buries the one thing that matters.
          const n = Math.max(expected.length, actual.length);
          let at = 0;
          while (at < n && JSON.stringify(expected[at]) === JSON.stringify(actual[at])) at++;
          diffs.push(
            `L${y} run ${at}/${n}: xterm=${JSON.stringify(expected[at])} parser=${JSON.stringify(actual[at])}`,
          );
        }

        expect(diffs.slice(0, 2).join(" || ").slice(0, 600), `${diffs.length} line(s) differ`).toBe("");
      } finally {
        term.dispose();
      }
    });
  }
});

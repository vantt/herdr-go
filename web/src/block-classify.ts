// Decides, per block of lines, whether wrapping would destroy information.
//
// The question is not "is this a table" — tables are only one of the things
// wrapping ruins, alongside trees, diffs, box-drawn frames and ASCII art. It is
// whether the program chose this line's shape. If it did, that shape is data
// and the reader pans; if it did not, the line is continuous text and wrapping
// it is a favour.
//
// Being wrong is asymmetric, and with no UI control to correct it that
// asymmetry is the design:
//
//   structured wrongly wrapped -> alignment lost, unreadable, unrecoverable
//   prose wrongly panned       -> exactly the behaviour that shipped before
//
// So `pan` is the default and `wrap` is the path that has to prove itself. A
// classifier that hesitates falls back to what the operator already lives with.

import type { StyledLine } from "./ansi";

export type WrapMode = "wrap" | "pan";

export interface Block {
  /** Index of the block's first line in the screen. */
  start: number;
  /** Lines belonging to the block, in order. */
  lines: StyledLine[];
  mode: WrapMode;
}

/**
 * How many consecutive lines a column has to stay blank across before it counts
 * as a gutter. Two lines align by coincidence often enough; four is where
 * alignment starts meaning someone laid it out.
 */
const GUTTER_MIN_LINES = 4;
/** Column gutters needed before a block is treated as laid out. */
const GUTTER_MIN_COUNT = 2;
/** Shortest line worth looking for gutters in. */
const GUTTER_MIN_WIDTH = 8;
/** Share of a block's lines carrying box-drawing before it counts as drawn. */
const BOX_LINE_RATIO = 0.3;

// Box drawing (U+2500-257F) and block elements (U+2580-259F) only — the
// characters programs use to *draw* something, including the branch glyphs of
// `cargo tree` and `git log --graph`.
//
// Deliberately excludes arrows and geometric shapes, which an agent scatters
// through ordinary prose as bullets and status marks. Counting those made a
// three-line paragraph containing one "→" read as a drawn layout and pan.
const BOX_CHARS = /[─-▟]/;

/** Shortest run of box-drawing characters that counts as a drawn rule rather than a stray corner glyph. */
const RULE_MIN_LENGTH = 8;

/**
 * A line that is nothing but box-drawing characters, end to end: a rule the
 * program drew on purpose. An input box keeps exactly two of these — top and
 * bottom — no matter how much text grows between them, so `BOX_LINE_RATIO`
 * alone cannot carry it: a long draft or banner dilutes the ratio below
 * threshold and the whole box, rules included, wraps. Counting a rule's mere
 * presence, not its share of the block, is what a growing block cannot dilute.
 */
function isFramingRule(text: string): boolean {
  const trimmed = text.trim();
  if (trimmed.length < RULE_MIN_LENGTH) return false;
  return [...trimmed].every((ch) => BOX_CHARS.test(ch));
}

/** A live selection cursor immediately in front of a numbered option. */
const MENU_CURSOR_ITEM = /^\s*❯\s*\d{1,2}[.)]\s/;
/** A numbered menu item, cursor optional — matches every option, selected or not. */
const MENU_ITEM = /^\s*(❯\s*)?\d{1,2}[.)]\s/;
/** Siblings a single numbered line needs before it reads as a menu rather than a stray reference. */
const MENU_MIN_ITEMS = 2;

const plainText = (line: StyledLine): string => line.segments.map((s) => s.text).join("");

/**
 * Count column positions that are blank in every line of the group, merging
 * adjacent blank columns into one gutter.
 *
 * This is the signal that catches the tables no character announces — `ls -la`,
 * `ps aux`, `df -h` pad their columns with spaces and draw nothing at all.
 */
export function stableGutters(texts: string[]): number {
  const rows = texts.filter((t) => t.trim() !== "");
  if (rows.length < GUTTER_MIN_LINES) return 0;

  const width = Math.min(...rows.map((r) => r.length));
  if (width < GUTTER_MIN_WIDTH) return 0;

  let count = 0;
  let inRun = false;
  for (let col = 0; col < width; col++) {
    const blank = rows.every((r) => r[col] === " ");
    if (blank && !inRun) {
      count++;
      inRun = true;
    } else if (!blank) {
      inRun = false;
    }
  }
  return count;
}

/** Whether a run of lines was laid out rather than written as continuous text. */
export function looksStructured(texts: string[]): boolean {
  const content = texts.filter((t) => t.trim() !== "");
  if (content.length === 0) return false;

  if (stableGutters(texts) >= GUTTER_MIN_COUNT) return true;

  if (content.some(isFramingRule)) return true;

  const boxed = content.filter((t) => BOX_CHARS.test(t)).length;
  if (boxed / content.length >= BOX_LINE_RATIO) return true;

  // Markdown-style tables: pipes holding the same columns line after line.
  const pipeColumns = content.map(
    (t) =>
      t
        .split("")
        .map((ch, i) => (ch === "|" ? i : -1))
        .filter((i) => i >= 0)
        .join(","),
  );
  if (
    content.length >= 2 &&
    pipeColumns[0] !== "" &&
    pipeColumns.every((c) => c === pipeColumns[0]) &&
    (pipeColumns[0].match(/,/g) ?? []).length >= 1
  ) {
    return true;
  }

  // Interactive choice menus: an agent narrating "1. do this" in prose never
  // draws a selection cursor, so seeing one in front of a number — alongside
  // at least one more numbered sibling — is Claude Code's own menu, not text.
  // Descriptions under each option vary in length by design, which is exactly
  // what defeats the gutter check above; the cursor is the signal that survives.
  const cursorItems = content.filter((t) => MENU_CURSOR_ITEM.test(t)).length;
  const menuItems = content.filter((t) => MENU_ITEM.test(t)).length;
  if (cursorItems >= 1 && menuItems >= MENU_MIN_ITEMS) return true;

  return false;
}

/**
 * Split a screen into blocks at blank lines and classify each one.
 *
 * Blank lines are the boundary because they are the one thing that does not
 * move as content grows: a boundary derived from the content itself would shift
 * between polls, and a block that re-classifies under the reader is worse than
 * one classified conservatively. A blank line becomes its own block so it never
 * joins two neighbours into one verdict.
 *
 * The verdict depends only on the block's own text — never on the viewport —
 * so rotating the phone or raising the keyboard cannot re-classify anything.
 * Width decides only whether a decision has any visible effect.
 */
export function classifyBlocks(lines: StyledLine[]): Block[] {
  const blocks: Block[] = [];
  let current: StyledLine[] = [];
  let start = 0;

  const flush = (): void => {
    if (current.length === 0) return;
    blocks.push({
      start,
      lines: current,
      mode: looksStructured(current.map(plainText)) ? "pan" : "wrap",
    });
    current = [];
  };

  lines.forEach((line, index) => {
    if (plainText(line).trim() === "") {
      flush();
      // A blank line has no width, so its mode never shows; `pan` keeps the
      // type total without implying a decision was made.
      blocks.push({ start: index, lines: [line], mode: "pan" });
      start = index + 1;
      return;
    }
    if (current.length === 0) start = index;
    current.push(line);
  });
  flush();

  return blocks;
}

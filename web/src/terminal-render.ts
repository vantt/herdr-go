// Renders parsed pane output as plain DOM.
//
// Replaces xterm.js in the terminal view. Nothing here emulates a terminal:
// herdr sends an already-rendered grid as SGR-styled text, so the job is to
// turn style runs into elements and let the browser lay them out.
//
// Text always reaches the DOM through text nodes, never innerHTML. Pane output
// is untrusted, and keeping styling as the only thing derived from it is what
// keeps this off the XSS surface.

import { parseAnsi, ANSI_PALETTE, type AnsiSegment, type StyledLine } from "./ansi";
import { classifyBlocks } from "./block-classify";
import { findLinks } from "./linkify";

export interface TerminalRender {
  /**
   * The element holding the rendered screen. Exposed because the pinch gesture
   * previews its zoom with a CSS transform on it — the same role xterm's own
   * `.element` played.
   */
  readonly element: HTMLPreElement;
  /** Replace the screen with freshly parsed output. */
  write(text: string): void;
  setFontSize(px: number): void;
  dispose(): void;
}

/** One run of text after links have been split out of the style runs. */
interface Piece {
  text: string;
  style: AnsiSegment;
  href?: string;
}

/**
 * Split a line's style runs at link boundaries.
 *
 * A URL routinely spans several runs (a coloured scheme, a plain host) and a
 * run routinely holds more than one URL, so neither list can be walked alone.
 * Both are ordered over the same coordinate space — the line's visible text —
 * so a single pass over the runs, consulting the link ranges by offset, is
 * enough.
 */
export function splitLineIntoPieces(line: StyledLine): Piece[] {
  const plain = line.segments.map((s) => s.text).join("");
  const links = findLinks(plain);
  if (links.length === 0) {
    return line.segments.map((style) => ({ text: style.text, style }));
  }

  const pieces: Piece[] = [];
  let offset = 0;

  for (const segment of line.segments) {
    const segStart = offset;
    const segEnd = offset + segment.text.length;
    offset = segEnd;

    let cursor = segStart;
    for (const link of links) {
      if (link.end <= cursor || link.start >= segEnd) continue;

      const overlapStart = Math.max(cursor, link.start);
      const overlapEnd = Math.min(segEnd, link.end);

      if (overlapStart > cursor) {
        pieces.push({
          text: segment.text.slice(cursor - segStart, overlapStart - segStart),
          style: segment,
        });
      }
      pieces.push({
        text: segment.text.slice(overlapStart - segStart, overlapEnd - segStart),
        style: segment,
        href: link.href,
      });
      cursor = overlapEnd;
    }

    if (cursor < segEnd) {
      pieces.push({ text: segment.text.slice(cursor - segStart), style: segment });
    }
  }

  return pieces;
}

function isPlain(style: AnsiSegment): boolean {
  return (
    style.fg === undefined &&
    style.bg === undefined &&
    !style.bold &&
    !style.dim &&
    !style.italic &&
    !style.underline &&
    !style.strike
  );
}

function applyStyle(el: HTMLElement, style: AnsiSegment): void {
  if (style.fg) el.style.color = style.fg;
  if (style.bg) el.style.backgroundColor = style.bg;
  if (style.bold) el.style.fontWeight = "600";
  if (style.italic) el.style.fontStyle = "italic";
  // Terminals render dim as a reduced-intensity version of the same colour;
  // opacity is the closest equivalent that works for any colour, including the
  // default foreground.
  if (style.dim) el.style.opacity = "0.6";
  const decoration = [style.underline ? "underline" : "", style.strike ? "line-through" : ""]
    .filter(Boolean)
    .join(" ");
  if (decoration) el.style.textDecoration = decoration;
}

function renderPiece(piece: Piece): Node {
  if (piece.href !== undefined) {
    const anchor = document.createElement("a");
    anchor.href = piece.href;
    anchor.target = "_blank";
    // Opener access would hand the pane's output control of this tab.
    anchor.rel = "noopener noreferrer";
    anchor.textContent = piece.text;
    applyStyle(anchor, piece.style);
    return anchor;
  }
  // An unstyled run needs no element of its own. Most of a screen is unstyled,
  // so this keeps the node count down where it matters.
  if (isPlain(piece.style)) return document.createTextNode(piece.text);

  const span = document.createElement("span");
  span.textContent = piece.text;
  applyStyle(span, piece.style);
  return span;
}

/**
 * Build the DOM for a whole screen, one element per classified block.
 *
 * A block that keeps its layout needs a horizontal scroller of its own, so all
 * of its rows pan together and stay in column — which forces it to be a
 * block-level box. That costs something subtle: `textContent` concatenates
 * across block boundaries without a newline, so a copied selection would run
 * the last line of one block into the first of the next.
 *
 * The zero-height separator spans put those newlines back. They carry a "\n" so
 * copy is faithful, and collapse to nothing visually because the block boxes
 * have already broken the line themselves.
 */
export function buildScreenFragment(lines: StyledLine[]): DocumentFragment {
  const fragment = document.createDocumentFragment();

  classifyBlocks(lines).forEach((block, blockIndex) => {
    if (blockIndex > 0) {
      const separator = document.createElement("span");
      separator.className = "term-nl";
      separator.textContent = "\n";
      fragment.appendChild(separator);
    }

    const el = document.createElement("div");
    el.className = `term-block is-${block.mode}`;
    block.lines.forEach((line, lineIndex) => {
      if (lineIndex > 0) el.appendChild(document.createTextNode("\n"));
      for (const piece of splitLineIntoPieces(line)) {
        if (piece.text === "") continue;
        el.appendChild(renderPiece(piece));
      }
    });
    fragment.appendChild(el);
  });

  return fragment;
}

/**
 * Mount a screen renderer inside `mount`.
 *
 * Lines are separated by newline text nodes inside one `<pre>` rather than
 * wrapped in per-line elements: `white-space: pre` then gives blank lines their
 * height for free, which per-line blocks would not.
 */
export function createTerminalRender(mount: HTMLElement, fontSize: number): TerminalRender {
  const element = document.createElement("pre");
  element.className = "term-screen";
  element.style.fontSize = `${fontSize}px`;
  element.style.color = ANSI_PALETTE.foreground;
  mount.appendChild(element);

  return {
    element,
    write(text: string): void {
      element.replaceChildren(buildScreenFragment(parseAnsi(text)));
    },
    setFontSize(px: number): void {
      element.style.fontSize = `${px}px`;
    },
    dispose(): void {
      element.remove();
    },
  };
}

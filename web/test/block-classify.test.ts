import { describe, it, expect } from "vitest";
import { classifyBlocks, looksStructured, stableGutters, type WrapMode } from "../src/block-classify";
import { parseAnsi } from "../src/ansi";

/** Classify raw screen text, as the renderer will. */
function modes(text: string): WrapMode[] {
  return classifyBlocks(parseAnsi(text)).map((b) => b.mode);
}

/** The single verdict for text with no blank lines in it. */
function verdict(text: string): WrapMode {
  const found = modes(text);
  expect(found).toHaveLength(1);
  return found[0];
}

describe("stableGutters", () => {
  it("finds the padded columns of a whitespace-aligned table", () => {
    const rows = [
      "alice    30   engineer",
      "bob      41   designer",
      "carolina 25   writer",
      "dan      37   manager",
    ];
    expect(stableGutters(rows)).toBeGreaterThanOrEqual(2);
  });

  it("finds none in prose, where spaces never land in the same column", () => {
    const rows = [
      "the parser reads a grid dump and",
      "turns style runs into elements so",
      "the browser can lay them out for us",
      "without emulating a terminal at all",
    ];
    expect(stableGutters(rows)).toBe(0);
  });

  it("ignores a group too short to mean anything", () => {
    expect(stableGutters(["a   b", "c   d"])).toBe(0);
  });

  it("ignores very narrow lines", () => {
    expect(stableGutters(["a b", "c d", "e f", "g h"])).toBe(0);
  });
});

describe("looksStructured", () => {
  it("accepts a box-drawn frame", () => {
    expect(
      looksStructured(["┌──────────┐", "│ hello    │", "└──────────┘"]),
    ).toBe(true);
  });

  it("accepts a tree rendering", () => {
    expect(
      looksStructured([
        "herdr-go v0.1.12",
        "├── axum v0.7.0",
        "│   └── tower v0.5.0",
        "└── serde v1.0.0",
      ]),
    ).toBe(true);
  });

  it("accepts a markdown table whose pipes hold their columns", () => {
    expect(
      looksStructured(["| a  | b  |", "| 11 | 22 |", "| 33 | 44 |"]),
    ).toBe(true);
  });

  it("rejects prose", () => {
    expect(
      looksStructured([
        "This is an ordinary paragraph of output from an agent,",
        "long enough that a phone would have to pan to read it,",
        "and carrying nothing that any program laid out in columns.",
      ]),
    ).toBe(false);
  });

  it("rejects a single stray box character in prose", () => {
    // One glyph is not a drawn layout; the ratio guard is what stops a
    // bullet or arrow in a sentence from panning the whole paragraph.
    expect(
      looksStructured([
        "the check passed → moving on to the next step now",
        "nothing here is laid out in columns of any kind",
        "just sentences that happen to contain an arrow",
      ]),
    ).toBe(false);
  });

  it("rejects an empty block", () => {
    expect(looksStructured([])).toBe(false);
    expect(looksStructured(["", "  "])).toBe(false);
  });

  it("accepts a footer whose rules are diluted by the content between them", () => {
    // Real Claude Code footer: two full-width rules bracket a draft the input
    // box already soft-wrapped. Growing the draft dilutes the rule-to-content
    // ratio below BOX_LINE_RATIO, so the ratio check alone loses the box; a
    // rule's mere presence is what the growth cannot dilute.
    expect(
      looksStructured([
        "─────────────────────────────────────────────",
        "❯ this stranded draft is long enough that Claude",
        "  soft-wraps it onto several lines inside the box",
        "  which is exactly the case that used to break",
        "─────────────────────────────────────────────",
        "  [Opus 4.8] ~/playground/some-project",
      ]),
    ).toBe(true);
  });

  it("accepts a numbered choice menu even though descriptions vary in length", () => {
    expect(
      looksStructured([
        "❯ 1. Red",
        "     A warm, high-energy accent colour.",
        "  2. Green",
        "     A calm, natural theme.",
        "  3. Blue",
        "     A cool, professional look.",
      ]),
    ).toBe(true);
  });

  it("accepts a yes/no confirmation menu", () => {
    expect(
      looksStructured([
        "Do you want to proceed?",
        "❯ 1. Yes",
        "  2. Yes, and don't ask again",
        "  3. No",
      ]),
    ).toBe(true);
  });

  it("still rejects an ordinary numbered list with no live cursor", () => {
    // An agent narrating steps reuses the same "N. " shape but never draws a
    // selection cursor in front of it — that glyph is what tells them apart.
    expect(
      looksStructured([
        "1. Install the dependencies",
        "2. Run the build",
        "3. Deploy to staging",
      ]),
    ).toBe(false);
  });

  it("accepts a 'Review your answers' question/answer summary", () => {
    expect(
      looksStructured([
        "● Which focus area should we work on?",
        "  → UI",
        "● What scope should this work have?",
        "  → Medium",
        "● How should we approach the work?",
        "  → Plan first",
      ]),
    ).toBe(true);
  });

  it("still rejects a single stray arrow under a bulleted line", () => {
    // One pair is a coincidence; the summary shape only shows itself on repeat.
    expect(
      looksStructured(["● Deploying now", "  → this may take a minute"]),
    ).toBe(false);
  });
});

describe("classifyBlocks", () => {
  it("wraps prose", () => {
    expect(
      verdict(
        [
          "This is an ordinary paragraph of output from an agent,",
          "long enough that a phone would have to pan to read it,",
          "and carrying nothing that any program laid out in columns.",
        ].join("\n"),
      ),
    ).toBe("wrap");
  });

  it("pans a whitespace-aligned table that draws no characters at all", () => {
    // The case no glyph announces: `ls -la`, `ps aux`, `df -h`.
    expect(
      verdict(
        [
          "alice    30   engineer",
          "bob      41   designer",
          "carolina 25   writer",
          "dan      37   manager",
        ].join("\n"),
      ),
    ).toBe("pan");
  });

  it("pans a box-drawn frame", () => {
    expect(verdict(["┌────────┐", "│ hello  │", "└────────┘"].join("\n"))).toBe("pan");
  });

  it("splits at blank lines so a table and its surrounding prose decide separately", () => {
    const text = [
      "Here is what the check produced, in full detail below:",
      "",
      "alice    30   engineer",
      "bob      41   designer",
      "carolina 25   writer",
      "dan      37   manager",
      "",
      "Everything above looks fine and nothing needs your attention.",
    ].join("\n");

    // prose, blank, table, blank, prose
    expect(modes(text)).toEqual(["wrap", "pan", "pan", "pan", "wrap"]);
  });

  it("keeps line order and coverage across blocks", () => {
    const lines = ["a", "", "b", "c", ""];
    const blocks = classifyBlocks(parseAnsi(lines.join("\n")));
    expect(blocks.reduce((n, b) => n + b.lines.length, 0)).toBe(lines.length);
    expect(blocks.map((b) => b.start)).toEqual([0, 1, 2, 4]);
  });

  it("ignores ANSI styling when deciding", () => {
    // Colour is not layout: the same table decides the same way painted or not.
    const plain = ["alice    30   engineer", "bob      41   designer", "carolina 25   writer", "dan      37   manager"];
    const painted = plain.map((l) => `\x1b[32m${l}\x1b[0m`);
    expect(verdict(plain.join("\n"))).toBe(verdict(painted.join("\n")));
  });

  describe("stability across polls", () => {
    it("does not change a block's verdict when later content is appended", () => {
      // Content grows every 1.5s. A verdict that flipped as lines arrived would
      // re-flow the block under a reader mid-drag.
      const table = ["alice    30   engineer", "bob      41   designer", "carolina 25   writer", "dan      37   manager"];
      const first = classifyBlocks(parseAnsi(table.join("\n")));
      const grown = classifyBlocks(parseAnsi([...table, "", "later prose arrives here"].join("\n")));
      expect(grown[0].mode).toBe(first[0].mode);
    });

    it("gives the same verdict no matter how wide the viewport is", () => {
      // The classifier takes no width at all — this test exists to keep it that
      // way, so rotating the phone or raising the keyboard cannot re-classify.
      const text = ["alice    30   engineer", "bob      41   designer", "carolina 25   writer", "dan      37   manager"].join("\n");
      expect(classifyBlocks(parseAnsi(text))).toEqual(classifyBlocks(parseAnsi(text)));
    });
  });

  describe("the bias that matters", () => {
    it("never wraps anything it recognises as laid out", () => {
      const structured = [
        ["┌───┐", "│ a │", "└───┘"],
        ["├── axum v0.7.0", "│   └── tower v0.5.0", "└── serde v1.0.0", "    └── x v1.0"],
        ["| a  | b  |", "| 11 | 22 |", "| 33 | 44 |"],
        ["alice    30   engineer", "bob      41   designer", "carolina 25   writer", "dan      37   manager"],
      ];
      for (const block of structured) {
        expect(verdict(block.join("\n"))).toBe("pan");
      }
    });
  });
});

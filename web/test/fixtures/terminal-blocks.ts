// Drop-in target: /home/vantt/projects/herdr-gateway/web/test/fixtures/terminal-blocks.ts
//
// A corpus of the block shapes an operator actually reads through a pane, each
// labelled with the treatment `docs/specs/terminal-detail.md` R22/R23 says it
// should get.
//
// The classifier's own unit tests (`web/test/block-classify.test.ts`) prove
// that each detector fires on the shape it was written for. They cannot answer
// the question `terminal-detail.md` leaves open under Open Gaps — "Real miss
// rates have not been measured against a corpus of the output the operator
// actually reads" — because every one of those tests was written *from* a
// detector. This file goes the other direction: shapes chosen first, verdicts
// measured afterwards.
//
// Labelling rule, applied literally from the spec rather than from taste:
//
//   pan  (R22) — the program chose this shape: columns, box drawing, or any
//                other alignment that carries meaning between lines.
//   wrap (R23) — continuous text; the line breaks are incidental.
//
// `tier` separates the two kinds of claim this corpus makes:
//
//   core     — a colleague would not argue with the label. Wrapping these
//              destroys something the reader cannot reconstruct.
//   extended — the label is a judgment call, defensible but arguable. Scored
//              and reported separately so a disputed label can never move the
//              number the operator's decision is actually about.
//
// Every fixture is transcribed by hand into the shape the real program emits,
// not captured off a live pane. That is this corpus's own limitation: the
// column positions are faithful but reconstructed. Capturing the same shapes
// straight off a real pane is the follow-up.

export type Expected = "pan" | "wrap";

export interface BlockFixture {
  /** Stable id, used as the confusion matrix's row label. */
  id: string;
  /** The program whose output this is. */
  source: string;
  tier: "core" | "extended";
  expected: Expected;
  /** Why that label, in the spec's own terms. */
  why: string;
  /** The block exactly as it reaches the classifier. No blank lines — a blank line would split it. */
  lines: string[];
}

export const BLOCK_CORPUS: BlockFixture[] = [
  // ── Laid out: wrapping destroys alignment ────────────────────────────────

  {
    id: "ls-la-body-only",
    source: "ls -la, body rows only",
    tier: "core",
    expected: "pan",
    why: "R22: eight space-padded columns, none of which draws a character.",
    lines: [
      "drwxr-xr-x  6 vantt vantt  4096 Sep  7 10:12 .",
      "-rw-r--r--  1 vantt vantt  1204 Sep  7 09:58 package.json",
      "-rw-r--r--  1 vantt vantt   318 Aug 30 21:07 tsconfig.json",
      "-rw-r--r--  1 vantt vantt  9932 Sep  1 08:11 pnpm-lock.yaml",
    ],
  },

  {
    id: "ls-la-with-total-header",
    source: "ls -la, as it is actually printed",
    tier: "core",
    expected: "pan",
    why:
      "Identical to ls-la-body-only plus the `total N` line ls always prints. The " +
      "columns below it are unchanged, so the label is unchanged. Paired with the " +
      "fixture above on purpose: the only variable between them is that header.",
    lines: [
      "total 48",
      "drwxr-xr-x  6 vantt vantt  4096 Sep  7 10:12 .",
      "-rw-r--r--  1 vantt vantt  1204 Sep  7 09:58 package.json",
      "-rw-r--r--  1 vantt vantt   318 Aug 30 21:07 tsconfig.json",
      "-rw-r--r--  1 vantt vantt  9932 Sep  1 08:11 pnpm-lock.yaml",
    ],
  },

  {
    id: "df-h",
    source: "df -h",
    tier: "core",
    expected: "pan",
    why: "R22: right-aligned size columns under a header row.",
    lines: [
      "Filesystem      Size  Used  Avail  Use%  Mounted",
      "/dev/nvme0n1p2  467G  213G   231G   48%  /",
      "/dev/nvme0n1p1  511M  6.1M   505M    2%  /boot",
      "tmpfs           7.7G   34M   7.6G    1%  /dev/shm",
    ],
  },

  {
    id: "vitest-run-summary",
    source: "vitest",
    tier: "core",
    expected: "pan",
    why: "R22: right-aligned labels against a left-aligned value column.",
    lines: [
      " Test Files  3 passed (3)",
      "      Tests  41 passed (41)",
      "   Start at  10:54:12",
      "   Duration  1.42s",
    ],
  },

  {
    id: "cargo-tree",
    source: "cargo tree",
    tier: "core",
    expected: "pan",
    why: "R22: box-drawing branch glyphs encode depth; a wrapped continuation line invents a branch.",
    lines: [
      "herdr-go v0.1.20",
      "├── axum v0.7.9",
      "│   ├── hyper v1.5.0",
      "│   └── tower v0.5.1",
      "└── serde v1.0.215",
    ],
  },

  {
    id: "claude-footer-box",
    source: "Claude Code input box with a soft-wrapped draft in it",
    tier: "core",
    expected: "pan",
    why: "R22: two drawn rules bracket the box; wrapping either rule breaks the frame.",
    lines: [
      "─────────────────────────────────────────────",
      "❯ this stranded draft is long enough that Claude",
      "  soft-wraps it onto several lines inside the box",
      "─────────────────────────────────────────────",
      "  [Opus 5] ~/projects/herdr-gateway",
    ],
  },

  {
    id: "claude-tool-panel-box",
    source: "Claude Code tool-call panel",
    tier: "core",
    expected: "pan",
    why: "R22: a rounded box drawn around the tool invocation.",
    lines: [
      "╭─────────────────────────────────────────╮",
      "│ Bash(npm test)                          │",
      "╰─────────────────────────────────────────╯",
    ],
  },

  {
    id: "claude-choice-menu",
    source: "Claude Code numbered choice prompt",
    tier: "core",
    expected: "pan",
    why: "R22: a live selection cursor in front of numbered options; the cursor column is the layout.",
    lines: [
      "Do you want to proceed?",
      "❯ 1. Yes",
      "  2. Yes, and don't ask again",
      "  3. No",
    ],
  },

  {
    id: "claude-review-your-answers",
    source: "Claude Code 'Review your answers' summary",
    tier: "core",
    expected: "pan",
    why: "R22: each answer sits directly beneath its own question; the pairing is the layout.",
    lines: [
      "● Which focus area should we work on?",
      "  → UI",
      "● What scope should this work have?",
      "  → Medium",
      "● How should we approach the work?",
      "  → Plan first",
    ],
  },

  {
    id: "markdown-table-padded",
    source: "an agent writing a column-padded markdown table",
    tier: "core",
    expected: "pan",
    why: "R22: pipes hold the same columns on every row.",
    lines: [
      "| Shape   | Mode | Why    |",
      "| ------- | ---- | ------ |",
      "| table   | pan  | pan    |",
    ],
  },

  {
    id: "git-log-graph",
    source: "git log --graph --oneline",
    tier: "core",
    expected: "pan",
    why:
      "R22: the graph rail is a drawn column — a wrapped continuation line lands under " +
      "the rail and reads as a commit on a branch that does not exist. Note git draws " +
      "this rail in ASCII (* | / \\), not in box-drawing characters, which is worth " +
      "checking against the claim in block-classify.ts's BOX_CHARS comment.",
    lines: [
      "* c1bebce fix(terminal): widen reply guard tail window past the footer",
      "|\\",
      "| * 15b1203 fix(terminal): pan Claude's 'Review your answers' Q&A summary",
      "| * 881fa33 fix(terminal): stop rules and choice menus wrapping under footers",
      "|/",
      "* fa2f1b7 chore: remove remaining .bee agent definitions",
    ],
  },

  {
    id: "git-diff-unified",
    source: "git diff",
    tier: "core",
    expected: "pan",
    why:
      "R22, and named outright in block-classify.ts's own opening comment as something " +
      "wrapping ruins: a wrapped body line loses its +/-/space marker and reads as context.",
    lines: [
      "diff --git a/web/src/block-classify.ts b/web/src/block-classify.ts",
      "index 8a1f2c4..b7e9d01 100644",
      "--- a/web/src/block-classify.ts",
      "+++ b/web/src/block-classify.ts",
      "@@ -131,7 +131,7 @@ export function looksStructured(texts) {",
      '   const content = texts.filter((t) => t.trim() !== "");',
      "-  if (content.length === 0) return false;",
      "+  if (content.length === 0) return true;",
    ],
  },

  {
    id: "tsc-caret-underline",
    source: "tsc --noEmit",
    tier: "core",
    expected: "pan",
    why: "R22: the underline's only meaning is the column it sits in; wrapping moves it under the wrong token.",
    lines: [
      "139   const boxed = content.filter((t) => BOX_CHARS.test(t)).length;",
      "                                          ~~~~~~~~~",
    ],
  },

  // ── Continuous text: wrapping is the favour ──────────────────────────────

  {
    id: "agent-paragraph",
    source: "an agent explaining what it did",
    tier: "core",
    expected: "wrap",
    why: "R23: sentences; the line breaks carry nothing.",
    lines: [
      "This is an ordinary paragraph of output from an agent, long enough",
      "that a phone would have to pan to read it, and carrying nothing that",
      "any program laid out in columns or drew a frame around anywhere.",
    ],
  },

  {
    id: "agent-numbered-narration",
    source: "an agent narrating steps in prose",
    tier: "core",
    expected: "wrap",
    why: "R23: reuses a menu's shape but has no selection cursor — it is a sentence list.",
    lines: [
      "1. Install the dependencies before anything else runs",
      "2. Run the build and wait for the bundle to be written",
      "3. Deploy to staging once the smoke test comes back green",
    ],
  },

  {
    id: "agent-single-arrow-pair",
    source: "an agent using a bullet and an arrow once",
    tier: "core",
    expected: "wrap",
    why: "R23: one question/answer-shaped pair is punctuation, not a laid-out summary.",
    lines: ["● Deploying now", "  → this may take a minute or two to finish"],
  },

  {
    id: "agent-prose-with-one-box-glyph",
    source: "an agent mentioning a drawing glyph mid-sentence",
    tier: "core",
    expected: "wrap",
    why: "R23: a single drawn character in four lines of prose is not a drawn layout.",
    lines: [
      "the tree drawing used │ once in this sentence and no more",
      "nothing here is laid out in columns of any kind whatsoever",
      "just sentences that happen to contain a drawing character",
      "and one more line so the ratio guard has room to be wrong",
    ],
  },

  {
    id: "agent-markdown-bullets",
    source: "an agent writing a bullet list",
    tier: "core",
    expected: "wrap",
    why: "R23: hyphen bullets in front of sentences; nothing aligns between lines.",
    lines: [
      "- The classifier splits at blank lines and judges each block alone.",
      "- A block the program laid out keeps its shape and pans sideways.",
      "- Continuous text is broken to fit the reader's width instead.",
    ],
  },

  {
    id: "agent-single-long-line",
    source: "an agent emitting one long sentence",
    tier: "core",
    expected: "wrap",
    why: "R23: the one case where wrapping is unambiguously the whole point.",
    lines: [
      "The classifier decides per block and never per screen, so a single long sentence sitting alone between two blank lines is judged entirely on its own content and nothing else.",
    ],
  },

  {
    id: "node-stack-trace",
    source: "node, an unhandled rejection",
    tier: "core",
    expected: "wrap",
    why:
      "R23: frames are sentences behind a fixed prefix. Panning them costs the reader a " +
      "sideways drag and nothing else, so a miss here is tolerated, not a defect.",
    lines: [
      "    at looksStructured (/home/vantt/web/src/block-classify.ts:139:9)",
      "    at classifyBlocks (/home/vantt/web/src/block-classify.ts:201:13)",
      "    at flush (/home/vantt/web/src/block-classify.ts:196:5)",
      "    at Array.forEach (<anonymous>)",
    ],
  },

  // ── Extended: defensible labels a colleague could argue with ─────────────

  {
    id: "markdown-table-unpadded",
    source: "an agent writing a markdown table without padding cells",
    tier: "extended",
    expected: "pan",
    why:
      "R22 by structure, though nothing visibly lines up — the pipes still delimit " +
      "columns, and a wrapped row stops being a row. Arguable: with no visual " +
      "alignment to lose, a reader may not notice either treatment.",
    lines: [
      "| Area | Mode | Notes |",
      "| --- | --- | --- |",
      "| tables | pan | gutters |",
      "| prose | wrap | reflow |",
    ],
  },

  {
    id: "pretty-printed-json",
    source: "an agent printing a JSON result",
    tier: "extended",
    expected: "pan",
    why:
      "R22: indentation is the nesting. Arguable: most JSON lines are short enough " +
      "that wrapping never fires, so the miss may cost nothing in practice.",
    lines: [
      "{",
      '  "status": "done",',
      '  "counts": {',
      '    "pan": 14,',
      '    "wrap": 10',
      "  }",
      "}",
    ],
  },

  {
    id: "indented-source-block",
    source: "an agent pasting source into the pane",
    tier: "extended",
    expected: "pan",
    why:
      "R22: indentation is program-chosen shape. Arguable: a reader of wrapped code " +
      "usually still follows it, unlike a wrapped table.",
    lines: [
      "export function looksStructured(texts: string[]): boolean {",
      '  const content = texts.filter((t) => t.trim() !== "");',
      "  if (content.length === 0) return false;",
      "  return true;",
      "}",
    ],
  },

  {
    id: "git-status-short",
    source: "git status --short",
    tier: "extended",
    expected: "pan",
    why:
      "R22: a two-character status column in front of a path. Arguable: the column is " +
      "narrow enough that a wrapped long path stays readable.",
    lines: [
      " M web/src/block-classify.ts",
      "?? web/test/fixtures/terminal-blocks.ts",
      "?? plans/reports/home-cards-260809.png",
      " M docs/specs/terminal-detail.md",
    ],
  },

  {
    id: "figlet-banner",
    source: "figlet / an installer banner",
    tier: "extended",
    expected: "pan",
    why:
      "R22, and named in block-classify.ts's own opening comment (ASCII art). Included " +
      "deliberately with no hand-predicted verdict: whether its space columns line up " +
      "well enough to trip the gutter check is exactly what only a run settles.",
    lines: [
      "  _   _ _____ ____  ____  ____",
      " | | | | ____|  _ \\|  _ \\|  _ \\",
      " | |_| |  _| | |_) | | | | |_) |",
      " |  _  | |___|  _ <| |_| |  _ <",
      " |_| |_|_____|_| \\_\\____/|_| \\_\\",
    ],
  },
];

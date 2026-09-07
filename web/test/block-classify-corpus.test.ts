// Drop-in target: /home/vantt/projects/herdr-gateway/web/test/block-classify-corpus.test.ts
//
// Scores the classifier against the corpus and prints a confusion matrix.
//
// The two errors are not symmetric, and the whole design rests on that
// asymmetry (`block-classify.ts` header; `terminal-detail.md` R25):
//
//   false wrap — a laid-out block was wrapped. Alignment is gone and the
//                reader cannot get it back. There is no control to correct it
//                (R24), so this is the failure the operator lives with.
//   false pan  — continuous text was panned. The reader drags sideways once.
//                This is exactly the behaviour that shipped before the
//                classifier existed, so it costs nothing new.
//
// The gate therefore runs one direction only: false wrap must be zero on the
// core tier. False pan is measured and reported, never asserted on.
//
// The extended tier is scored and printed but not gated: its labels are
// judgment calls (see each fixture's `why`), and a disputed label must never be
// able to move the number the operator's decision is actually about.

import { describe, it, expect } from "vitest";
import { classifyBlocks, type WrapMode } from "../src/block-classify";
import { parseAnsi } from "../src/ansi";
import { BLOCK_CORPUS, type BlockFixture } from "./fixtures/terminal-blocks";

/** The classifier's verdict for one fixture, taken the way the renderer takes it. */
function verdictFor(fixture: BlockFixture): WrapMode {
  const blocks = classifyBlocks(parseAnsi(fixture.lines.join("\n")));
  // A fixture holds no blank lines, so it must arrive as exactly one block. If
  // this ever fails, the fixture is malformed, not the classifier.
  expect(blocks, `${fixture.id} should be a single block`).toHaveLength(1);
  return blocks[0].mode;
}

interface Matrix {
  /** expected pan, got pan — alignment kept. */
  panPan: number;
  /** expected pan, got wrap — the unrecoverable failure. */
  panWrap: string[];
  /** expected wrap, got pan — the tolerated one. */
  wrapPan: string[];
  /** expected wrap, got wrap. */
  wrapWrap: number;
}

function score(fixtures: BlockFixture[]): Matrix {
  const m: Matrix = { panPan: 0, panWrap: [], wrapPan: [], wrapWrap: 0 };
  for (const f of fixtures) {
    const actual = verdictFor(f);
    if (f.expected === "pan") {
      if (actual === "pan") m.panPan++;
      else m.panWrap.push(f.id);
    } else {
      if (actual === "wrap") m.wrapWrap++;
      else m.wrapPan.push(f.id);
    }
  }
  return m;
}

function report(tier: string, m: Matrix): string {
  const pan = m.panPan + m.panWrap.length;
  const wrap = m.wrapWrap + m.wrapPan.length;
  const rate = (n: number, d: number) => (d === 0 ? "n/a" : `${((n / d) * 100).toFixed(1)}%`);
  return [
    ``,
    `${tier} tier — ${pan + wrap} fixtures`,
    `                 got pan   got wrap`,
    `  expect pan   ${String(m.panPan).padStart(8)}   ${String(m.panWrap.length).padStart(8)}`,
    `  expect wrap  ${String(m.wrapPan.length).padStart(8)}   ${String(m.wrapWrap).padStart(8)}`,
    ``,
    `  false wrap (must be zero): ${m.panWrap.length}/${pan} = ${rate(m.panWrap.length, pan)}`,
    m.panWrap.length ? `    ${m.panWrap.join("\n    ")}` : `    none`,
    `  false pan (tolerated):     ${m.wrapPan.length}/${wrap} = ${rate(m.wrapPan.length, wrap)}`,
    m.wrapPan.length ? `    ${m.wrapPan.join("\n    ")}` : `    none`,
    ``,
  ].join("\n");
}

describe("block classification against the real-output corpus", () => {
  const core = BLOCK_CORPUS.filter((f) => f.tier === "core");
  const extended = BLOCK_CORPUS.filter((f) => f.tier === "extended");

  it("prints the confusion matrix for both tiers", () => {
    // Always prints, pass or fail. The matrix is the deliverable; the gate
    // below is only the part of it the operator's decision made binding.
    console.log(report("core", score(core)));
    console.log(report("extended", score(extended)));
  });

  it("never wraps a block the program laid out (core tier)", () => {
    // The operator's locked decision (2026-08-11): the choice is fully
    // automatic with no UI control, and the classifier errs toward pan when
    // unsure. That makes this the one number with a hard bar on it.
    //
    // If this fails, the honest fixes are to add a detector for the listed
    // shapes or to invert the fallback in `looksStructured` — not to relabel
    // the fixture. A fixture is only wrong if R22 does not actually apply to
    // it, which is an argument to have in its `why`, in the open.
    expect(score(core).panWrap).toEqual([]);
  });

  it("keeps the tolerated direction visible without gating on it", () => {
    // No bar, on purpose: a false pan is what shipped before the classifier
    // existed. This exists so the number cannot drift unnoticed to 100%,
    // which would mean the classifier had quietly stopped classifying.
    const m = score(core);
    expect(m.wrapWrap).toBeGreaterThan(0);
  });
});

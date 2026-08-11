// Measures the DOM cost of the collie-style render approach against real
// herdr `format:"ansi"` captures: how many <span> elements a line actually
// becomes, and how long the SGR parse takes.
//
// The span count is the load-bearing number. xterm's DOM renderer paints only
// the rows inside its grid viewport; a flat <pre> paints every line it holds,
// so the node count at 400/1000 lines decides whether the swap helps or hurts
// smoothness on a phone.
//
// Usage: node parse-and-count.mjs <fixture.ans> [...]

import { readFileSync } from "node:fs";
import { basename } from "node:path";

// Faithful-enough port of collie's `parseAnsi` for counting purposes: split the
// stream into style runs, where a run ends whenever an SGR sequence changes the
// active style. Non-SGR escapes are consumed and produce no run, matching
// collie's defensive skip. Only the run *count* matters here, not the colors,
// so styles are compared by a serialized key rather than modelled fully.
function segmentsPerLine(text) {
  const perLine = [];
  let style = "";
  let runsThisLine = 0;
  let runOpen = false;

  let i = 0;
  const n = text.length;
  while (i < n) {
    const c = text[i];

    if (c === "\n") {
      perLine.push(runsThisLine);
      runsThisLine = 0;
      runOpen = false;
      i++;
      continue;
    }

    if (c === "\x1b") {
      if (text[i + 1] === "[") {
        let j = i + 2;
        while (j < n && !(text[j] >= "@" && text[j] <= "~")) j++;
        const final = text[j];
        const params = text.slice(i + 2, j);
        if (final === "m") {
          // Match collie's `flush()`: any SGR sequence closes the current run
          // when text has accumulated, whether or not the resulting style
          // actually differs (flush() only short-circuits on an empty buffer).
          // Treating a repeated `ESC[0m` as a no-op would undercount spans.
          style = applySgr(style, params);
          runOpen = false;
        }
        i = j + 1;
        continue;
      }
      // OSC / DCS / two-byte escapes: consumed, no run produced.
      if (text[i + 1] === "]") {
        let j = i + 2;
        while (j < n && text[j] !== "\x07" && !(text[j] === "\x1b" && text[j + 1] === "\\")) j++;
        i = text[j] === "\x07" ? j + 1 : j + 2;
        continue;
      }
      i += 2;
      continue;
    }

    if (!runOpen) {
      runsThisLine++;
      runOpen = true;
    }
    i++;
  }
  perLine.push(runsThisLine);
  return perLine;
}

// Reduce an SGR parameter string to a style key. Correctness of the *key* only
// needs to distinguish styles, so unhandled codes collapse into the raw param.
function applySgr(style, params) {
  const parts = params.split(/[;:]/);
  let s = style;
  for (const p of parts) {
    if (p === "" || p === "0") {
      s = "";
      continue;
    }
    s = s + "|" + p;
  }
  return s;
}

function stats(values) {
  if (values.length === 0) return { mean: 0, p50: 0, p95: 0, max: 0 };
  const sorted = [...values].sort((a, b) => a - b);
  const sum = values.reduce((a, b) => a + b, 0);
  return {
    mean: sum / values.length,
    p50: sorted[Math.floor(sorted.length * 0.5)],
    p95: sorted[Math.floor(sorted.length * 0.95)],
    max: sorted[sorted.length - 1],
  };
}

const files = process.argv.slice(2);
if (files.length === 0) {
  console.error("usage: node parse-and-count.mjs <fixture.ans> [...]");
  process.exit(2);
}

console.log(
  ["fixture", "lines", "bytes", "spans", "spans/line", "p50", "p95", "max", "parse ms"].join("\t"),
);

for (const file of files) {
  const text = readFileSync(file, "utf8");
  const lines = text.split("\n");

  // Warm, then time a batch so a single fast parse doesn't sit under timer
  // resolution.
  segmentsPerLine(text);
  const ITER = 50;
  const t0 = performance.now();
  let counts;
  for (let k = 0; k < ITER; k++) counts = segmentsPerLine(text);
  const parseMs = (performance.now() - t0) / ITER;

  const nonEmpty = counts.filter((c) => c > 0);
  const total = counts.reduce((a, b) => a + b, 0);
  const s = stats(nonEmpty);

  console.log(
    [
      basename(file),
      lines.length,
      text.length,
      total,
      s.mean.toFixed(1),
      s.p50,
      s.p95,
      s.max,
      parseMs.toFixed(2),
    ].join("\t"),
  );
}

// Project the node count to the sizes the gateway actually renders. The screen
// view caps at HISTORY_ROW_CEILING = 1000 rows (web/src/views/terminal.ts) and
// the history path renders up to 400.
console.log("\nProjected DOM nodes (spans + one line container each):");
for (const file of files) {
  const text = readFileSync(file, "utf8");
  const counts = segmentsPerLine(text);
  const nonEmpty = counts.filter((c) => c > 0);
  if (nonEmpty.length === 0) continue;
  const perLine = nonEmpty.reduce((a, b) => a + b, 0) / nonEmpty.length;
  for (const rows of [400, 1000]) {
    console.log(
      `  ${basename(file)} @ ${rows} rows: ~${Math.round(perLine * rows).toLocaleString()} spans`,
    );
  }
}

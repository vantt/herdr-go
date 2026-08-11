// Renders real pane captures through the actual renderer, in a real browser, at
// a phone width — the check the unit tests structurally cannot do.
//
// Drives the vite dev server so the modules and stylesheet under test are the
// ones the app ships, not a copy. Writes screenshots next to itself and prints
// measured geometry, so "the table pans and the prose wraps" is an observation
// rather than a claim.
//
// Usage: node visual-check.mjs   (expects `npx vite` already serving web/)

import { readFileSync, readdirSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const HERE = dirname(fileURLToPath(import.meta.url));
// This spike lives outside web/, and ESM resolves packages from the importing
// file's own directory — so reach into web/'s installed playwright explicitly
// rather than moving the script into the app's tree.
const { chromium } = createRequire(join(HERE, "../../web/package.json"))("playwright");
const FIXTURES = join(HERE, "../260811-1219-terminal-render-benchmark/fixtures");
const BASE = process.env.BASE_URL ?? "http://localhost:5173/";
// iPhone-ish logical viewport: the case the whole feature exists for.
const VIEWPORT = { width: 390, height: 780 };

const SYNTHETIC = [
  "This is an ordinary paragraph of agent output, long enough that a phone would",
  "otherwise have to be dragged sideways to read all of it comfortably at all.",
  "",
  "NAME      AGE  ROLE          LOCATION            STATUS      LAST SEEN",
  "alice     30   engineer      Ho Chi Minh City    active      2 minutes ago",
  "bob       41   designer      Da Nang             away        3 hours ago",
  "carolina  25   writer        Ha Noi              active      just now",
  "dan       37   manager       Can Tho             offline     yesterday",
  "",
  "[32mcolour[0m and [1mbold[0m and a link https://example.dev/some/path here",
  "",
  "┌──────────────────────┐",
  "│ a drawn box that must not wrap │",
  "└──────────────────────┘",
].join("\n");

async function renderScreen(page, text) {
  return page.evaluate(async (screenText) => {
    const { createTerminalRender } = await import("/src/terminal-render.ts");
    document.body.style.margin = "0";
    document.body.innerHTML = '<div class="view view-terminal" style="height:100vh"><div class="term-viewport" id="probe"></div></div>';
    const host = document.getElementById("probe");
    const render = createTerminalRender(host, 12);
    render.write(screenText);

    const screen = host.querySelector(".term-screen");
    const lineHeight = parseFloat(getComputedStyle(screen).lineHeight);

    const blocks = Array.from(host.querySelectorAll(".term-block")).map((el) => {
      const text = el.textContent ?? "";
      return {
        mode: el.classList.contains("is-wrap") ? "wrap" : "pan",
        clientWidth: el.clientWidth,
        scrollWidth: el.scrollWidth,
        height: el.getBoundingClientRect().height,
        // What the content says it is, versus how many rows it actually
        // occupies. Equal means nothing was broken; more means it wrapped.
        logicalLines: text.split("\n").length,
        renderedLines: Math.round(el.getBoundingClientRect().height / lineHeight),
        firstLine: text.split("\n")[0].slice(0, 40),
      };
    });
    // Measure the gap *between* consecutive blocks, not the separator span's
    // own height. A zero-height inline still sits in a line box whose strut
    // comes from the containing block's font, so the span can measure 0 while
    // pushing the blocks a full line apart.
    const blockEls = Array.from(host.querySelectorAll(".term-block"));
    const gaps = blockEls.slice(1).map((el, i) => {
      const prev = blockEls[i].getBoundingClientRect();
      return Math.round(el.getBoundingClientRect().top - prev.bottom);
    });

    // The renderer strips escapes and normalises CRLF, so a faithful copy is
    // the parser's own plain text — comparing against the raw ANSI bytes would
    // only be asserting that escapes survive, which they must not.
    const { parseAnsi } = await import("/src/ansi.ts");
    const expected = parseAnsi(screenText)
      .map((line) => line.segments.map((s) => s.text).join(""))
      .join("\n");

    return {
      viewportClientWidth: host.clientWidth,
      viewportScrollWidth: host.scrollWidth,
      screenWidth: screen ? screen.getBoundingClientRect().width : 0,
      textRoundTrips: (screen?.textContent ?? "") === expected,
      anchors: host.querySelectorAll("a").length,
      styledSpans: host.querySelectorAll("span:not(.term-nl)").length,
      blocks,
      gaps,
    };
  }, text);
}

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: VIEWPORT, deviceScaleFactor: 2 });
await page.goto(BASE, { waitUntil: "networkidle" });

let failures = 0;
const check = (label, ok, detail = "") => {
  if (!ok) failures++;
  console.log(`${ok ? "PASS" : "FAIL"}  ${label}${detail ? ` — ${detail}` : ""}`);
};

console.log(`\n=== synthetic screen @ ${VIEWPORT.width}px ===`);
const s = await renderScreen(page, SYNTHETIC);
await page.screenshot({ path: join(HERE, "shots/shot-synthetic.png"), fullPage: true });

console.log(
  `viewport client/scroll = ${s.viewportClientWidth}/${s.viewportScrollWidth}, screen width = ${Math.round(s.screenWidth)}`,
);
for (const b of s.blocks) {
  console.log(
    `  ${b.mode.padEnd(4)} client=${String(b.clientWidth).padStart(4)} scroll=${String(b.scrollWidth).padStart(5)} h=${String(Math.round(b.height)).padStart(3)}  ${JSON.stringify(b.firstLine)}`,
  );
}

check("screen never forces the page to pan sideways", s.viewportScrollWidth <= s.viewportClientWidth + 1,
  `scroll ${s.viewportScrollWidth} vs client ${s.viewportClientWidth}`);
check("copied text round-trips exactly", s.textRoundTrips);
check("the URL became a link", s.anchors === 1, `${s.anchors} anchors`);
check("styling produced spans", s.styledSpans > 0, `${s.styledSpans} spans`);

const wrapBlocks = s.blocks.filter((b) => b.mode === "wrap");
const panBlocks = s.blocks.filter((b) => b.mode === "pan");
check("every wrapping block fits its width", wrapBlocks.every((b) => b.scrollWidth <= b.clientWidth + 1),
  wrapBlocks.map((b) => `${b.scrollWidth}/${b.clientWidth}`).join(" "));
// The property that matters for "tables intact": a laid-out block occupies
// exactly as many rows as it has lines, i.e. nothing was broken. Overflowing
// the viewport is incidental — a narrow table is still a table.
check("no laid-out block is ever broken", panBlocks.every((b) => b.renderedLines === b.logicalLines),
  panBlocks.map((b) => `${b.renderedLines}/${b.logicalLines}`).join(" "));
check("a laid-out block wider than the screen scrolls on its own",
  panBlocks.some((b) => b.scrollWidth > b.clientWidth + 1),
  `${panBlocks.filter((b) => b.scrollWidth > b.clientWidth + 1).length} of ${panBlocks.length} overflow`);
// And the wrap side actually did something: at least one prose block occupies
// more rows than it has lines.
check("a long prose block really is broken to fit",
  wrapBlocks.some((b) => b.renderedLines > b.logicalLines),
  wrapBlocks.map((b) => `${b.renderedLines}/${b.logicalLines}`).join(" "));
const blanks = s.blocks.filter((b) => b.firstLine === "");
check("blank lines keep a line's height", blanks.every((b) => b.height > 4),
  blanks.map((b) => Math.round(b.height)).join(","));
check("blocks sit flush against each other", s.gaps.every((g) => g === 0),
  `gaps: ${s.gaps.join(",")}`);

if (existsSync(FIXTURES)) {
  for (const file of readdirSync(FIXTURES).filter((f) => f.endsWith(".ans")).sort()) {
    const text = readFileSync(join(FIXTURES, file), "utf8");
    if (text.trim() === "") continue;
    console.log(`\n=== ${file} @ ${VIEWPORT.width}px ===`);
    const r = await renderScreen(page, text);
    await page.screenshot({ path: join(HERE, `shots/shot-${file.replace(/\.ans$/, "")}.png`), fullPage: false });
    const wrap = r.blocks.filter((b) => b.mode === "wrap").length;
    const pan = r.blocks.filter((b) => b.mode === "pan").length;
    console.log(`  blocks: ${wrap} wrap / ${pan} pan; page scroll ${r.viewportScrollWidth} vs client ${r.viewportClientWidth}`);
    check(`${file}: page does not pan sideways`, r.viewportScrollWidth <= r.viewportClientWidth + 1);
    check(`${file}: copied text round-trips`, r.textRoundTrips);
    check(
      `${file}: wrapping blocks fit`,
      r.blocks.filter((b) => b.mode === "wrap").every((b) => b.scrollWidth <= b.clientWidth + 1),
    );
    const brokenPan = r.blocks.filter((b) => b.mode === "pan" && b.renderedLines !== b.logicalLines);
    check(`${file}: no laid-out block broken`, brokenPan.length === 0,
      brokenPan.map((b) => `${b.renderedLines}/${b.logicalLines} ${JSON.stringify(b.firstLine)}`).join(" | "));
  }
}

await browser.close();
console.log(`\n${failures === 0 ? "ALL CHECKS PASSED" : `${failures} CHECK(S) FAILED`}`);
process.exit(failures === 0 ? 0 : 1);

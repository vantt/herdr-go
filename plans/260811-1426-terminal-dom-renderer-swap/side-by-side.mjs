// Renders the same capture through xterm.js and through the new renderer, at
// the same width and font size, and screenshots both.
//
// "No visible change" is the phase-2 acceptance criterion, so the reference has
// to be the thing being replaced — not an idea of what correct looks like. An
// artefact present in both is inherited, not introduced.
//
// Usage: node side-by-side.mjs <fixture.ans>   (expects vite serving web/)

import { readFileSync } from "node:fs";
import { dirname, join, basename } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const HERE = dirname(fileURLToPath(import.meta.url));
const { chromium } = createRequire(join(HERE, "../../web/package.json"))("playwright");
const BASE = process.env.BASE_URL ?? "http://localhost:5173/";
const FONT = 12;
const VIEWPORT = { width: 390, height: 900 };

const file = process.argv[2];
if (!file) {
  console.error("usage: node side-by-side.mjs <fixture.ans>");
  process.exit(2);
}
const text = readFileSync(file, "utf8");
const name = basename(file).replace(/\.ans$/, "");

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: VIEWPORT, deviceScaleFactor: 2 });
await page.goto(BASE, { waitUntil: "networkidle" });

// New renderer.
await page.evaluate(async ({ screenText, font }) => {
  const { createTerminalRender } = await import("/src/terminal-render.ts");
  document.body.style.margin = "0";
  document.body.innerHTML = '<div class="term-viewport" id="probe" style="height:100vh"></div>';
  createTerminalRender(document.getElementById("probe"), font).write(screenText);
}, { screenText: text, font: FONT });
await page.screenshot({ path: join(HERE, `shots/cmp-${name}-new.png`), fullPage: false });

// xterm, configured exactly as the view used to configure it.
await page.evaluate(async ({ screenText, font }) => {
  // Vite's CJS interop can put the export on `default` rather than as a
  // named binding, depending on how the dep was pre-bundled.
  const mod = await import("/@id/@xterm/xterm");
  const Terminal = mod.Terminal ?? mod.default?.Terminal ?? mod.default;
  document.body.innerHTML = '<div class="term-viewport" id="probe" style="height:100vh"></div>';
  const host = document.getElementById("probe");
  const lines = screenText.split("\n");
  const visible = (l) => l.replace(/\x1b\[[0-9;?]*[a-zA-Z]/g, "").length;
  const term = new Terminal({
    fontFamily: "ui-monospace, 'JetBrains Mono', 'SF Mono', Menlo, Consolas, monospace",
    fontSize: font,
    cursorBlink: false,
    disableStdin: true,
    scrollback: 0,
    convertEol: true,
    cols: Math.min(400, Math.max(20, Math.max(...lines.map(visible)) + 1)),
    rows: Math.min(1000, Math.max(4, lines.length + 1)),
    theme: { background: "#0b0e14", foreground: "#e4e8f1" },
  });
  term.open(host);
  await new Promise((r) => term.write(screenText.replace(/\n/g, "\r\n"), r));
  // Match the old CSS, which let the grid keep its natural size and left
  // .term-viewport to scroll.
  const el = host.querySelector(".xterm");
  if (el) {
    el.style.height = "max-content";
    el.style.width = "max-content";
  }
}, { screenText: text, font: FONT });
await page.waitForTimeout(300);
await page.screenshot({ path: join(HERE, `shots/cmp-${name}-xterm.png`), fullPage: false });

await browser.close();
console.log(`wrote cmp-${name}-new.png and cmp-${name}-xterm.png`);

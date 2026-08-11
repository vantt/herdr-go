import { describe, it, expect } from "vitest";
import { buildScreenFragment, splitLineIntoPieces } from "../src/terminal-render";
import { parseAnsi, ANSI_PALETTE } from "../src/ansi";

/** Render text the way the view does and hand back the container. */
function render(text: string): HTMLElement {
  const host = document.createElement("pre");
  host.appendChild(buildScreenFragment(parseAnsi(text)));
  return host;
}

describe("splitLineIntoPieces", () => {
  it("leaves a line with no links as its own style runs", () => {
    const [line] = parseAnsi("\x1b[31mred\x1b[0m plain");
    expect(splitLineIntoPieces(line).map((p) => p.text)).toEqual(["red", " plain"]);
  });

  it("splits a style run around a link inside it", () => {
    const [line] = parseAnsi("see https://example.dev/a now");
    const pieces = splitLineIntoPieces(line);
    expect(pieces.map((p) => p.text)).toEqual(["see ", "https://example.dev/a", " now"]);
    expect(pieces[1].href).toBe("https://example.dev/a");
    expect(pieces[0].href).toBeUndefined();
  });

  it("keeps a link whole across a style change in the middle of it", () => {
    // A prompt often colours the scheme differently from the host; the URL is
    // still one link and must not become two.
    const [line] = parseAnsi("\x1b[34mhttps://\x1b[0mexample.dev/a");
    const pieces = splitLineIntoPieces(line);
    expect(pieces.filter((p) => p.href !== undefined).map((p) => p.text).join("")).toBe(
      "https://example.dev/a",
    );
  });

  it("preserves the whole line's text whatever the splitting", () => {
    const source = "\x1b[31mred\x1b[0m https://example.dev/a tail";
    const [line] = parseAnsi(source);
    const rebuilt = splitLineIntoPieces(line)
      .map((p) => p.text)
      .join("");
    expect(rebuilt).toBe("red https://example.dev/a tail");
  });
});

describe("buildScreenFragment", () => {
  describe("copy fidelity", () => {
    it("round-trips the screen text through textContent", () => {
      // Blocks are block-level boxes, which concatenate in textContent without
      // a newline of their own. A copied selection has to come back out as the
      // screen went in.
      const text = [
        "This is a paragraph of ordinary output that should wrap for a reader.",
        "",
        "alice    30   engineer",
        "bob      41   designer",
        "carolina 25   writer",
        "dan      37   manager",
        "",
        "And a closing sentence after the table above.",
      ].join("\n");
      expect(render(text).textContent).toBe(text);
    });

    it("round-trips consecutive blank lines", () => {
      const text = "a\n\n\nb";
      expect(render(text).textContent).toBe(text);
    });

    it("round-trips a single line with no trailing newline", () => {
      expect(render("just one").textContent).toBe("just one");
    });

    it("round-trips text carrying styling", () => {
      expect(render("\x1b[31mred\x1b[0m and plain").textContent).toBe("red and plain");
    });
  });

  describe("block classification reaches the DOM", () => {
    it("marks a prose block to wrap and a table block to pan", () => {
      const text = [
        "This is an ordinary sentence of agent output for a reader to follow.",
        "",
        "alice    30   engineer",
        "bob      41   designer",
        "carolina 25   writer",
        "dan      37   manager",
      ].join("\n");
      const classes = Array.from(render(text).querySelectorAll(".term-block")).map(
        (el) => (el.classList.contains("is-wrap") ? "wrap" : "pan"),
      );
      expect(classes).toEqual(["wrap", "pan", "pan"]);
    });
  });

  describe("styling", () => {
    it("emits a bare text node for unstyled runs, not a span", () => {
      // Most of a screen is unstyled; giving each run an element would multiply
      // the node count for nothing.
      const host = render("plain text only");
      expect(host.querySelectorAll("span:not(.term-nl)")).toHaveLength(0);
      expect(host.textContent).toBe("plain text only");
    });

    it("applies colour, weight and decoration to a styled run", () => {
      const host = render("\x1b[1;4;31mstyled");
      const span = host.querySelector<HTMLElement>("span:not(.term-nl)")!;
      expect(span.style.color).toBeTruthy();
      expect(span.style.fontWeight).toBe("600");
      expect(span.style.textDecoration).toContain("underline");
    });

    it("renders inverse video as swapped colours", () => {
      const host = render("\x1b[7minverse");
      const span = host.querySelector<HTMLElement>("span:not(.term-nl)")!;
      expect(span.style.backgroundColor).toBeTruthy();
      expect(span.style.color).toBeTruthy();
      expect(span.style.color).not.toBe(span.style.backgroundColor);
    });

    it("uses the terminal's own foreground, not the app text colour", () => {
      // The two differ (#e4e8f1 vs the app's --text-primary); the mirror keeps
      // the terminal's.
      expect(ANSI_PALETTE.foreground).toBe("#e4e8f1");
    });
  });

  describe("links", () => {
    it("renders a URL as an anchor that cannot reach back into this tab", () => {
      const anchor = render("open https://example.dev/a").querySelector<HTMLAnchorElement>("a")!;
      expect(anchor.getAttribute("href")).toBe("https://example.dev/a");
      expect(anchor.target).toBe("_blank");
      expect(anchor.rel).toContain("noopener");
    });

    it("leaves prose without links alone", () => {
      expect(render("nothing to click here").querySelector("a")).toBeNull();
    });
  });

  describe("untrusted content", () => {
    it("renders markup as text, never as elements", () => {
      // Pane output is untrusted; styling is the only thing derived from it.
      const host = render('<img src=x onerror="boom">');
      expect(host.querySelector("img")).toBeNull();
      expect(host.textContent).toBe('<img src=x onerror="boom">');
    });

    it("does not build an anchor from a javascript: scheme", () => {
      const host = render("javascript:alert(1) and data:text/html,x");
      expect(host.querySelector("a")).toBeNull();
    });
  });
});

import { describe, it, expect } from "vitest";
import { findLinks } from "../src/linkify";

/** The matched substrings, which is what the renderer turns into anchors. */
function hrefs(text: string): string[] {
  return findLinks(text).map((l) => l.href);
}

describe("findLinks", () => {
  it("finds a bare http and https URL", () => {
    expect(hrefs("see http://example.com and https://example.dev/x")).toEqual([
      "http://example.com",
      "https://example.dev/x",
    ]);
  });

  it("reports ranges that map back onto the source text", () => {
    const text = "go to https://example.dev/a now";
    const [link] = findLinks(text);
    expect(text.slice(link.start, link.end)).toBe(link.href);
  });

  it("finds nothing in plain prose", () => {
    expect(hrefs("no links here, just words. and example.com bare")).toEqual([]);
  });

  it("ignores a scheme with no host", () => {
    expect(hrefs("https:// and http://")).toEqual([]);
  });

  describe("trailing punctuation", () => {
    it("drops sentence punctuation after the URL", () => {
      expect(hrefs("open https://example.dev/a.")).toEqual(["https://example.dev/a"]);
      expect(hrefs("open https://example.dev/a, then")).toEqual(["https://example.dev/a"]);
      expect(hrefs("open https://example.dev/a?")).toEqual(["https://example.dev/a"]);
    });

    it("drops a closing bracket that has no opener inside the URL", () => {
      expect(hrefs("(see https://example.dev/a)")).toEqual(["https://example.dev/a"]);
      expect(hrefs("[https://example.dev/a]")).toEqual(["https://example.dev/a"]);
    });

    it("keeps a balanced bracket pair belonging to the URL", () => {
      expect(hrefs("https://example.dev/a_(b)")).toEqual(["https://example.dev/a_(b)"]);
    });

    it("keeps a query string and fragment", () => {
      expect(hrefs("https://example.dev/a?b=1&c=2#frag")).toEqual([
        "https://example.dev/a?b=1&c=2#frag",
      ]);
    });
  });

  describe("terminal-adjacent characters", () => {
    it("stops at whitespace", () => {
      expect(hrefs("https://example.dev/a https://example.dev/b")).toEqual([
        "https://example.dev/a",
        "https://example.dev/b",
      ]);
    });

    it("stops at quotes and angle brackets", () => {
      expect(hrefs('"https://example.dev/a" <https://example.dev/b>')).toEqual([
        "https://example.dev/a",
        "https://example.dev/b",
      ]);
    });
  });

  it("is not stateful across calls", () => {
    // A module-level regex with the global flag keeps lastIndex between calls
    // unless reset; a second identical call must find the same link.
    const text = "https://example.dev/a";
    expect(hrefs(text)).toEqual(hrefs(text));
  });
});

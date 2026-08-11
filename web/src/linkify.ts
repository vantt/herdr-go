// URL detection for the terminal mirror.
//
// The terminal view promised clickable URLs (docs/specs/terminal-detail.md,
// Data Dictionary row 3) and delivered it through xterm's WebLinksAddon. The
// DOM renderer has no addon to lean on, so the same behaviour is reproduced
// here: find bare http(s) URLs in a line's plain text and report their ranges,
// leaving it to the renderer to split styled runs around them.
//
// Ranges are reported over the line's *visible* text — escape sequences are
// already gone by the time this runs.

export interface LinkRange {
  /** Index of the first character of the URL within the line. */
  start: number;
  /** Index one past the last character. */
  end: number;
  href: string;
}

// Bare http(s) URLs. Stops at whitespace and at characters that cannot appear
// unencoded in a URL and that terminals routinely put next to one (angle
// brackets, quotes, backtick).
const URL_RE = /https?:\/\/[^\s<>"'`\\]+/g;

// Trailing punctuation that far more often ends the surrounding sentence than
// the URL itself.
const TRAILING = new Set([".", ",", ";", ":", "!", "?"]);

/**
 * Trim characters a terminal is likely to have written after the URL rather
 * than as part of it: sentence punctuation, and a closing bracket with no
 * matching opener inside the match (so `(see http://x.dev/a)` keeps `/a` but
 * drops the `)`, while `http://x.dev/a_(b)` keeps its balanced pair).
 */
function trimTrailing(url: string): string {
  let end = url.length;
  const pairs: Array<[string, string]> = [
    ["(", ")"],
    ["[", "]"],
    ["{", "}"],
  ];

  for (;;) {
    if (end === 0) break;
    const ch = url[end - 1];

    if (TRAILING.has(ch)) {
      end--;
      continue;
    }

    const pair = pairs.find(([, close]) => close === ch);
    if (pair) {
      const slice = url.slice(0, end);
      const opens = slice.split(pair[0]).length - 1;
      const closes = slice.split(pair[1]).length - 1;
      if (closes > opens) {
        end--;
        continue;
      }
    }

    break;
  }

  return url.slice(0, end);
}

/** Every http(s) URL in `text`, in order, with trailing punctuation trimmed. */
export function findLinks(text: string): LinkRange[] {
  const out: LinkRange[] = [];
  URL_RE.lastIndex = 0;
  for (let m = URL_RE.exec(text); m !== null; m = URL_RE.exec(text)) {
    const href = trimTrailing(m[0]);
    // A match that trims away to just the scheme is not a link worth offering.
    if (!/^https?:\/\/[^/\s]+/.test(href)) continue;
    out.push({ start: m.index, end: m.index + href.length, href });
  }
  return out;
}

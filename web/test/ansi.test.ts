import { describe, it, expect } from "vitest";
import { parseAnsi, ANSI_PALETTE, type StyledLine } from "../src/ansi";

/** Concatenated visible text of a line — what the operator actually reads. */
function textOf(line: StyledLine): string {
  return line.segments.map((s) => s.text).join("");
}

describe("parseAnsi", () => {
  describe("plain text", () => {
    it("returns one unstyled segment for a bare line", () => {
      const [line] = parseAnsi("hello");
      expect(line.segments).toEqual([{ text: "hello" }]);
    });

    it("splits on newlines and keeps empty lines", () => {
      const lines = parseAnsi("a\n\nb");
      expect(lines.map(textOf)).toEqual(["a", "", "b"]);
    });

    it("preserves text that looks like markup, as text", () => {
      // The renderer puts these in text nodes; the parser must not mangle or
      // interpret them.
      const [line] = parseAnsi("<script>alert(1)</script> & \"quotes\"");
      expect(textOf(line)).toBe('<script>alert(1)</script> & "quotes"');
    });
  });

  describe("colour", () => {
    it("maps the basic 16 foreground and background codes", () => {
      const [line] = parseAnsi("\x1b[31mred\x1b[42mgreenbg");
      expect(line.segments[0]).toMatchObject({ text: "red", fg: ANSI_PALETTE.red });
      expect(line.segments[1]).toMatchObject({ fg: ANSI_PALETTE.red, bg: ANSI_PALETTE.green });
    });

    it("maps bright foreground and background codes", () => {
      const [line] = parseAnsi("\x1b[91mbright\x1b[102mbg");
      expect(line.segments[0]).toMatchObject({ fg: ANSI_PALETTE.brightRed });
      expect(line.segments[1]).toMatchObject({ bg: ANSI_PALETTE.brightGreen });
    });

    it("resets only the foreground on 39 and only the background on 49", () => {
      const [line] = parseAnsi("\x1b[31;42ma\x1b[39mb\x1b[49mc");
      expect(line.segments[0]).toMatchObject({ fg: ANSI_PALETTE.red, bg: ANSI_PALETTE.green });
      expect(line.segments[1]).toMatchObject({ bg: ANSI_PALETTE.green });
      expect(line.segments[1].fg).toBeUndefined();
      expect(line.segments[2].bg).toBeUndefined();
    });

    it("resolves 256-colour indices across all three ranges", () => {
      // 0-15 reuse the named palette, 16-231 are the 6x6x6 cube, 232-255 grey.
      const [line] = parseAnsi("\x1b[38;5;1ma\x1b[38;5;196mb\x1b[38;5;232mc");
      expect(line.segments[0].fg).toBe(ANSI_PALETTE.red);
      expect(line.segments[1].fg).toBe("#ff0000");
      expect(line.segments[2].fg).toBe("#080808");
    });

    it("resolves 24-bit colour in the semicolon form", () => {
      const [line] = parseAnsi("\x1b[38;2;177;185;249mx");
      expect(line.segments[0].fg).toBe("#b1b9f9");
    });

    it("resolves 24-bit colour in the ISO colon form", () => {
      const [line] = parseAnsi("\x1b[38:2:177:185:249mx");
      expect(line.segments[0].fg).toBe("#b1b9f9");
    });

    it("resolves the ISO colon form with an empty colour-space field", () => {
      const [line] = parseAnsi("\x1b[38:2::177:185:249mx");
      expect(line.segments[0].fg).toBe("#b1b9f9");
    });

    it("resolves a 256-colour background", () => {
      const [line] = parseAnsi("\x1b[48;5;196mx");
      expect(line.segments[0].bg).toBe("#ff0000");
    });
  });

  describe("attributes", () => {
    it("sets each attribute flag", () => {
      const [line] = parseAnsi("\x1b[1;2;3;4;9mstyled");
      expect(line.segments[0]).toMatchObject({
        bold: true,
        dim: true,
        italic: true,
        underline: true,
        strike: true,
      });
    });

    it("clears attributes with their off-codes", () => {
      const [line] = parseAnsi("\x1b[1;3;4;9ma\x1b[22;23;24;29mb");
      const plain = line.segments[1];
      expect(plain.bold).toBeUndefined();
      expect(plain.italic).toBeUndefined();
      expect(plain.underline).toBeUndefined();
      expect(plain.strike).toBeUndefined();
    });

    it("clears both bold and dim on 22", () => {
      const [line] = parseAnsi("\x1b[1;2ma\x1b[22mb");
      expect(line.segments[1].bold).toBeUndefined();
      expect(line.segments[1].dim).toBeUndefined();
    });

    it("resets everything on 0 and on a bare ESC[m", () => {
      const [line] = parseAnsi("\x1b[1;31ma\x1b[0mb\x1b[1;31mc\x1b[md");
      expect(line.segments[1]).toEqual({ text: "b" });
      expect(line.segments[3]).toEqual({ text: "d" });
    });
  });

  describe("inverse video", () => {
    it("swaps explicit foreground and background", () => {
      const [line] = parseAnsi("\x1b[31;42;7mx");
      expect(line.segments[0]).toMatchObject({
        fg: ANSI_PALETTE.green,
        bg: ANSI_PALETTE.red,
      });
    });

    it("falls back to the viewport defaults when a side is unset", () => {
      const [line] = parseAnsi("\x1b[7mx");
      expect(line.segments[0]).toMatchObject({
        fg: ANSI_PALETTE.background,
        bg: ANSI_PALETTE.foreground,
      });
    });

    it("stops swapping on 27", () => {
      const [line] = parseAnsi("\x1b[31;7ma\x1b[27mb");
      expect(line.segments[1]).toMatchObject({ fg: ANSI_PALETTE.red });
      expect(line.segments[1].bg).toBeUndefined();
    });
  });

  describe("carriage return", () => {
    it("discards the line so far when text follows on the same line", () => {
      // Progress bars redraw by returning to column 0; only the final frame
      // should survive.
      const [line] = parseAnsi("50%\rdone");
      expect(textOf(line)).toBe("done");
    });

    it("keeps the style state across the overwrite", () => {
      const [line] = parseAnsi("\x1b[31m50%\rdone");
      expect(line.segments[0]).toMatchObject({ text: "done", fg: ANSI_PALETTE.red });
    });

    it("treats CRLF as a plain line terminator", () => {
      // herdr's buffer is CRLF; overwriting here would blank every line.
      const lines = parseAnsi("first\r\nsecond");
      expect(lines.map(textOf)).toEqual(["first", "second"]);
    });

    it("treats a trailing CR as a terminator, not an overwrite", () => {
      const [line] = parseAnsi("kept\r");
      expect(textOf(line)).toBe("kept");
    });
  });

  describe("tabs", () => {
    it("advances to the next multiple of eight columns", () => {
      const [line] = parseAnsi("ab\tc");
      expect(textOf(line)).toBe("ab      c");
    });

    it("advances a full stop when already on a tab stop", () => {
      const [line] = parseAnsi("12345678\tx");
      expect(textOf(line)).toBe("12345678        x");
    });

    it("restarts column tracking on each line", () => {
      const lines = parseAnsi("abcdefghij\na\tb");
      expect(textOf(lines[1])).toBe("a       b");
    });
  });

  describe("sequences that must be consumed, not rendered", () => {
    it("drops a BEL-terminated OSC", () => {
      const [line] = parseAnsi("\x1b]0;window title\x07visible");
      expect(textOf(line)).toBe("visible");
    });

    it("drops an ST-terminated OSC", () => {
      const [line] = parseAnsi("\x1b]0;window title\x1b\\visible");
      expect(textOf(line)).toBe("visible");
    });

    it("drops a DEC-private sequence whole", () => {
      const [line] = parseAnsi("\x1b[?1049hvisible");
      expect(textOf(line)).toBe("visible");
    });

    it("does not treat a private-marker final m as SGR", () => {
      const [line] = parseAnsi("\x1b[?4mvisible");
      expect(textOf(line)).toBe("visible");
      expect(line.segments[0].fg).toBeUndefined();
    });

    it("drops a non-SGR CSI without altering style", () => {
      const [line] = parseAnsi("\x1b[31ma\x1b[2Kb");
      expect(line.segments.every((s) => s.fg === ANSI_PALETTE.red)).toBe(true);
      expect(textOf(line)).toBe("ab");
    });

    it("drops a two-byte escape", () => {
      const [line] = parseAnsi("\x1b(Bvisible");
      expect(textOf(line)).toBe("visible");
    });

    it("terminates on an unfinished escape at end of input", () => {
      expect(parseAnsi("text\x1b[31").map(textOf)).toEqual(["text"]);
      expect(parseAnsi("text\x1b]0;no-end").map(textOf)).toEqual(["text"]);
    });

    it("drops stray control characters", () => {
      const [line] = parseAnsi("a\x00b\x7fc");
      expect(textOf(line)).toBe("abc");
    });
  });

  describe("unicode", () => {
    it("keeps CJK and emoji in the text as-is", () => {
      const [line] = parseAnsi("\x1b[32m日本語テスト 🚀 ok");
      expect(textOf(line)).toBe("日本語テスト 🚀 ok");
    });
  });

  describe("segmentation", () => {
    it("opens a new segment at every style change", () => {
      const [line] = parseAnsi("\x1b[31ma\x1b[32mb\x1b[0mc");
      expect(line.segments.map((s) => s.text)).toEqual(["a", "b", "c"]);
    });

    it("produces no segment for an SGR with no text after it", () => {
      const [line] = parseAnsi("\x1b[31m\x1b[32mtext");
      expect(line.segments).toHaveLength(1);
      expect(line.segments[0]).toMatchObject({ text: "text", fg: ANSI_PALETTE.green });
    });
  });
});

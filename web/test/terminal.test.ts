import { describe, it, expect } from "vitest";
import { stripAnsiLen } from "../src/views/terminal";

describe("stripAnsiLen", () => {
  it("counts visible characters, ignoring ANSI escapes", () => {
    expect(stripAnsiLen("hello")).toBe(5);
    expect(stripAnsiLen("\x1b[32mhello\x1b[0m")).toBe(5);
    expect(stripAnsiLen("\x1b[1;33mA\x1b[0mB")).toBe(2);
  });

  it("handles a plain empty line", () => {
    expect(stripAnsiLen("")).toBe(0);
  });
});

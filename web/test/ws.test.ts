import { describe, it, expect, vi } from "vitest";
import { applyFrame, base64ToBytes, type ServerFrame, type TerminalLike } from "../src/ws";

function fakeTerm(): TerminalLike & { reset: ReturnType<typeof vi.fn>; write: ReturnType<typeof vi.fn> } {
  return { reset: vi.fn(), write: vi.fn() };
}

describe("base64ToBytes", () => {
  it("decodes standard base64 to the matching byte sequence", () => {
    // "hi" -> base64 "aGk="
    expect(Array.from(base64ToBytes("aGk="))).toEqual([104, 105]);
  });

  it("decodes an empty string to an empty array", () => {
    expect(base64ToBytes("").length).toBe(0);
  });

  it("round-trips arbitrary bytes through btoa/atob", () => {
    const original = new Uint8Array([0, 1, 27, 255, 128, 65]);
    const b64 = btoa(String.fromCharCode(...original));
    expect(Array.from(base64ToBytes(b64))).toEqual(Array.from(original));
  });
});

describe("applyFrame", () => {
  const baseFrame: ServerFrame = {
    type: "terminal.frame",
    seq: 1,
    encoding: "ansi",
    width: 80,
    height: 24,
    full: false,
    bytes: "aGk=",
  };

  it("resets before writing when full=true (whole-screen redraw)", () => {
    const term = fakeTerm();
    applyFrame(term, { ...baseFrame, full: true });
    expect(term.reset).toHaveBeenCalledTimes(1);
    expect(term.write).toHaveBeenCalledTimes(1);
    expect(term.reset.mock.invocationCallOrder[0]).toBeLessThan(term.write.mock.invocationCallOrder[0]);
  });

  it("writes without resetting when full=false (diff frame)", () => {
    const term = fakeTerm();
    applyFrame(term, baseFrame);
    expect(term.reset).not.toHaveBeenCalled();
    expect(term.write).toHaveBeenCalledTimes(1);
    expect(term.write).toHaveBeenCalledWith(base64ToBytes(baseFrame.bytes));
  });

  it("ignores gateway.error messages (no terminal payload to apply)", () => {
    const term = fakeTerm();
    applyFrame(term, { type: "gateway.error", reason: "boom" });
    expect(term.reset).not.toHaveBeenCalled();
    expect(term.write).not.toHaveBeenCalled();
  });
});

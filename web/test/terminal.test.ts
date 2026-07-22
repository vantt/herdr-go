import { describe, it, expect } from "vitest";
import { computeKeyboardInset, stripAnsiLen, terminalHead } from "../src/views/terminal";
import type { AgentRow } from "../src/api";
import type { NewPaneRef } from "../src/main";

describe("computeKeyboardInset", () => {
  it("returns 0 when the visual viewport matches the layout viewport (no keyboard)", () => {
    expect(computeKeyboardInset(800, 800, 0)).toBe(0);
  });

  it("returns the positive gap when the OS keyboard shrinks the visual viewport", () => {
    expect(computeKeyboardInset(800, 500, 0)).toBe(300);
  });

  it("accounts for a nonzero visualViewport.offsetTop", () => {
    expect(computeKeyboardInset(800, 500, 20)).toBe(280);
  });

  it("never returns a negative inset", () => {
    expect(computeKeyboardInset(800, 810, 0)).toBe(0);
  });

  it("degrades to 0 for the unsupported-API case (caller passes innerHeight as viewportHeight)", () => {
    // Callers feature-detect window.visualViewport and never call this
    // helper when it's undefined, which is the D2 graceful no-op — modeled
    // here by a viewport that matches the window exactly.
    const innerHeight = 800;
    expect(computeKeyboardInset(innerHeight, innerHeight, 0)).toBe(0);
  });
});

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

describe("terminalHead", () => {
  const agentRow: AgentRow = {
    pane_id: "p1",
    workspace: "ws-1",
    display: "claude · herdr",
    kind: "claude",
    status: "working",
    title: "building",
    workspace_label: "herdr",
    tab_label: "herdr",
    workspace_status: "working",
  };

  it("reads an AgentRow's own kind and display unchanged", () => {
    expect(terminalHead(agentRow)).toEqual({
      kind: "claude",
      display: "claude · herdr",
    });
  });

  it("derives 'shell' as the kind for a NewPaneRef with no name", () => {
    const ref: NewPaneRef = {
      pane_id: "p2",
      workspace_id: "ws-2",
      label: "herdr-gateway",
    };
    expect(terminalHead(ref)).toEqual({
      kind: "shell",
      display: "herdr-gateway",
    });
  });

  it("uses the preset name as the kind for a NewPaneRef with a name", () => {
    const ref: NewPaneRef = {
      pane_id: "p3",
      workspace_id: "ws-3",
      label: "herdr-gateway",
      name: "claude-abc123",
    };
    expect(terminalHead(ref)).toEqual({
      kind: "claude-abc123",
      display: "herdr-gateway",
    });
  });
});

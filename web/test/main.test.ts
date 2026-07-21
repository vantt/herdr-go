import { describe, it, expect, vi, afterEach } from "vitest";
import {
  pathForRoute,
  parseTerminalPaneId,
  resolvePaneRef,
  resolveLoginRedirect,
  navigate,
  goBack,
  type Route,
} from "../src/main";
import type { AgentRow, AgentsResponse, ShellRow } from "../src/api";

function agentRow(overrides: Partial<AgentRow> = {}): AgentRow {
  return {
    pane_id: "w1:p1",
    workspace: "w1",
    display: "claude · title",
    kind: "claude",
    status: "working",
    title: "title",
    workspace_label: "herdr-gateway",
    tab_label: "ui",
    workspace_status: "working",
    ...overrides,
  };
}

function shellRow(overrides: Partial<ShellRow> = {}): ShellRow {
  return {
    pane_id: "w2:p9",
    workspace_id: "w2",
    workspace_label: "scratch",
    tab_label: "shell",
    path: "/home/dev/scratch",
    ...overrides,
  };
}

describe("pathForRoute", () => {
  it("builds /terminal/<pane_id> for a terminal route (D1)", () => {
    const route: Route = { name: "terminal", agent: agentRow({ pane_id: "w1:p1" }) };
    expect(pathForRoute(route)).toBe("/terminal/w1%3Ap1");
  });

  it("resolves login and switcher to root '/' (D4)", () => {
    expect(pathForRoute({ name: "login" })).toBe("/");
    expect(pathForRoute({ name: "switcher" })).toBe("/");
  });
});

describe("parseTerminalPaneId", () => {
  it("round-trips a pane_id built by pathForRoute", () => {
    const route: Route = { name: "terminal", agent: agentRow({ pane_id: "w1:p1" }) };
    const path = pathForRoute(route);
    expect(parseTerminalPaneId(path)).toBe("w1:p1");
  });

  it("returns null for root and any non-terminal path", () => {
    expect(parseTerminalPaneId("/")).toBeNull();
    expect(parseTerminalPaneId("/switcher")).toBeNull();
    expect(parseTerminalPaneId("/terminal/")).toBeNull();
  });
});

describe("resolvePaneRef", () => {
  const agents = [agentRow({ pane_id: "w1:p1" })];
  const shells = [shellRow({ pane_id: "w2:p9", path: "/home/dev/scratch" })];

  it("returns the AgentRow unchanged on an agents[] hit", () => {
    expect(resolvePaneRef("w1:p1", agents, shells)).toEqual(agentRow({ pane_id: "w1:p1" }));
  });

  it("reshapes a ShellRow hit into a NewPaneRef, using path as the label", () => {
    expect(resolvePaneRef("w2:p9", agents, shells)).toEqual({
      pane_id: "w2:p9",
      workspace_id: "w2",
      label: "/home/dev/scratch",
    });
  });

  it("falls back to workspace_label when a ShellRow has no path", () => {
    const noPathShells = [shellRow({ pane_id: "w2:p9", path: null, workspace_label: "scratch" })];
    expect(resolvePaneRef("w2:p9", agents, noPathShells)).toEqual({
      pane_id: "w2:p9",
      workspace_id: "w2",
      label: "scratch",
    });
  });

  it("returns null for a pane_id in neither list (D3 stale link)", () => {
    expect(resolvePaneRef("does-not-exist", agents, shells)).toBeNull();
  });
});

describe("resolveLoginRedirect", () => {
  const snapshot: AgentsResponse = {
    agents: [agentRow({ pane_id: "w1:p1" })],
    shells: [shellRow({ pane_id: "w2:p9" })],
  };

  it("redirects into the intended terminal when it still resolves (D5)", () => {
    expect(resolveLoginRedirect("w1:p1", snapshot)).toEqual({
      name: "terminal",
      agent: agentRow({ pane_id: "w1:p1" }),
    });
  });

  it("falls back to switcher when the intended pane no longer resolves", () => {
    expect(resolveLoginRedirect("gone", snapshot)).toEqual({ name: "switcher" });
  });

  it("falls back to switcher when there was no intended pane", () => {
    expect(resolveLoginRedirect(null, snapshot)).toEqual({ name: "switcher" });
  });

  it("falls back to switcher when the snapshot itself is null", () => {
    expect(resolveLoginRedirect("w1:p1", null)).toEqual({ name: "switcher" });
  });
});

describe("navigate", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    history.replaceState(null, "", "/");
  });

  it("pushes a new history entry when moving into a different route (D2)", () => {
    history.replaceState(null, "", "/");
    const pushSpy = vi.spyOn(history, "pushState");
    navigate({ name: "terminal", agent: agentRow({ pane_id: "w1:p1" }) });
    expect(pushSpy).toHaveBeenCalledWith({ route: { name: "terminal", agent: agentRow({ pane_id: "w1:p1" }) } }, "", "/terminal/w1%3Ap1");
    expect(location.pathname).toBe("/terminal/w1%3Ap1");
  });

  it("replaces the current entry instead of pushing when the path is unchanged (switcher <-> login share '/')", () => {
    history.replaceState(null, "", "/");
    const pushSpy = vi.spyOn(history, "pushState");
    const replaceSpy = vi.spyOn(history, "replaceState");
    navigate({ name: "login" });
    expect(pushSpy).not.toHaveBeenCalled();
    expect(replaceSpy).toHaveBeenCalledWith({ route: { name: "login" } }, "", "/");
  });
});

describe("goBack", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("calls history.back() and never pushes a fresh forward entry (D2)", () => {
    const backSpy = vi.spyOn(history, "back").mockImplementation(() => {});
    const pushSpy = vi.spyOn(history, "pushState");
    goBack();
    expect(backSpy).toHaveBeenCalledTimes(1);
    expect(pushSpy).not.toHaveBeenCalled();
  });
});

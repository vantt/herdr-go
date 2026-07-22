import { describe, it, expect, vi, afterEach } from "vitest";
import {
  pathForRoute,
  parseTerminalPaneId,
  parseRoutePath,
  resolvePaneRef,
  resolveLoginRedirect,
  resolveBootstrapDecision,
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
  it("builds /terminal/<pane_id> for a terminal route (D1), leaving ':' unescaped", () => {
    const route: Route = { name: "terminal", agent: agentRow({ pane_id: "w1:p1" }) };
    expect(pathForRoute(route)).toBe("/terminal/w1:p1");
  });

  it("still escapes '/' in a pane_id so it can't be mistaken for a path boundary", () => {
    const route: Route = { name: "terminal", agent: agentRow({ pane_id: "w1/p1" }) };
    expect(pathForRoute(route)).toBe("/terminal/w1%2Fp1");
  });

  it("builds /login and /switcher for login and switcher routes (D1)", () => {
    expect(pathForRoute({ name: "login" })).toBe("/login");
    expect(pathForRoute({ name: "switcher" })).toBe("/switcher");
  });
});

describe("parseRoutePath", () => {
  it("recognizes /login and /switcher", () => {
    expect(parseRoutePath("/login")).toEqual({ name: "login" });
    expect(parseRoutePath("/switcher")).toEqual({ name: "switcher" });
  });

  it("recognizes /terminal/<pane_id>, delegating to parseTerminalPaneId", () => {
    expect(parseRoutePath("/terminal/w1:p1")).toEqual({ name: "terminal", paneId: "w1:p1" });
  });

  it("returns null for '/' and any other unrecognized path (D2)", () => {
    expect(parseRoutePath("/")).toBeNull();
    expect(parseRoutePath("/nope")).toBeNull();
    expect(parseRoutePath("/terminal/")).toBeNull();
    expect(parseRoutePath("/terminal/%")).toBeNull();
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

  it("returns null instead of throwing on a malformed percent-encoded segment (P1 review fix)", () => {
    expect(parseTerminalPaneId("/terminal/%")).toBeNull();
    expect(parseTerminalPaneId("/terminal/%E4%B8")).toBeNull();
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

describe("resolveBootstrapDecision", () => {
  const snapshot: AgentsResponse = {
    agents: [agentRow({ pane_id: "w1:p1" })],
    shells: [shellRow({ pane_id: "w2:p9" })],
  };

  it("redirects an authenticated /login visit straight to switcher, never showing login (D3)", () => {
    expect(resolveBootstrapDecision({ name: "login" }, snapshot)).toEqual({
      route: { name: "switcher" },
      intendedPaneId: null,
      canonicalize: true,
    });
  });

  it("lands an authenticated /switcher or unrecognized/'/' visit on switcher (D2, D4)", () => {
    for (const parsed of [{ name: "switcher" as const }, null]) {
      expect(resolveBootstrapDecision(parsed, snapshot)).toEqual({
        route: { name: "switcher" },
        intendedPaneId: null,
        canonicalize: true,
      });
    }
  });

  it("shows login and canonicalizes to /login for an unauthenticated /login, /switcher, or unrecognized/'/' visit (D5)", () => {
    for (const parsed of [{ name: "login" as const }, { name: "switcher" as const }, null]) {
      expect(resolveBootstrapDecision(parsed, null)).toEqual({
        route: { name: "login" },
        intendedPaneId: null,
        canonicalize: true,
      });
    }
  });

  it("redirects an authenticated /terminal/<pane_id> visit into that pane, unchanged from PBI-025", () => {
    expect(resolveBootstrapDecision({ name: "terminal", paneId: "w1:p1" }, snapshot)).toEqual({
      route: { name: "terminal", agent: agentRow({ pane_id: "w1:p1" }) },
      intendedPaneId: null,
      canonicalize: true,
    });
  });

  it("falls back to switcher for an authenticated /terminal/<pane_id> visit that no longer resolves", () => {
    expect(resolveBootstrapDecision({ name: "terminal", paneId: "gone" }, snapshot)).toEqual({
      route: { name: "switcher" },
      intendedPaneId: null,
      canonicalize: true,
    });
  });

  it("shows login without touching the URL for an unauthenticated /terminal/<pane_id> visit (PBI-025 D5 carve-out, unchanged)", () => {
    expect(resolveBootstrapDecision({ name: "terminal", paneId: "w1:p1" }, null)).toEqual({
      route: { name: "login" },
      intendedPaneId: "w1:p1",
      canonicalize: false,
    });
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
    expect(pushSpy).toHaveBeenCalledWith({ route: { name: "terminal", agent: agentRow({ pane_id: "w1:p1" }) } }, "", "/terminal/w1:p1");
    expect(location.pathname).toBe("/terminal/w1:p1");
  });

  it("replaces instead of pushing when navigating into login, even from a different path (D7)", () => {
    history.replaceState({ route: { name: "switcher" } }, "", "/switcher");
    const pushSpy = vi.spyOn(history, "pushState");
    const replaceSpy = vi.spyOn(history, "replaceState");
    navigate({ name: "login" });
    expect(pushSpy).not.toHaveBeenCalled();
    expect(replaceSpy).toHaveBeenCalledWith({ route: { name: "login" } }, "", "/login");
  });

  it("replaces instead of pushing when navigating away from login, even to a different path (D7)", () => {
    history.replaceState({ route: { name: "login" } }, "", "/login");
    const pushSpy = vi.spyOn(history, "pushState");
    const replaceSpy = vi.spyOn(history, "replaceState");
    navigate({ name: "switcher" });
    expect(pushSpy).not.toHaveBeenCalled();
    expect(replaceSpy).toHaveBeenCalledWith({ route: { name: "switcher" } }, "", "/switcher");
  });

  it("still pushes for a normal switcher <-> terminal transition, unaffected by D7 (D6)", () => {
    history.replaceState({ route: { name: "switcher" } }, "", "/switcher");
    const pushSpy = vi.spyOn(history, "pushState");
    navigate({ name: "terminal", agent: agentRow({ pane_id: "w1:p1" }) });
    expect(pushSpy).toHaveBeenCalledWith(
      { route: { name: "terminal", agent: agentRow({ pane_id: "w1:p1" }) } },
      "",
      "/terminal/w1:p1",
    );
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

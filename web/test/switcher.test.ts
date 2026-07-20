import { describe, it, expect, vi, afterEach } from "vitest";
import { groupByWorkspace, kindAccentColor, renderSwitcher } from "../src/views/switcher";
import type { AgentRow } from "../src/api";

function row(overrides: Partial<AgentRow>): AgentRow {
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

describe("groupByWorkspace", () => {
  it("returns an empty array for an empty input", () => {
    expect(groupByWorkspace([])).toEqual([]);
  });

  it("returns exactly 1 group when all rows share one workspace_id", () => {
    const rows = [
      row({ pane_id: "w1:p1", workspace: "w1" }),
      row({ pane_id: "w1:p2", workspace: "w1" }),
    ];
    const groups = groupByWorkspace(rows);
    expect(groups).toHaveLength(1);
    expect(groups[0].workspace_id).toBe("w1");
    expect(groups[0].rows).toEqual(rows);
  });

  it("groups 2+ distinct workspace_ids, sorted alphabetically by workspace_label", () => {
    const zebra = row({ pane_id: "w2:p1", workspace: "w2", workspace_label: "zebra" });
    const alphaA = row({ pane_id: "w1:p1", workspace: "w1", workspace_label: "alpha" });
    const alphaB = row({ pane_id: "w1:p2", workspace: "w1", workspace_label: "alpha" });

    const groups = groupByWorkspace([zebra, alphaA, alphaB]);

    expect(groups).toHaveLength(2);
    expect(groups.map((g) => g.workspace_label)).toEqual(["alpha", "zebra"]);
    expect(groups[0].workspace_id).toBe("w1");
    expect(groups[0].rows).toEqual([alphaA, alphaB]);
    expect(groups[1].workspace_id).toBe("w2");
    expect(groups[1].rows).toEqual([zebra]);
  });
});

describe("kindAccentColor", () => {
  it("returns the same color for the same kind across multiple calls", () => {
    expect(kindAccentColor("claude")).toBe(kindAccentColor("claude"));
    expect(kindAccentColor("codex")).toBe(kindAccentColor("codex"));
  });

  it("returns a syntactically valid hsl(...) string for a never-seen-before kind", () => {
    expect(kindAccentColor("gpt5")).toMatch(/^hsl\(\d{1,3}, \d{1,3}%, \d{1,3}%\)$/);
    expect(kindAccentColor("unknown-agent")).toMatch(/^hsl\(\d{1,3}, \d{1,3}%, \d{1,3}%\)$/);
  });
});

describe("renderSwitcher health-dot", () => {
  const originalFetch = globalThis.fetch;

  afterEach(() => {
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  it("includes the backend version in the health-dot's title once health data loads", async () => {
    const health = { version: "9.9.9", protocol: 1, herdr_up: true };
    globalThis.fetch = vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/api/health")) {
        return Promise.resolve(new Response(JSON.stringify(health), { status: 200 }));
      }
      return Promise.resolve(new Response(JSON.stringify([]), { status: 200 }));
    }) as typeof fetch;

    const root = document.createElement("div");
    renderSwitcher(root, { onSelect: () => {}, onLoggedOut: () => {} });

    // Let the pending fetchHealth() promise chain settle before asserting.
    await new Promise((resolve) => setTimeout(resolve, 0));

    const healthDot = root.querySelector<HTMLSpanElement>("#health-dot")!;
    expect(healthDot.getAttribute("title")).toContain(health.version);
  });
});

import { describe, it, expect } from "vitest";
import { groupByWorkspace } from "../src/views/switcher";
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

import { describe, it, expect, vi, afterEach } from "vitest";
import { buildHomeGroups, groupByWorkspace, renderSwitcher } from "../src/views/switcher";
import { kindAccentColor } from "../src/kind-marks";
import type { AgentRow, ShellRow } from "../src/api";
import { renderCreateSheet } from "../src/views/create-sheet";
import type { NewPaneRef } from "../src/main";

vi.mock("../src/views/create-sheet", () => ({
  renderCreateSheet: vi.fn(),
}));

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
    path: null,
    label: null,
    ...overrides,
  };
}

function shell(overrides: Partial<ShellRow>): ShellRow {
  return {
    pane_id: "wB:p1",
    workspace_id: "wB",
    workspace_label: "scratch",
    tab_label: "shell",
    path: "/home/dev/scratch",
    label: null,
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
      return Promise.resolve(new Response(JSON.stringify({ agents: [], shells: [] }), { status: 200 }));
    }) as typeof fetch;

    const root = document.createElement("div");
    renderSwitcher(root, { onSelect: () => {}, onLoggedOut: () => {}, onCreated: () => {} });

    // Let the pending fetchHealth() promise chain settle before asserting.
    await new Promise((resolve) => setTimeout(resolve, 0));

    const healthDot = root.querySelector<HTMLSpanElement>("#health-dot")!;
    expect(healthDot.getAttribute("title")).toContain(health.version);
  });
});

describe("renderSwitcher create FAB (S4, D1)", () => {
  const originalFetch = globalThis.fetch;

  function mockFetch(herdrUp: boolean | null): void {
    globalThis.fetch = vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/api/health")) {
        if (herdrUp === null) return Promise.resolve(new Response("", { status: 500 }));
        const health = { version: "1.0.0", protocol: 1, herdr_up: herdrUp };
        return Promise.resolve(new Response(JSON.stringify(health), { status: 200 }));
      }
      return Promise.resolve(new Response(JSON.stringify({ agents: [], shells: [] }), { status: 200 }));
    }) as typeof fetch;
  }

  afterEach(() => {
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  it("is disabled before the health check resolves and while herdr is unreachable", async () => {
    mockFetch(false);
    const root = document.createElement("div");
    renderSwitcher(root, { onSelect: () => {}, onLoggedOut: () => {}, onCreated: () => {} });

    const fab = root.querySelector<HTMLButtonElement>("#create-fab")!;
    expect(fab.disabled).toBe(true);

    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(fab.disabled).toBe(true);
  });

  it("is enabled once loadHealth's fetchHealth() reports herdr reachable, using the same health call the dot already makes", async () => {
    mockFetch(true);
    const root = document.createElement("div");
    renderSwitcher(root, { onSelect: () => {}, onLoggedOut: () => {}, onCreated: () => {} });

    const fab = root.querySelector<HTMLButtonElement>("#create-fab")!;
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(fab.disabled).toBe(false);
    // Exactly one health probe backs both the dot and the FAB — no second check.
    const healthCalls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls.filter((call) =>
      String(call[0]).includes("/api/health"),
    );
    expect(healthCalls).toHaveLength(1);
  });

  it("stays disabled when fetchHealth() fails outright (null)", async () => {
    mockFetch(null);
    const root = document.createElement("div");
    renderSwitcher(root, { onSelect: () => {}, onLoggedOut: () => {}, onCreated: () => {} });

    const fab = root.querySelector<HTMLButtonElement>("#create-fab")!;
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(fab.disabled).toBe(true);
  });

  it("opens the create sheet on tap without re-fetching or altering the agent list beneath it", async () => {
    mockFetch(true);
    const open = vi.fn();
    const close = vi.fn();
    vi.mocked(renderCreateSheet).mockReturnValue({ open, close });

    const root = document.createElement("div");
    renderSwitcher(root, { onSelect: () => {}, onLoggedOut: () => {}, onCreated: () => {} });
    await new Promise((resolve) => setTimeout(resolve, 0));

    const fetchCallsBeforeTap = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls.length;
    const fab = root.querySelector<HTMLButtonElement>("#create-fab")!;
    fab.click();

    expect(open).toHaveBeenCalledTimes(1);
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls.length).toBe(fetchCallsBeforeTap);
    // The list container is untouched by opening the sheet.
    expect(root.querySelector("#agent-list")).not.toBeNull();
  });

  it("passes onCreated straight through to the create sheet, unmodified", () => {
    mockFetch(true);
    let capturedRef: NewPaneRef | undefined;
    vi.mocked(renderCreateSheet).mockImplementation((_root, props) => {
      props.onCreated({ pane_id: "w1:p9", workspace_id: "w1", label: "herdr-gateway" });
      return { open: vi.fn(), close: vi.fn() };
    });

    const root = document.createElement("div");
    renderSwitcher(root, {
      onSelect: () => {},
      onLoggedOut: () => {},
      onCreated: (ref) => {
        capturedRef = ref;
      },
    });

    expect(capturedRef).toEqual({ pane_id: "w1:p9", workspace_id: "w1", label: "herdr-gateway" });
  });
});

describe("buildHomeGroups", () => {
  it("keeps agents and shells in separate label-sorted groups when their workspace_labels differ", () => {
    const groups = buildHomeGroups(
      [row({ workspace: "w1", workspace_label: "alpha", workspace_status: "working" })],
      [
        shell({ workspace_id: "wB", workspace_label: "zebra", pane_id: "wB:p1" }),
        shell({ workspace_id: "wB", workspace_label: "zebra", pane_id: "wB:p2" }),
      ],
    );

    expect(groups).toHaveLength(2);
    expect(groups.map((g) => g.workspace_label)).toEqual(["alpha", "zebra"]);
    // The agent group keeps its status; it holds only agent rows.
    expect(groups[0].workspace_status).toBe("working");
    expect(groups[0].rows.map((r) => r.type)).toEqual(["agent"]);
    // The shell-only group carries no status and holds only shell rows.
    expect(groups[1].workspace_status).toBeNull();
    expect(groups[1].rows.map((r) => r.type)).toEqual(["shell", "shell"]);
  });

  it("merges a shell group into the agent group sharing its workspace_label", () => {
    const agentRow = row({ workspace: "w1", workspace_label: "herdr-gateway", workspace_status: "working" });
    const shellRow = shell({ workspace_id: "wB", workspace_label: "herdr-gateway", pane_id: "wB:p1" });

    const groups = buildHomeGroups([agentRow], [shellRow]);

    expect(groups).toHaveLength(1);
    expect(groups[0].workspace_label).toBe("herdr-gateway");
    expect(groups[0].rows.map((r) => r.type)).toEqual(["agent", "shell"]);
    // The merged group keeps the agent's own status, never the shell's null.
    expect(groups[0].workspace_status).toBe("working");
  });

  it("merges a shell into the agent group sharing its exact workspace_id, even when the labels differ", () => {
    // hsw-D3 removed: a shell created in a workspace that already has agents
    // now surfaces there directly, keyed on the real workspace_id -- not
    // the label-matching fallback (shellgrp-D1), which is for a genuinely
    // different workspace_id.
    const agentRow = row({ workspace: "w1", workspace_label: "herdr-gateway", workspace_status: "working" });
    const shellRow = shell({ workspace_id: "w1", workspace_label: "stale-label", pane_id: "w1:p9" });

    const groups = buildHomeGroups([agentRow], [shellRow]);

    expect(groups).toHaveLength(1);
    // The agent group's own label wins -- the shell's own workspace_label
    // copy is not consulted once workspace_id already matched.
    expect(groups[0].workspace_label).toBe("herdr-gateway");
    expect(groups[0].rows.map((r) => r.type)).toEqual(["agent", "shell"]);
    expect(groups[0].workspace_status).toBe("working");
  });
});

describe("renderSwitcher shell rows (D1/D2/D5/D6/D7)", () => {
  const originalFetch = globalThis.fetch;

  function mockSnapshot(snapshot: { agents?: AgentRow[]; shells?: ShellRow[] }): void {
    globalThis.fetch = vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/api/health")) {
        const health = { version: "1.0.0", protocol: 1, herdr_up: true };
        return Promise.resolve(new Response(JSON.stringify(health), { status: 200 }));
      }
      if (url.includes("/api/create-options")) {
        // Every workspace named by a fixture row/shell below is offered as
        // its own destination -- the create sheet's own contract (GET
        // /api/create-options: "every workspace as a destination").
        const workspaceIds = new Set([
          ...(snapshot.agents ?? []).map((a) => a.workspace),
          ...(snapshot.shells ?? []).map((s) => s.workspace_id),
        ]);
        const labelFor = new Map([
          ...(snapshot.agents ?? []).map((a): [string, string] => [a.workspace, a.workspace_label]),
          ...(snapshot.shells ?? []).map((s): [string, string] => [s.workspace_id, s.workspace_label]),
        ]);
        const destinations = Array.from(workspaceIds).map((workspace_id) => ({
          workspace_id,
          label: labelFor.get(workspace_id)!,
          path: "/home/dev/project",
          path_is_live: true,
        }));
        return Promise.resolve(
          new Response(JSON.stringify({ destinations, presets: [] }), { status: 200 }),
        );
      }
      return Promise.resolve(
        new Response(
          JSON.stringify({ agents: snapshot.agents ?? [], shells: snapshot.shells ?? [] }),
          { status: 200 },
        ),
      );
    }) as typeof fetch;
  }

  function mount(snapshot: { agents?: AgentRow[]; shells?: ShellRow[] }): {
    root: HTMLElement;
    selected: () => AgentRow | NewPaneRef | undefined;
  } {
    mockSnapshot(snapshot);
    let target: AgentRow | NewPaneRef | undefined;
    const root = document.createElement("div");
    renderSwitcher(root, {
      onSelect: (t) => {
        target = t;
      },
      onLoggedOut: () => {},
      onCreated: () => {},
    });
    return { root, selected: () => target };
  }

  afterEach(() => {
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  it("renders a shell pane as its own row: path + tab-label suffix, no status badge, no agent watermark", async () => {
    const { root } = mount({ shells: [shell({ path: "/home/dev/scratch", tab_label: "zsh" })] });
    await new Promise((r) => setTimeout(r, 0));

    expect(root.querySelectorAll(".shell-row")).toHaveLength(1);
    expect(root.querySelector(".shell-row .shell-label")?.textContent).toBe("/home/dev/scratch · zsh");
    expect(root.querySelector(".shell-row .shell-label-suffix")?.textContent).toBe("· zsh");
    expect(root.querySelector(".shell-row .status-badge")).toBeNull();
    expect(root.querySelector(".shell-row .agent-watermark")).toBeNull();
  });

  it("renders a leading mono prompt glyph as the first child of the shell row's button", async () => {
    const { root } = mount({ shells: [shell({ path: "/home/dev/scratch", tab_label: "zsh" })] });
    await new Promise((r) => setTimeout(r, 0));

    const button = root.querySelector(".shell-row");
    expect(button?.querySelector(".shell-prompt")?.textContent).toBe(">");
    expect(button?.firstElementChild?.classList.contains("shell-prompt")).toBe(true);
  });

  it("gives the shell row an accessible name that never announces a status", async () => {
    const { root } = mount({ shells: [shell({ path: "/home/dev/scratch", tab_label: "zsh" })] });
    await new Promise((r) => setTimeout(r, 0));

    expect(root.querySelector(".shell-row")?.getAttribute("aria-label")).toBe(
      "Shell terminal · /home/dev/scratch · zsh",
    );
  });

  it("falls back to 'no folder yet' when a shell pane has no resolved path", async () => {
    const { root } = mount({ shells: [shell({ path: null })] });
    await new Promise((r) => setTimeout(r, 0));

    expect(root.querySelector(".shell-row .shell-label")?.textContent).toBe("no folder yet · shell");
  });

  it("renders 2+ shell panes in the same zero-agent workspace as separate rows, decorative caret on the first only", async () => {
    const { root } = mount({
      shells: [shell({ pane_id: "wB:p1" }), shell({ pane_id: "wB:p2" })],
    });
    await new Promise((r) => setTimeout(r, 0));

    const rows = root.querySelectorAll(".shell-row");
    expect(rows).toHaveLength(2);
    expect(rows[0].querySelector(".shell-caret")).not.toBeNull();
    expect(rows[1].querySelector(".shell-caret")).toBeNull();
  });

  it("gives the header chevron a status class on an agent group while a shell-only group's chevron stays plain", async () => {
    const { root } = mount({
      agents: [row({ workspace: "w1", workspace_label: "alpha", workspace_status: "working" })],
      shells: [shell({ workspace_id: "wB", workspace_label: "zzz-scratch" })],
    });
    await new Promise((r) => setTimeout(r, 0));

    const sections = root.querySelectorAll(".workspace-section");
    expect(sections).toHaveLength(2);
    // "alpha" (agents) sorts before "zzz-scratch" (shells).
    const [agentSection, shellSection] = Array.from(sections);
    // No .status-badge remains in either header (D1).
    expect(agentSection.querySelector(".workspace-header .status-badge")).toBeNull();
    expect(shellSection.querySelector(".workspace-header .status-badge")).toBeNull();
    // The agent group's chevron carries its status decor class...
    const agentChevronWrap = agentSection.querySelector(".workspace-header .workspace-chevron-wrap");
    expect(agentChevronWrap?.classList.contains("status-working")).toBe(true);
    // ...while the shell-only group's chevron stays plain, no status class at all.
    const shellChevronWrap = shellSection.querySelector(".workspace-header .workspace-chevron-wrap");
    expect(shellChevronWrap).not.toBeNull();
    expect(Array.from(shellChevronWrap!.classList)).toEqual(["workspace-chevron-wrap"]);
    // The status label is exposed as accessible text for the agent group only.
    expect(agentSection.querySelector(".workspace-header .sr-only")?.textContent).toBe("Working");
    expect(shellSection.querySelector(".workspace-header .sr-only")).toBeNull();
  });

  it("gives an 'unknown' workspace_status group's chevron the same color+wash treatment as any other status", async () => {
    // A single group renders flat (no header) unless there are 2+ groups, so
    // pair it with a second group to force the workspace-section layout.
    const { root } = mount({
      agents: [
        row({ workspace: "w1", workspace_label: "alpha", workspace_status: "unknown", status: "unknown" }),
        row({ workspace: "w2", workspace_label: "zeta", workspace_status: "idle", pane_id: "w2:p1" }),
      ],
    });
    await new Promise((r) => setTimeout(r, 0));

    const section = Array.from(root.querySelectorAll(".workspace-section")).find((el) =>
      el.querySelector(".workspace-header-label")?.textContent?.includes("alpha"),
    )!;
    const chevronWrap = section.querySelector(".workspace-header .workspace-chevron-wrap");
    expect(chevronWrap?.classList.contains("status-unknown")).toBe(true);
    expect(section.querySelector(".workspace-header .sr-only")?.textContent).toBe("Unknown");
  });

  it("navigates via a NewPaneRef when a shell row is tapped, label = the pane's path", async () => {
    const { root, selected } = mount({
      shells: [shell({ pane_id: "wB:p1", workspace_id: "wB", path: "/home/dev/scratch" })],
    });
    await new Promise((r) => setTimeout(r, 0));

    root.querySelector<HTMLButtonElement>(".shell-row")!.click();
    expect(selected()).toEqual({ pane_id: "wB:p1", workspace_id: "wB", label: "/home/dev/scratch" });
  });

  it("uses workspace_label as the NewPaneRef label when the tapped shell pane has no path", async () => {
    const { root, selected } = mount({
      shells: [shell({ pane_id: "wB:p1", workspace_id: "wB", workspace_label: "scratch", path: null })],
    });
    await new Promise((r) => setTimeout(r, 0));

    root.querySelector<HTMLButtonElement>(".shell-row")!.click();
    expect(selected()).toEqual({ pane_id: "wB:p1", workspace_id: "wB", label: "scratch" });
  });

  describe("group quick-add (+)", () => {
    it("renders one add button per group, each naming its own workspace", async () => {
      const { root } = mount({
        agents: [
          row({ workspace: "w1", workspace_label: "alpha", pane_id: "w1:p1" }),
          row({ workspace: "w2", workspace_label: "beta", pane_id: "w2:p1" }),
        ],
      });
      await new Promise((r) => setTimeout(r, 0));

      const buttons = root.querySelectorAll<HTMLButtonElement>(".workspace-header-add");
      expect(buttons).toHaveLength(2);
      const labels = Array.from(buttons).map((b) => b.getAttribute("aria-label"));
      expect(labels).toContain("New shell or agent in alpha");
      expect(labels).toContain("New shell or agent in beta");
    });

    it("opens the create sheet locked to that group's own workspace_id, not the operator's last pick", async () => {
      // create-sheet.ts is mocked module-wide in this file (line 8) so
      // switcher.ts's own test boundary stops at "calls open() with the
      // right workspace_id" -- what the sheet then does with that lock
      // (hide the picker, retitle itself) is create-sheet.test.ts's job,
      // already covered there.
      const open = vi.fn();
      vi.mocked(renderCreateSheet).mockReturnValue({ open, close: vi.fn() });

      const { root } = mount({
        agents: [
          row({ workspace: "w1", workspace_label: "alpha", pane_id: "w1:p1" }),
          row({ workspace: "w2", workspace_label: "beta", pane_id: "w2:p1" }),
        ],
      });
      await new Promise((r) => setTimeout(r, 0));

      const betaButton = Array.from(root.querySelectorAll<HTMLButtonElement>(".workspace-header-add")).find(
        (b) => b.getAttribute("aria-label") === "New shell or agent in beta",
      )!;
      betaButton.click();

      expect(open).toHaveBeenCalledTimes(1);
      expect(open).toHaveBeenCalledWith("w2");
    });

    it("does not also toggle the group's own collapsed state", async () => {
      vi.mocked(renderCreateSheet).mockReturnValue({ open: vi.fn(), close: vi.fn() });
      const { root } = mount({
        agents: [
          row({ workspace: "w1", workspace_label: "alpha", pane_id: "w1:p1" }),
          row({ workspace: "w2", workspace_label: "beta", pane_id: "w2:p1" }),
        ],
      });
      await new Promise((r) => setTimeout(r, 0));

      const toggle = root.querySelector<HTMLButtonElement>(".workspace-header-toggle")!;
      const addButton = root.querySelector<HTMLButtonElement>(".workspace-header-add")!;
      expect(toggle.getAttribute("aria-expanded")).toBe("true");

      addButton.click();

      expect(toggle.getAttribute("aria-expanded")).toBe("true"); // unchanged
    });

    it("collapse/expand still works after the header split into two buttons", async () => {
      const { root } = mount({
        agents: [
          row({ workspace: "w1", workspace_label: "alpha", pane_id: "w1:p1" }),
          row({ workspace: "w2", workspace_label: "beta", pane_id: "w2:p1" }),
        ],
      });
      await new Promise((r) => setTimeout(r, 0));

      const toggle = root.querySelector<HTMLButtonElement>(".workspace-header-toggle")!;
      const rowsList = toggle.closest(".workspace-section")!.querySelector<HTMLUListElement>(".workspace-rows")!;
      expect(rowsList.hidden).toBe(false);

      toggle.click();

      expect(rowsList.hidden).toBe(true);
      expect(toggle.getAttribute("aria-expanded")).toBe("false");
    });
  });
});

describe("renderSwitcher agent card path", () => {
  const originalFetch = globalThis.fetch;

  function mount(snapshot: { agents?: AgentRow[]; shells?: ShellRow[] }): HTMLElement {
    globalThis.fetch = vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/api/health")) {
        return Promise.resolve(
          new Response(JSON.stringify({ version: "1.0.0", protocol: 1, herdr_up: true }), { status: 200 }),
        );
      }
      return Promise.resolve(
        new Response(JSON.stringify({ agents: snapshot.agents ?? [], shells: snapshot.shells ?? [] }), {
          status: 200,
        }),
      );
    }) as typeof fetch;
    const root = document.createElement("div");
    renderSwitcher(root, { onSelect: () => {}, onLoggedOut: () => {}, onCreated: () => {} });
    return root;
  }

  afterEach(() => {
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  it("shows the agent's own folder path on its card", async () => {
    const root = mount({ agents: [row({ path: "/home/dev/projects/herdr-gateway" })] });
    await new Promise((r) => setTimeout(r, 0));

    expect(root.querySelector(".agent-card .agent-path")?.textContent).toBe("/home/dev/projects/herdr-gateway");
  });

  it("renders no path element at all when the agent's path is unresolved", async () => {
    const root = mount({ agents: [row({ path: null })] });
    await new Promise((r) => setTimeout(r, 0));

    expect(root.querySelector(".agent-card .agent-path")).toBeNull();
  });

  it("shows the operator's own label instead of the title on an agent card", async () => {
    const root = mount({ agents: [row({ title: "building", label: "API fix" })] });
    await new Promise((r) => setTimeout(r, 0));

    expect(root.querySelector(".agent-card .agent-title")?.textContent).toBe("API fix");
  });

  it("falls back to the title when no label is set", async () => {
    const root = mount({ agents: [row({ title: "building", label: null })] });
    await new Promise((r) => setTimeout(r, 0));

    expect(root.querySelector(".agent-card .agent-title")?.textContent).toBe("building");
  });

  it("shows the operator's own label instead of the path on a shell row", async () => {
    const root = mount({
      shells: [shell({ path: "/home/dev/scratch", tab_label: "zsh", label: "scratch box" })],
    });
    await new Promise((r) => setTimeout(r, 0));

    expect(root.querySelector(".shell-row .shell-label")?.textContent).toBe("scratch box · zsh");
  });
});

describe("renderSwitcher 'Needs you' attention section", () => {
  const originalFetch = globalThis.fetch;

  function mount(snapshot: { agents?: AgentRow[]; shells?: ShellRow[] }): {
    root: HTMLElement;
    selected: () => AgentRow | NewPaneRef | undefined;
  } {
    globalThis.fetch = vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/api/health")) {
        return Promise.resolve(
          new Response(JSON.stringify({ version: "1.0.0", protocol: 1, herdr_up: true }), { status: 200 }),
        );
      }
      return Promise.resolve(
        new Response(JSON.stringify({ agents: snapshot.agents ?? [], shells: snapshot.shells ?? [] }), {
          status: 200,
        }),
      );
    }) as typeof fetch;
    let target: AgentRow | NewPaneRef | undefined;
    const root = document.createElement("div");
    renderSwitcher(root, {
      onSelect: (t) => {
        target = t;
      },
      onLoggedOut: () => {},
      onCreated: () => {},
    });
    return { root, selected: () => target };
  }

  afterEach(() => {
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  it("renders no attention section at all when no agent is blocked", async () => {
    const { root } = mount({
      agents: [row({ pane_id: "w1:p1", workspace: "w1", status: "working" })],
    });
    await new Promise((r) => setTimeout(r, 0));

    expect(root.querySelector(".attention-group")).toBeNull();
  });

  it("hoists a blocked agent into its own section above the workspace groups", async () => {
    const { root } = mount({
      agents: [
        row({ pane_id: "w1:p1", workspace: "w1", workspace_label: "alpha", status: "working" }),
        row({ pane_id: "w2:p1", workspace: "w2", workspace_label: "beta", status: "blocked" }),
      ],
    });
    await new Promise((r) => setTimeout(r, 0));

    const attention = root.querySelector(".attention-group");
    expect(attention).not.toBeNull();
    expect(attention?.querySelectorAll(".agent-card")).toHaveLength(1);
    // The attention section is the list's first child, ahead of any workspace group.
    expect(root.querySelector("#agent-list")?.firstElementChild).toBe(attention);
    // The blocked agent still appears a second time, inside its own workspace
    // group below -- context for that project, not a replacement for it (1
    // attention card + 2 workspace cards: the working agent and the blocked
    // agent's own-group copy).
    expect(root.querySelectorAll(".agent-card")).toHaveLength(3);
  });

  it("selects the right agent when tapping its card inside the attention section", async () => {
    const blockedAgent = row({ pane_id: "w2:p1", workspace: "w2", workspace_label: "beta", status: "blocked" });
    const { root, selected } = mount({
      agents: [row({ pane_id: "w1:p1", workspace: "w1", workspace_label: "alpha" }), blockedAgent],
    });
    await new Promise((r) => setTimeout(r, 0));

    root.querySelector<HTMLButtonElement>(".attention-group .agent-card")!.click();
    expect(selected()).toEqual(blockedAgent);
  });
});

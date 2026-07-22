import { describe, it, expect, vi, afterEach } from "vitest";
import { buildHomeGroups, groupByWorkspace, kindAccentColor, renderSwitcher } from "../src/views/switcher";
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

  it("renders a shell pane as its own row: path primary, 'Shell · <tab>' caption, no status badge, no watermark", async () => {
    const { root } = mount({ shells: [shell({ path: "/home/dev/scratch", tab_label: "zsh" })] });
    await new Promise((r) => setTimeout(r, 0));

    expect(root.querySelectorAll(".shell-row")).toHaveLength(1);
    expect(root.querySelector(".shell-row .agent-path")?.textContent).toBe("/home/dev/scratch");
    expect(root.querySelector(".shell-row .agent-caption")?.textContent).toBe("Shell · zsh");
    expect(root.querySelector(".shell-row .status-badge")).toBeNull();
    expect(root.querySelector(".shell-row .agent-watermark")).toBeNull();
  });

  it("renders a leading .shell-icon as the first child of the shell row's button (D3)", async () => {
    const { root } = mount({ shells: [shell({ path: "/home/dev/scratch", tab_label: "zsh" })] });
    await new Promise((r) => setTimeout(r, 0));

    const button = root.querySelector(".shell-row");
    expect(button?.querySelector(".shell-icon")).not.toBeNull();
    expect(button?.firstElementChild?.classList.contains("shell-icon")).toBe(true);
  });

  it("falls back to 'no folder yet' when a shell pane has no resolved path", async () => {
    const { root } = mount({ shells: [shell({ path: null })] });
    await new Promise((r) => setTimeout(r, 0));

    expect(root.querySelector(".shell-row .agent-path")?.textContent).toBe("no folder yet");
  });

  it("renders 2+ shell panes in the same zero-agent workspace as separate rows", async () => {
    const { root } = mount({
      shells: [shell({ pane_id: "wB:p1" }), shell({ pane_id: "wB:p2" })],
    });
    await new Promise((r) => setTimeout(r, 0));

    expect(root.querySelectorAll(".shell-row")).toHaveLength(2);
  });

  it("hides the header badge on a shell-only group while an agent group keeps its badge", async () => {
    const { root } = mount({
      agents: [row({ workspace: "w1", workspace_label: "alpha", workspace_status: "working" })],
      shells: [shell({ workspace_id: "wB", workspace_label: "zzz-scratch" })],
    });
    await new Promise((r) => setTimeout(r, 0));

    const sections = root.querySelectorAll(".workspace-section");
    expect(sections).toHaveLength(2);
    // "alpha" (agents) sorts before "zzz-scratch" (shells).
    const [agentSection, shellSection] = Array.from(sections);
    expect(agentSection.querySelector(".workspace-header .status-badge")).not.toBeNull();
    expect(shellSection.querySelector(".workspace-header .status-badge")).toBeNull();
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
});

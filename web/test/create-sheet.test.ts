import { describe, it, expect, vi, afterEach } from "vitest";
import { renderCreateSheet } from "../src/views/create-sheet";
import type { NewPaneRef } from "../src/main";

const CREATE_OPTIONS = {
  destinations: [
    { workspace_id: "ws-1", label: "herdr-gateway", path: "/home/op/herdr-gateway", path_is_live: true },
    { workspace_id: "ws-2", label: "no-folder", path: null, path_is_live: false },
    { workspace_id: "ws-3", label: "stale-folder", path: "/home/op/stale", path_is_live: false },
  ],
  presets: [{ label: "claude" }, { label: "codex" }],
};

function settle(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

function mockFetch(handlers: {
  createOptions?: () => Response;
  createPane?: () => Response | Promise<Response>;
  createAgent?: () => Response | Promise<Response>;
}): ReturnType<typeof vi.fn> {
  const fn = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input);
    const method = init?.method ?? "GET";
    if (url.includes("/api/create-options")) {
      return Promise.resolve((handlers.createOptions ?? (() => new Response(JSON.stringify(CREATE_OPTIONS), { status: 200 })))());
    }
    if (url.includes("/api/panes") && method === "POST") {
      return Promise.resolve(handlers.createPane ? handlers.createPane() : new Response(JSON.stringify({ tab_id: "t1", pane_id: "p1" }), { status: 200 }));
    }
    if (url.includes("/api/agents") && method === "POST") {
      return Promise.resolve(
        handlers.createAgent ? handlers.createAgent() : new Response(JSON.stringify({ tab_id: "t1", pane_id: "p1", name: "claude-abc" }), { status: 200 }),
      );
    }
    return Promise.resolve(new Response("not found", { status: 404 }));
  });
  globalThis.fetch = fn as unknown as typeof fetch;
  return fn;
}

/** Opens the Destination dropdown, selects the row at `index`, closing it again. */
function selectDestination(root: HTMLElement, index: number): void {
  root.querySelector<HTMLButtonElement>("#destination-trigger")!.click();
  root.querySelectorAll<HTMLButtonElement>(".destination-row")[index].click();
}

/** Opens the Type dropdown and selects the row with the given data-kind/data-preset. */
function selectType(root: HTMLElement, selector: string): void {
  root.querySelector<HTMLButtonElement>("#type-trigger")!.click();
  root.querySelector<HTMLButtonElement>(selector)!.click();
}

describe("renderCreateSheet", () => {
  const originalFetch = globalThis.fetch;

  afterEach(() => {
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  it("lists every destination, including one with a null path and one that is not live, each with a caveat", async () => {
    mockFetch({});
    const root = document.createElement("div");
    const controls = renderCreateSheet(root, { onCreated: () => {} });
    controls.open();
    await settle();

    root.querySelector<HTMLButtonElement>("#destination-trigger")!.click();
    const rows = root.querySelectorAll<HTMLButtonElement>(".destination-row");
    expect(rows).toHaveLength(3);

    const normal = rows[0];
    const missingPath = rows[1];
    const stalePath = rows[2];

    expect(normal.querySelector(".destination-caveat")).toBeNull();
    expect(missingPath.querySelector(".destination-caveat")).not.toBeNull();
    expect(stalePath.querySelector(".destination-caveat")).not.toBeNull();
  });

  it("appends a workspace_id suffix only to destinations colliding on label+path, including two with a null path", async () => {
    mockFetch({
      createOptions: () =>
        new Response(
          JSON.stringify({
            destinations: [
              { workspace_id: "ws-aaaa1111", label: "herdr-gateway", path: "/home/op/herdr-gateway", path_is_live: true },
              { workspace_id: "ws-bbbb2222", label: "herdr-gateway", path: "/home/op/herdr-gateway", path_is_live: true },
              { workspace_id: "ws-cccc3333", label: "no-folder", path: null, path_is_live: false },
              { workspace_id: "ws-dddd4444", label: "no-folder", path: null, path_is_live: false },
              { workspace_id: "ws-eeee5555", label: "unique", path: "/home/op/unique", path_is_live: true },
            ],
            presets: [],
          }),
          { status: 200 },
        ),
    });
    const root = document.createElement("div");
    const controls = renderCreateSheet(root, { onCreated: () => {} });
    controls.open();
    await settle();

    root.querySelector<HTMLButtonElement>("#destination-trigger")!.click();
    const labels = root.querySelectorAll<HTMLSpanElement>(".destination-label");
    expect(labels).toHaveLength(5);
    expect(labels[0].textContent).toBe("herdr-gateway · 1111");
    expect(labels[1].textContent).toBe("herdr-gateway · 2222");
    expect(labels[2].textContent).toBe("no-folder · 3333");
    expect(labels[3].textContent).toBe("no-folder · 4444");
    expect(labels[4].textContent).toBe("unique");
  });

  it("renders Shell first, then one row per preset in API order", async () => {
    mockFetch({});
    const root = document.createElement("div");
    const controls = renderCreateSheet(root, { onCreated: () => {} });
    controls.open();
    await settle();

    root.querySelector<HTMLButtonElement>("#type-trigger")!.click();
    const actionRows = root.querySelectorAll<HTMLButtonElement>(".action-row");
    expect(actionRows).toHaveLength(3);
    expect(actionRows[0].dataset.kind).toBe("shell");
    expect(actionRows[1].dataset.preset).toBe("claude");
    expect(actionRows[2].dataset.preset).toBe("codex");
  });

  it("calls createPane with the selected destination and invokes onCreated with a NewPaneRef built from the response", async () => {
    mockFetch({});
    const root = document.createElement("div");
    let created: NewPaneRef | null = null;
    const controls = renderCreateSheet(root, { onCreated: (ref) => (created = ref) });
    controls.open();
    await settle();

    selectDestination(root, 1);
    selectType(root, '.action-row[data-kind="shell"]');
    root.querySelector<HTMLButtonElement>("#create-sheet-new")!.click();
    await settle();

    expect(created).toEqual({ pane_id: "p1", workspace_id: "ws-2", label: "no-folder" });
  });

  it("calls createAgent with the destination and preset, and invokes onCreated with the generated name", async () => {
    mockFetch({});
    const root = document.createElement("div");
    let created: NewPaneRef | null = null;
    const controls = renderCreateSheet(root, { onCreated: (ref) => (created = ref) });
    controls.open();
    await settle();

    selectDestination(root, 0);
    selectType(root, '.action-row[data-preset="claude"]');
    root.querySelector<HTMLButtonElement>("#create-sheet-new")!.click();
    await settle();

    expect(created).toEqual({
      pane_id: "p1",
      workspace_id: "ws-1",
      label: "herdr-gateway",
      name: "claude-abc",
    });
  });

  it("does not fire a second overlapping request when New is tapped twice before the first resolves", async () => {
    let resolvePane: (res: Response) => void = () => {};
    const pending = new Promise<Response>((resolve) => {
      resolvePane = resolve;
    });
    const fetchMock = mockFetch({ createPane: () => pending });

    const root = document.createElement("div");
    const controls = renderCreateSheet(root, { onCreated: () => {} });
    controls.open();
    await settle();

    const newBtn = root.querySelector<HTMLButtonElement>("#create-sheet-new")!;
    newBtn.click();
    newBtn.click();
    newBtn.click();

    const paneCalls = fetchMock.mock.calls.filter(([input]) => String(input).includes("/api/panes"));
    expect(paneCalls).toHaveLength(1);

    resolvePane(new Response(JSON.stringify({ tab_id: "t1", pane_id: "p1" }), { status: 200 }));
    await settle();
  });

  it("renders a create-call error inline and keeps the sheet open and usable", async () => {
    mockFetch({
      createPane: () => new Response(JSON.stringify({ error: "workspace closed" }), { status: 409 }),
    });
    const root = document.createElement("div");
    let created: NewPaneRef | null = null;
    const controls = renderCreateSheet(root, { onCreated: (ref) => (created = ref) });
    controls.open();
    await settle();

    root.querySelector<HTMLButtonElement>("#create-sheet-new")!.click();
    await settle();

    const sheet = root.querySelector<HTMLDivElement>("#create-sheet")!;
    expect(sheet.hidden).toBe(false);
    expect(created).toBeNull();

    const errorEl = root.querySelector<HTMLParagraphElement>("#create-sheet-error")!;
    expect(errorEl.hidden).toBe(false);
    expect(errorEl.textContent).toContain("workspace closed");

    const newBtn = root.querySelector<HTMLButtonElement>("#create-sheet-new")!;
    expect(newBtn.disabled).toBe(false);
  });

  it("closes the Destination dropdown when the Type dropdown is opened while it was open (D7)", async () => {
    mockFetch({});
    const root = document.createElement("div");
    const controls = renderCreateSheet(root, { onCreated: () => {} });
    controls.open();
    await settle();

    root.querySelector<HTMLButtonElement>("#destination-trigger")!.click();
    expect(root.querySelector<HTMLUListElement>("#destination-list")!.hidden).toBe(false);

    root.querySelector<HTMLButtonElement>("#type-trigger")!.click();
    expect(root.querySelector<HTMLUListElement>("#destination-list")!.hidden).toBe(true);
    expect(root.querySelector<HTMLUListElement>("#action-list")!.hidden).toBe(false);
  });
});

import { describe, it, expect, vi, afterEach } from "vitest";
import {
  computeKeyboardInset,
  terminalHead,
  preserveScrollTop,
  pinchFontSize,
  sanitizeTypedText,
  screenTailContainsSent,
  renderTerminal,
} from "../src/views/terminal";
import type { AgentRow } from "../src/api";
import type { NewPaneRef } from "../src/main";

// The matchMedia and canvas stubs that used to live here existed only so
// xterm.js could be constructed under jsdom. The renderer builds plain DOM now,
// so neither is needed.

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
    path: "/home/dev/projects/herdr-gateway",
    label: null,
  };

  it("reads an AgentRow's own kind, title (not display), and path unchanged", () => {
    expect(terminalHead(agentRow)).toEqual({
      kind: "claude",
      title: "building",
      path: "/home/dev/projects/herdr-gateway",
    });
  });

  it("derives 'shell' as the kind for a NewPaneRef with no name, path defaulting to null", () => {
    const ref: NewPaneRef = {
      pane_id: "p2",
      workspace_id: "ws-2",
      label: "herdr-gateway",
    };
    expect(terminalHead(ref)).toEqual({
      kind: "shell",
      title: "herdr-gateway",
      path: null,
    });
  });

  it("uses the preset name as the kind for a NewPaneRef with a name, and carries its path", () => {
    const ref: NewPaneRef = {
      pane_id: "p3",
      workspace_id: "ws-3",
      label: "herdr-gateway",
      name: "claude-abc123",
      path: "/home/dev/projects/herdr-gateway",
    };
    expect(terminalHead(ref)).toEqual({
      kind: "claude-abc123",
      title: "herdr-gateway",
      path: "/home/dev/projects/herdr-gateway",
    });
  });
});

describe("preserveScrollTop", () => {
  it("stays glued to the new bottom when the operator was already at the bottom", () => {
    // distanceFromBottom 0 == "following the live tail" -- after content
    // grows, the new bottom-most scrollTop keeps that promise.
    expect(preserveScrollTop(0, 2000, 200)).toBe(1800);
  });

  it("preserves a mid-content reading position when it still fits in the new range", () => {
    // Was 300px above the bottom in a 1000px-tall, 200px-viewport render;
    // content grew to 1400px without touching the operator's own rows.
    expect(preserveScrollTop(300, 1400, 200)).toBe(900);
  });

  it("clamps to 0 (scrolls as far up as the new content allows) when the old position no longer fits", () => {
    // The content shrank (e.g. a poll tick right after a history escalation)
    // so far that the prior distance-from-bottom now exceeds the entire new
    // scrollable range -- must not go negative.
    expect(preserveScrollTop(5000, 300, 200)).toBe(0);
  });

  it("returns 0 when the new content doesn't overflow the viewport at all", () => {
    expect(preserveScrollTop(0, 150, 200)).toBe(0);
  });
});

describe("pinchFontSize", () => {
  it("returns the starting size when the fingers did not move", () => {
    expect(pinchFontSize(12, 1)).toBe(12);
  });

  it("snaps to whole pixels, the only sizes the renderer uses", () => {
    // 12 * 1.1 == 13.2 -- an in-gesture preview at 13.2px would have to snap
    // somewhere on release; rounding up front means it never does.
    expect(pinchFontSize(12, 1.1)).toBe(13);
    expect(pinchFontSize(12, 1.2)).toBe(14);
  });

  it("clamps to the same ceiling the A+ button stops at", () => {
    expect(pinchFontSize(12, 5)).toBe(22);
  });

  it("clamps to the same floor the A− button stops at", () => {
    expect(pinchFontSize(12, 0.1)).toBe(7);
  });

  it("scales down proportionally within range", () => {
    expect(pinchFontSize(20, 0.5)).toBe(10);
  });
});

describe("sanitizeTypedText", () => {
  it("leaves plain text untouched", () => {
    expect(sanitizeTypedText("hello world")).toBe("hello world");
  });

  it("keeps tab and newline", () => {
    expect(sanitizeTypedText("a\tb\nc")).toBe("a\tb\nc");
  });

  it("strips ESC and BEL a clipboard paste could smuggle in", () => {
    expect(sanitizeTypedText("yes\x1bno\x07")).toBe("yesno");
  });

  it("strips C1 control bytes", () => {
    expect(sanitizeTypedText("a\x9bb")).toBe("ab");
  });
});

describe("screenTailContainsSent", () => {
  it("matches when the sent text is on the last line", () => {
    expect(screenTailContainsSent("line1\nline2\nhello", "hello")).toBe(true);
  });

  it("does not match text scrolled past the tail window", () => {
    const screen = ["hello", "l2", "l3", "l4", "l5", "l6", "l7"].join("\n");
    expect(screenTailContainsSent(screen, "hello")).toBe(false);
  });

  it("treats an empty sent text as already matched (the submit-only call)", () => {
    expect(screenTailContainsSent("anything", "")).toBe(true);
  });

  it("matches a long reply the agent's own pty wrapped across lines (real-device fix)", () => {
    // A pty wraps at its own column width, not the client's -- a reply long
    // enough to wrap lands as two physical lines, the space at the wrap
    // point turned into a newline. A literal substring check never matches
    // this; normalizeForMatch collapses both sides' whitespace the same way.
    const screen = ["l1", "please walk me through the", "authentication flow", "❯ "].join("\n");
    expect(screenTailContainsSent(screen, "please walk me through the authentication flow")).toBe(true);
  });
});

describe("renderTerminal", () => {
  const originalFetch = globalThis.fetch;
  const ROW_PX = 20;

  const agent: AgentRow = {
    pane_id: "w1:p1",
    workspace: "w1",
    display: "claude · herdr",
    kind: "claude",
    status: "working",
    title: "building",
    workspace_label: "herdr",
    tab_label: "herdr",
    workspace_status: "working",
    path: "/home/dev/projects/herdr-gateway",
    label: null,
  };

  afterEach(() => {
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  function settle(): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, 0));
  }

  /** Mocks GET /api/panes/:pane/screen, routing any `?history=<n>` separately from the default (live) path. */
  function mockScreenFetch(handlers: {
    live?: () => Response | Promise<Response>;
    history?: () => Response | Promise<Response>;
    input?: (body: { text: string; submit: boolean }) => Response | Promise<Response>;
    close?: () => Response | Promise<Response>;
    label?: (body: { label: string | null }) => Response | Promise<Response>;
  }): ReturnType<typeof vi.fn> {
    const fn = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (init?.method === "DELETE") {
        return Promise.resolve((handlers.close ?? (() => new Response(null, { status: 200 })))());
      }
      if (url.includes("/label") && init?.method === "PUT") {
        const body = JSON.parse(String(init.body)) as { label: string | null };
        return Promise.resolve((handlers.label ?? (() => new Response(null, { status: 200 })))(body));
      }
      if (url.includes("/input") && init?.method === "POST") {
        const body = JSON.parse(String(init.body)) as { text: string; submit: boolean };
        return Promise.resolve((handlers.input ?? (() => new Response(null, { status: 200 })))(body));
      }
      if (url.includes("history=")) {
        return Promise.resolve(
          (handlers.history ??
            (() => new Response(JSON.stringify({ text: "history\n❯ ", revision: 2 }), { status: 200 })))(),
        );
      }
      if (url.includes("/screen")) {
        return Promise.resolve(
          (handlers.live ?? (() => new Response(JSON.stringify({ text: "live\n❯ ", revision: 1 }), { status: 200 })))(),
        );
      }
      return Promise.resolve(new Response("not found", { status: 404 }));
    });
    globalThis.fetch = fn as unknown as typeof fetch;
    return fn;
  }

  function mountTerminal(onBack: () => void = () => {}): HTMLDivElement {
    const root = document.createElement("div");
    document.body.appendChild(root);
    renderTerminal(root, { agent, onBack });
    return root.querySelector<HTMLDivElement>("#term-viewport")!;
  }

  describe("pane rename (herdr's pane.rename)", () => {
    it("shows the operator's own label instead of the title, in both the footer and the floating header", () => {
      const root = document.createElement("div");
      renderTerminal(root, { agent: { ...agent, label: "API fix" }, onBack: () => {} });

      expect(root.querySelector("#term-name")?.textContent).toBe("API fix");
      expect(root.querySelector("#term-header-name")?.textContent).toBe("API fix");
    });

    it("falls back to the title when no label is set", () => {
      const root = document.createElement("div");
      renderTerminal(root, { agent, onBack: () => {} });

      expect(root.querySelector("#term-name")?.textContent).toBe(agent.title);
    });

    it("the rename button reveals an input pre-filled with the current label, hiding the name", () => {
      const root = document.createElement("div");
      renderTerminal(root, { agent: { ...agent, label: "API fix" }, onBack: () => {} });

      root.querySelector<HTMLButtonElement>("#term-rename-btn")!.click();

      const name = root.querySelector<HTMLSpanElement>("#term-name")!;
      const input = root.querySelector<HTMLInputElement>("#term-name-input")!;
      expect(name.hidden).toBe(true);
      expect(input.hidden).toBe(false);
      expect(input.value).toBe("API fix");
    });

    it("Enter commits the typed label via PUT /api/panes/:pane/label and updates the display", async () => {
      const fetchMock = mockScreenFetch({});
      const root = document.createElement("div");
      renderTerminal(root, { agent, onBack: () => {} });

      root.querySelector<HTMLButtonElement>("#term-rename-btn")!.click();
      const input = root.querySelector<HTMLInputElement>("#term-name-input")!;
      input.value = "New Label";
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));
      await settle();

      const call = fetchMock.mock.calls.find(([url]) => String(url).includes("/label"));
      expect(call).toBeDefined();
      const [, init] = call!;
      expect(init.method).toBe("PUT");
      expect(JSON.parse(String(init.body))).toEqual({ label: "New Label" });
      expect(root.querySelector("#term-name")?.textContent).toBe("New Label");
      expect(root.querySelector<HTMLInputElement>("#term-name-input")!.hidden).toBe(true);
    });

    it("Escape cancels without calling the API, restoring the original name", async () => {
      const fetchMock = mockScreenFetch({});
      const root = document.createElement("div");
      renderTerminal(root, { agent, onBack: () => {} });

      root.querySelector<HTMLButtonElement>("#term-rename-btn")!.click();
      const input = root.querySelector<HTMLInputElement>("#term-name-input")!;
      input.value = "Discarded";
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
      await settle();

      expect(fetchMock.mock.calls.some(([url]) => String(url).includes("/label"))).toBe(false);
      expect(root.querySelector("#term-name")?.textContent).toBe(agent.title);
      expect(root.querySelector<HTMLSpanElement>("#term-name")!.hidden).toBe(false);
    });

    it("clearing the input to empty text sends label: null (clears the operator label)", async () => {
      const fetchMock = mockScreenFetch({});
      const root = document.createElement("div");
      renderTerminal(root, { agent: { ...agent, label: "API fix" }, onBack: () => {} });

      root.querySelector<HTMLButtonElement>("#term-rename-btn")!.click();
      const input = root.querySelector<HTMLInputElement>("#term-name-input")!;
      input.value = "   ";
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));
      await settle();

      const call = fetchMock.mock.calls.find(([url]) => String(url).includes("/label"));
      expect(JSON.parse(String(call![1].body))).toEqual({ label: null });
      // Falls back to the title once the label is cleared.
      expect(root.querySelector("#term-name")?.textContent).toBe(agent.title);
    });

    it("blur commits the typed value, same as Enter", async () => {
      const fetchMock = mockScreenFetch({});
      const root = document.createElement("div");
      renderTerminal(root, { agent, onBack: () => {} });

      root.querySelector<HTMLButtonElement>("#term-rename-btn")!.click();
      const input = root.querySelector<HTMLInputElement>("#term-name-input")!;
      input.value = "Via Blur";
      input.dispatchEvent(new Event("blur"));
      await settle();

      const call = fetchMock.mock.calls.find(([url]) => String(url).includes("/label"));
      expect(JSON.parse(String(call![1].body))).toEqual({ label: "Via Blur" });
      expect(root.querySelector("#term-name")?.textContent).toBe("Via Blur");
    });
  });

  /**
   * jsdom computes no layout, so scrollHeight/clientHeight always read 0.
   * Override them on the actual #term-viewport node: clientHeight is a fixed
   * mock "visible" height, and scrollHeight is derived from the real rendered
   * line count -- so it stays in sync with whatever applyScreen actually
   * rendered, the same way a real browser's layout would.
   */
  function mockViewportMetrics(viewport: HTMLDivElement, clientHeight: number): void {
    Object.defineProperty(viewport, "clientHeight", { configurable: true, get: () => clientHeight });
    Object.defineProperty(viewport, "scrollHeight", {
      configurable: true,
      get: () => rowCount(viewport) * ROW_PX,
    });
  }

  /**
   * Rendered line count. Lines live in one <pre> separated by newline text
   * nodes (blank lines then keep their height for free), so the newlines are
   * what there is to count.
   */
  function rowCount(viewport: HTMLDivElement): number {
    const text = viewport.querySelector(".term-screen")?.textContent ?? "";
    return text === "" ? 0 : text.split("\n").length;
  }

  it("fires exactly one history request when scrolled to the top, even across several scroll events", async () => {
    const fetchMock = mockScreenFetch({});
    const viewport = mountTerminal();
    await settle();

    viewport.scrollTop = 0;
    viewport.dispatchEvent(new Event("scroll"));
    viewport.dispatchEvent(new Event("scroll"));
    viewport.dispatchEvent(new Event("scroll"));
    await settle();

    const historyCalls = fetchMock.mock.calls.filter(([input]) => String(input).includes("history=1"));
    expect(historyCalls).toHaveLength(1);
  });

  it("re-arms the trigger only after scrolling away from the top and back", async () => {
    const fetchMock = mockScreenFetch({});
    const viewport = mountTerminal();
    await settle();

    viewport.scrollTop = 0;
    viewport.dispatchEvent(new Event("scroll"));
    await settle();

    viewport.scrollTop = 50; // scrolls away from the top
    viewport.dispatchEvent(new Event("scroll"));
    viewport.scrollTop = 0; // back to the top
    viewport.dispatchEvent(new Event("scroll"));
    await settle();

    const historyCalls = fetchMock.mock.calls.filter(([input]) => String(input).includes("history="));
    expect(historyCalls).toHaveLength(2);
  });

  it("renders history content beyond the previous 400-row clamp, up to herdr's 1000-line cap", async () => {
    const lines = Array.from({ length: 600 }, (_, i) => `line ${i + 1}`);
    mockScreenFetch({
      history: () => new Response(JSON.stringify({ text: lines.join("\n"), revision: 2 }), { status: 200 }),
    });
    const viewport = mountTerminal();
    await settle();

    viewport.scrollTop = 0;
    viewport.dispatchEvent(new Event("scroll"));
    await settle();

    // Every returned line is rendered: there is no grid to clamp to, so
    // nothing can be discarded for being past a row ceiling. 600, not the
    // grid era's 601 -- that extra row was a blank spare the fixed grid
    // needed, with no counterpart here.
    expect(rowCount(viewport)).toBe(600);
    expect(rowCount(viewport)).toBeGreaterThan(400);
  });

  it("preserves the operator's reading position when a load-older re-render expands the content", async () => {
    const shortText = Array.from({ length: 50 }, (_, i) => `line ${i + 1}`).join("\n");
    const longText = Array.from({ length: 500 }, (_, i) => `line ${i + 1}`).join("\n");
    mockScreenFetch({
      live: () => new Response(JSON.stringify({ text: shortText, revision: 1 }), { status: 200 }),
      history: () => new Response(JSON.stringify({ text: longText, revision: 2 }), { status: 200 }),
    });
    const viewport = mountTerminal();
    mockViewportMetrics(viewport, 200);
    await settle();

    const rowsBefore = rowCount(viewport);
    const scrollHeightBefore = rowsBefore * ROW_PX;
    viewport.scrollTop = 0; // operator scrolled all the way to the top to trigger "load older"
    const distanceFromBottomBefore = scrollHeightBefore - 0 - 200;

    viewport.dispatchEvent(new Event("scroll"));
    await settle();

    const rowsAfter = rowCount(viewport);
    expect(rowsAfter).toBeGreaterThan(rowsBefore);
    const expected = preserveScrollTop(distanceFromBottomBefore, rowsAfter * ROW_PX, 200);
    expect(viewport.scrollTop).toBe(expected);
  });

  it("keeps showing history through the next would-be poll tick instead of reverting (D4 pause)", async () => {
    vi.useFakeTimers();
    const shortText = Array.from({ length: 10 }, (_, i) => `line ${i + 1}`).join("\n");
    const longText = Array.from({ length: 500 }, (_, i) => `line ${i + 1}`).join("\n");
    mockScreenFetch({
      live: () => new Response(JSON.stringify({ text: shortText, revision: 1 }), { status: 200 }),
      history: () => new Response(JSON.stringify({ text: longText, revision: 2 }), { status: 200 }),
    });
    const viewport = mountTerminal();
    mockViewportMetrics(viewport, 200);
    await vi.advanceTimersByTimeAsync(0); // flush the initial (short) poll

    viewport.scrollTop = 0;
    viewport.dispatchEvent(new Event("scroll")); // load-older escalates to the long content, pausing poll
    await vi.advanceTimersByTimeAsync(0);
    expect(rowCount(viewport)).toBeGreaterThan(10);

    await vi.advanceTimersByTimeAsync(1500); // the regular poll tick that used to revert to shortText

    // Still viewing history: poll() paused itself instead of fetching/
    // rendering the live (short) screen, so the long content survives.
    expect(rowCount(viewport)).toBe(500); // longText's 500 lines, one line per line
  });

  it("resumes poll once the reply sheet's forced scroll-to-bottom fires a scroll event (D6)", async () => {
    vi.useFakeTimers();
    const shortText = Array.from({ length: 10 }, (_, i) => `line ${i + 1}`).join("\n");
    const longText = Array.from({ length: 500 }, (_, i) => `line ${i + 1}`).join("\n");
    mockScreenFetch({
      live: () => new Response(JSON.stringify({ text: shortText, revision: 1 }), { status: 200 }),
      history: () => new Response(JSON.stringify({ text: longText, revision: 2 }), { status: 200 }),
    });
    const viewport = mountTerminal();
    mockViewportMetrics(viewport, 200);
    await vi.advanceTimersByTimeAsync(0); // flush the initial (short) poll

    viewport.scrollTop = 0;
    viewport.dispatchEvent(new Event("scroll")); // load-older escalates to the long content, pausing poll
    await vi.advanceTimersByTimeAsync(0);
    expect(rowCount(viewport)).toBeGreaterThan(10);

    const replyOpen = viewport.parentElement!.querySelector<HTMLButtonElement>("#reply-open")!;
    replyOpen.click(); // openReply() -> applySheetInset() forces scrollTop = scrollHeight

    // jsdom doesn't fire a native 'scroll' event for a programmatic scrollTop
    // assignment -- dispatch it manually, the same pattern this file already
    // uses for the top-threshold trigger above.
    viewport.dispatchEvent(new Event("scroll"));
    await vi.advanceTimersByTimeAsync(0); // let the resume's immediate poll() resolve

    expect(rowCount(viewport)).toBe(10); // reverted to live (shortText's 10 lines)
  });

  it("never lets a poll tick interleave with an in-flight history-load request", async () => {
    vi.useFakeTimers();
    let resolveHistory: (res: Response) => void = () => {};
    const historyPending = new Promise<Response>((resolve) => {
      resolveHistory = resolve;
    });
    const fetchMock = mockScreenFetch({ history: () => historyPending });
    const viewport = mountTerminal();
    mockViewportMetrics(viewport, 200);
    await vi.advanceTimersByTimeAsync(0); // flush the initial poll

    const liveCalls = () => fetchMock.mock.calls.filter(([input]) => !String(input).includes("history=")).length;
    const callsBeforeTrigger = liveCalls();

    viewport.scrollTop = 0;
    viewport.dispatchEvent(new Event("scroll")); // starts loadOlder(), historyInFlight becomes true

    await vi.advanceTimersByTimeAsync(1500); // a live poll tick would normally fire here
    expect(liveCalls()).toBe(callsBeforeTrigger); // poll skipped itself while history was in flight

    resolveHistory(new Response(JSON.stringify({ text: "resolved\n❯ ", revision: 3 }), { status: 200 }));
    await vi.advanceTimersByTimeAsync(0); // let loadOlder finish, clearing historyInFlight

    // Still viewing history at this point (nothing returned to live yet) --
    // poll must still be paused. Explicitly return to live (nudge-down) the
    // same way an operator would, rather than relying on scrollTop/
    // scrollHeight both reading 0 in jsdom's default (unmeasured) layout.
    await vi.advanceTimersByTimeAsync(1500);
    expect(liveCalls()).toBe(callsBeforeTrigger);

    const nudgeDown = viewport.parentElement!.querySelector<HTMLButtonElement>("#nudge-down")!;
    nudgeDown.click();
    await vi.advanceTimersByTimeAsync(0);

    await vi.advanceTimersByTimeAsync(1500); // poll resumes on the next tick
    expect(liveCalls()).toBeGreaterThan(callsBeforeTrigger);
  });

  it("renders the scroll-nudge buttons for a non-Claude agent kind (e.g. codex)", async () => {
    mockScreenFetch({});
    const codexAgent: AgentRow = { ...agent, kind: "codex" };
    const root = document.createElement("div");
    document.body.appendChild(root);
    renderTerminal(root, { agent: codexAgent, onBack: () => {} });
    await settle();

    expect(root.querySelector("#nudge-up")).not.toBeNull();
    expect(root.querySelector("#nudge-down")).not.toBeNull();
  });

  it("tapping the up-button (scroll-nudge) triggers exactly one history fetch", async () => {
    const fetchMock = mockScreenFetch({});
    const viewport = mountTerminal();
    await settle();

    const nudgeUp = viewport.parentElement!.querySelector<HTMLButtonElement>("#nudge-up")!;
    nudgeUp.click();
    await settle();

    const historyCalls = fetchMock.mock.calls.filter(([input]) => String(input).includes("history=1"));
    expect(historyCalls).toHaveLength(1);
  });

  it("tapping the up-button from the live bottom actually scrolls into the loaded history", async () => {
    // Regression: a tap from the live bottom (distanceFromBottom ~= 0) used to
    // land back at the bottom of the freshly-expanded content via
    // preserveScrollTop's distance-preservation -- a full re-render with no
    // visible scroll, unlike the drag-scroll-to-top trigger which already
    // moved scrollTop before loadOlder() ran.
    const shortText = Array.from({ length: 10 }, (_, i) => `line ${i + 1}`).join("\n");
    const longText = Array.from({ length: 500 }, (_, i) => `line ${i + 1}`).join("\n");
    const fetchMock = mockScreenFetch({
      live: () => new Response(JSON.stringify({ text: shortText, revision: 1 }), { status: 200 }),
      history: () => new Response(JSON.stringify({ text: longText, revision: 2 }), { status: 200 }),
    });
    const viewport = mountTerminal();
    mockViewportMetrics(viewport, 200);
    await settle();
    viewport.scrollTop = viewport.scrollHeight; // sitting at the live bottom, not pre-scrolled

    const nudgeUp = viewport.parentElement!.querySelector<HTMLButtonElement>("#nudge-up")!;
    nudgeUp.click();
    await settle();

    expect(viewport.scrollTop).toBe(0);

    // jsdom doesn't fire a native 'scroll' event for the programmatic
    // scrollTop=0 assignment above -- dispatch it manually (same pattern this
    // file already uses elsewhere) to prove a real browser's resulting
    // 'scroll' event doesn't re-arm a second, redundant history fetch.
    viewport.dispatchEvent(new Event("scroll"));
    await settle();
    const historyCalls = fetchMock.mock.calls.filter(([input]) => String(input).includes("history=1"));
    expect(historyCalls).toHaveLength(1);
  });

  it("does not mistake a short loaded page for a return to live (regression)", async () => {
    // Real Claude Code pages vary in length hop to hop (CONTEXT.md
    // D-multi-page) -- after nudge-up jumps scrollTop to 0, a page short
    // enough not to overflow the viewport also reads as "distance from
    // bottom near zero" (nothing to scroll), which used to be mistaken for
    // a genuine return-to-live drag: historyDepth silently reset to 0 and
    // poll silently resumed mid-browse, felt as "sometimes jumps back to
    // live on its own".
    const shortText = "short page\n❯ ";
    const fetchMock = mockScreenFetch({
      history: () => new Response(JSON.stringify({ text: shortText, revision: 2 }), { status: 200 }),
    });
    const viewport = mountTerminal();
    mockViewportMetrics(viewport, 200); // clientHeight 200, content far shorter
    await settle();

    const nudgeUp = viewport.parentElement!.querySelector<HTMLButtonElement>("#nudge-up")!;
    nudgeUp.click();
    await settle();
    viewport.dispatchEvent(new Event("scroll")); // jsdom needs this dispatched manually

    nudgeUp.click(); // a 2nd tap should ask for depth=2, not restart at depth=1
    await settle();

    const depth2Calls = fetchMock.mock.calls.filter(([input]) => String(input).includes("history=2"));
    expect(depth2Calls.length).toBeGreaterThan(0);
  });

  it("repeated nudge-up taps request increasing history depth, and restart from 1 after returning to live", async () => {
    // User field-test finding (2026-07-28): "load older" always revealed the
    // same one page back no matter how many times it was repeated, because
    // the gateway restores to live between requests and forgets depth.
    // Fixed by having the client track how many hops deep it already is and
    // ask for one more each time (?history=<n>).
    const pageByDepth: Record<string, string> = {
      "1": "page1\n❯ ",
      "2": "page1\npage2\n❯ ",
    };
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const match = String(input).match(/history=(\d+)/);
      if (match) {
        return Promise.resolve(
          new Response(JSON.stringify({ text: pageByDepth[match[1]] ?? "unexpected depth", revision: 2 }), {
            status: 200,
          }),
        );
      }
      return Promise.resolve(new Response(JSON.stringify({ text: "live\n❯ ", revision: 1 }), { status: 200 }));
    });
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    const viewport = mountTerminal();
    await settle();

    const nudgeUp = viewport.parentElement!.querySelector<HTMLButtonElement>("#nudge-up")!;
    nudgeUp.click();
    await settle();
    expect(fetchMock.mock.calls.some(([input]) => String(input).includes("history=1"))).toBe(true);

    nudgeUp.click(); // still viewing history -- should ask for depth 2, not repeat depth 1
    await settle();
    expect(fetchMock.mock.calls.some(([input]) => String(input).includes("history=2"))).toBe(true);

    const nudgeDown = viewport.parentElement!.querySelector<HTMLButtonElement>("#nudge-down")!;
    nudgeDown.click(); // return to live
    viewport.dispatchEvent(new Event("scroll"));
    await settle();

    nudgeUp.click(); // fresh escalation after returning to live -- must restart at depth 1
    await settle();
    const depth1CallsAfterReturn = fetchMock.mock.calls.filter(([input]) => String(input).includes("history=1")).length;
    expect(depth1CallsAfterReturn).toBeGreaterThan(1);
  });

  it("tapping the down-button (scroll-nudge) returns to live and resumes polling", async () => {
    vi.useFakeTimers();
    const shortText = Array.from({ length: 10 }, (_, i) => `line ${i + 1}`).join("\n");
    const longText = Array.from({ length: 500 }, (_, i) => `line ${i + 1}`).join("\n");
    mockScreenFetch({
      live: () => new Response(JSON.stringify({ text: shortText, revision: 1 }), { status: 200 }),
      history: () => new Response(JSON.stringify({ text: longText, revision: 2 }), { status: 200 }),
    });
    const viewport = mountTerminal();
    mockViewportMetrics(viewport, 200);
    await vi.advanceTimersByTimeAsync(0); // flush the initial (short) poll

    viewport.scrollTop = 0;
    viewport.dispatchEvent(new Event("scroll")); // load-older escalates to the long content, pausing poll
    await vi.advanceTimersByTimeAsync(0);
    expect(rowCount(viewport)).toBeGreaterThan(10);

    const nudgeDown = viewport.parentElement!.querySelector<HTMLButtonElement>("#nudge-down")!;
    nudgeDown.click(); // the same scrollTop = scrollHeight jump applySheetInset uses

    // jsdom doesn't fire a native 'scroll' event for a programmatic scrollTop
    // assignment -- dispatch it manually, same pattern as the D6 sheet test
    // above.
    viewport.dispatchEvent(new Event("scroll"));
    await vi.advanceTimersByTimeAsync(0); // let the resume's immediate poll() resolve

    expect(rowCount(viewport)).toBe(10); // reverted to live (shortText's 10 lines)
  });

  /**
   * jsdom implements neither Touch nor TouchEvent, and the pinch handlers only
   * ever read `touches[i].clientX/clientY` and call preventDefault -- so a
   * plain Event carrying a `touches` array exercises the same code path a real
   * device would.
   */
  function touchEvent(type: string, points: Array<{ clientX: number; clientY: number }>): Event {
    const ev = new Event(type, { bubbles: true, cancelable: true });
    Object.defineProperty(ev, "touches", { value: points });
    return ev;
  }

  const FONT_DEFAULT = 12; // module-private; mirrored here to express intent

  it("previews a pinch as a transform sized to the font it will commit to", async () => {
    mockScreenFetch({});
    const viewport = mountTerminal();
    await settle();
    const termEl = viewport.querySelector<HTMLElement>(".term-screen")!;

    viewport.dispatchEvent(
      touchEvent("touchstart", [
        { clientX: 100, clientY: 100 },
        { clientX: 200, clientY: 100 },
      ]),
    ); // starting distance 100px
    viewport.dispatchEvent(
      touchEvent("touchmove", [
        { clientX: 50, clientY: 100 },
        { clientX: 250, clientY: 100 },
      ]),
    ); // spread to 200px == scale 2

    // The preview is the ratio of the *committed* size, not the raw finger
    // ratio (2), so release is visually a no-op.
    const expected = pinchFontSize(FONT_DEFAULT, 2) / FONT_DEFAULT;
    expect(termEl.style.transform).toBe(`scale(${expected})`);
  });

  it("clears the preview transform on release so no scale is left stuck on the element", async () => {
    mockScreenFetch({});
    const viewport = mountTerminal();
    await settle();
    const termEl = viewport.querySelector<HTMLElement>(".term-screen")!;

    viewport.dispatchEvent(
      touchEvent("touchstart", [
        { clientX: 100, clientY: 100 },
        { clientX: 200, clientY: 100 },
      ]),
    );
    viewport.dispatchEvent(
      touchEvent("touchmove", [
        { clientX: 50, clientY: 100 },
        { clientX: 250, clientY: 100 },
      ]),
    );
    expect(termEl.style.transform).not.toBe("");

    // One finger lifted: the gesture is over even though a touch remains.
    viewport.dispatchEvent(touchEvent("touchend", [{ clientX: 50, clientY: 100 }]));

    expect(termEl.style.transform).toBe("");
    expect(termEl.style.transformOrigin).toBe("");
  });

  it("does not mistake a pinch near the top of the content for a scroll-to-top history request", async () => {
    // Two fingers landing on content that happens to be scrolled to the top
    // used to read as the deliberate "load older" drag, firing a history fetch
    // nobody asked for in the middle of a zoom.
    const fetchMock = mockScreenFetch({});
    const viewport = mountTerminal();
    await settle();

    viewport.dispatchEvent(
      touchEvent("touchstart", [
        { clientX: 100, clientY: 100 },
        { clientX: 200, clientY: 100 },
      ]),
    );
    viewport.scrollTop = 0;
    viewport.dispatchEvent(new Event("scroll"));
    await settle();

    expect(fetchMock.mock.calls.filter(([input]) => String(input).includes("history="))).toHaveLength(0);
  });

  it("still honours a genuine scroll-to-top once the pinch has ended", async () => {
    const fetchMock = mockScreenFetch({});
    const viewport = mountTerminal();
    await settle();

    viewport.dispatchEvent(
      touchEvent("touchstart", [
        { clientX: 100, clientY: 100 },
        { clientX: 200, clientY: 100 },
      ]),
    );
    viewport.dispatchEvent(touchEvent("touchend", []));

    viewport.scrollTop = 0;
    viewport.dispatchEvent(new Event("scroll"));
    await settle();

    expect(fetchMock.mock.calls.filter(([input]) => String(input).includes("history=1"))).toHaveLength(1);
  });

  it("ignores a one-finger touch, leaving ordinary pan untouched", async () => {
    mockScreenFetch({});
    const viewport = mountTerminal();
    await settle();
    const termEl = viewport.querySelector<HTMLElement>(".term-screen")!;

    viewport.dispatchEvent(touchEvent("touchstart", [{ clientX: 100, clientY: 100 }]));
    viewport.dispatchEvent(touchEvent("touchmove", [{ clientX: 100, clientY: 180 }]));

    expect(termEl.style.transform).toBe("");
  });

  it("hides the scroll-nudge buttons while the Reply or Keys sheet is open", async () => {
    mockScreenFetch({});
    const viewport = mountTerminal();
    await settle();

    const scrollNudge = viewport.parentElement!.querySelector<HTMLDivElement>("#scroll-nudge")!;
    expect(scrollNudge.hidden).toBe(false);

    const replyOpen = viewport.parentElement!.querySelector<HTMLButtonElement>("#reply-open")!;
    replyOpen.click();
    expect(scrollNudge.hidden).toBe(true);

    const replyClose = viewport.parentElement!.querySelector<HTMLButtonElement>("#reply-close")!;
    replyClose.click();
    expect(scrollNudge.hidden).toBe(false);

    const keysOpen = viewport.parentElement!.querySelector<HTMLButtonElement>("#keys-open")!;
    keysOpen.click();
    expect(scrollNudge.hidden).toBe(true);
  });

  it("never idle-fades the scroll-nudge buttons while viewing history, even past the idle timeout", async () => {
    // Regression: the idle-hide timer used to fire unconditionally 3s after
    // the last touch/scroll -- exactly what happens while an operator is
    // quietly reading revealed history (no further touch/scroll). The faded
    // buttons go pointer-events:none (styles.css .scroll-nudge.is-idle),
    // silently swallowing the very next tap -- "scrolled once, then stopped
    // working."
    vi.useFakeTimers();
    const shortText = Array.from({ length: 10 }, (_, i) => `line ${i + 1}`).join("\n");
    const longText = Array.from({ length: 500 }, (_, i) => `line ${i + 1}`).join("\n");
    mockScreenFetch({
      live: () => new Response(JSON.stringify({ text: shortText, revision: 1 }), { status: 200 }),
      history: () => new Response(JSON.stringify({ text: longText, revision: 2 }), { status: 200 }),
    });
    const viewport = mountTerminal();
    mockViewportMetrics(viewport, 200);
    await vi.advanceTimersByTimeAsync(0); // flush the initial (short) poll

    const scrollNudge = viewport.parentElement!.querySelector<HTMLDivElement>("#scroll-nudge")!;
    const nudgeUp = viewport.parentElement!.querySelector<HTMLButtonElement>("#nudge-up")!;
    nudgeUp.click();
    await vi.advanceTimersByTimeAsync(0); // loadOlder() resolves, sets viewingHistory
    expect(rowCount(viewport)).toBeGreaterThan(10);

    await vi.advanceTimersByTimeAsync(10_000); // well past the idle timeout, no further touch/scroll
    expect(scrollNudge.classList.contains("is-idle")).toBe(false);

    const nudgeDown = viewport.parentElement!.querySelector<HTMLButtonElement>("#nudge-down")!;
    nudgeDown.click(); // return to live
    viewport.dispatchEvent(new Event("scroll")); // jsdom needs this dispatched manually
    await vi.advanceTimersByTimeAsync(0);

    await vi.advanceTimersByTimeAsync(10_000); // idle-hide resumes once back live
    expect(scrollNudge.classList.contains("is-idle")).toBe(true);
  });

  it("renders the pane name, kind, and path in the floating header", async () => {
    mockScreenFetch({});
    const viewport = mountTerminal();
    await settle();

    const header = viewport.parentElement!.querySelector<HTMLDivElement>("#term-header")!;
    expect(header.querySelector(".term-header-name")!.textContent).toBe(agent.title);
    expect(header.querySelector(".term-header-kind")!.textContent).toBe(agent.kind);
    expect(header.querySelector<HTMLSpanElement>("#term-header-path")!.textContent).toBe(agent.path);
  });

  it("shows the floating header while scrolling down, hides it on scrolling back up", async () => {
    mockScreenFetch({});
    const viewport = mountTerminal();
    mockViewportMetrics(viewport, 200);
    await settle();

    const header = viewport.parentElement!.querySelector<HTMLDivElement>("#term-header")!;
    expect(header.classList.contains("is-visible")).toBe(false);

    viewport.scrollTop = 50; // scrolling down (away from 0)
    viewport.dispatchEvent(new Event("scroll"));
    expect(header.classList.contains("is-visible")).toBe(true);

    viewport.scrollTop = 10; // scrolling back up
    viewport.dispatchEvent(new Event("scroll"));
    expect(header.classList.contains("is-visible")).toBe(false);
  });

  it("auto-hides the floating header after a few seconds with no further scroll", async () => {
    vi.useFakeTimers();
    mockScreenFetch({});
    const viewport = mountTerminal();
    mockViewportMetrics(viewport, 200);
    await vi.advanceTimersByTimeAsync(0);

    const header = viewport.parentElement!.querySelector<HTMLDivElement>("#term-header")!;
    viewport.scrollTop = 50;
    viewport.dispatchEvent(new Event("scroll"));
    expect(header.classList.contains("is-visible")).toBe(true);

    await vi.advanceTimersByTimeAsync(10_000); // well past the idle timeout
    expect(header.classList.contains("is-visible")).toBe(false);
  });

  it("guards a reply send: types without submitting, then submits separately once the text is verifiably visible", async () => {
    vi.useFakeTimers();
    let screenText = "live\n❯ ";
    const fetchMock = mockScreenFetch({
      live: () => new Response(JSON.stringify({ text: screenText, revision: 1 }), { status: 200 }),
      input: (body) => {
        if (!body.submit) screenText = "live\n❯ hello"; // the TUI "renders" the typed text
        return new Response(null, { status: 200 });
      },
    });
    const viewport = mountTerminal();
    await vi.advanceTimersByTimeAsync(0); // flush the initial poll

    viewport.parentElement!.querySelector<HTMLButtonElement>("#reply-open")!.click();
    const replyText = viewport.parentElement!.querySelector<HTMLTextAreaElement>("#reply-text")!;
    const replySend = viewport.parentElement!.querySelector<HTMLButtonElement>("#reply-send")!;
    replyText.value = "hello";
    replySend.click();
    await vi.advanceTimersByTimeAsync(0); // flush the type-only send
    await vi.advanceTimersByTimeAsync(350); // one guard poll tick, catches the landed text

    const inputCalls = fetchMock.mock.calls.filter(([i]) => String(i).includes("/input"));
    expect(inputCalls.map(([, init]) => JSON.parse(String((init as RequestInit).body)))).toEqual([
      { text: "hello", submit: false },
      { text: "", submit: true },
    ]);
    expect(replyText.value).toBe(""); // cleared on confirmed send
  });

  it("never fires the submit key when the typed reply never becomes visible (guard stalls, no blind Enter)", async () => {
    vi.useFakeTimers();
    const fetchMock = mockScreenFetch({
      live: () => new Response(JSON.stringify({ text: "live\n❯ ", revision: 1 }), { status: 200 }),
      // The typed text never appears in a re-read screen -- simulates a
      // dialog swallowing it instead of the reply box.
    });
    const viewport = mountTerminal();
    await vi.advanceTimersByTimeAsync(0);

    viewport.parentElement!.querySelector<HTMLButtonElement>("#reply-open")!.click();
    const replyText = viewport.parentElement!.querySelector<HTMLTextAreaElement>("#reply-text")!;
    const replySend = viewport.parentElement!.querySelector<HTMLButtonElement>("#reply-send")!;
    const replySheet = viewport.parentElement!.querySelector<HTMLDivElement>("#reply-sheet")!;
    replyText.value = "hello";
    replySend.click();
    await vi.advanceTimersByTimeAsync(0);
    await vi.advanceTimersByTimeAsync(8 * 350); // exhaust every guard poll attempt

    const inputCalls = fetchMock.mock.calls.filter(([i]) => String(i).includes("/input"));
    expect(inputCalls).toHaveLength(1); // only the type-only call -- never a submit
    expect(replyText.value).toBe("hello"); // draft preserved, not lost
    expect(replySheet.hidden).toBe(false); // panel stays open
    expect(replyText.getAttribute("aria-invalid")).toBe("true");
    expect(replySend.disabled).toBe(false); // re-enabled so the operator can retry
  });

  describe("Close terminal", () => {
    function closeButton(viewport: HTMLDivElement): HTMLButtonElement {
      return viewport.parentElement!.querySelector<HTMLButtonElement>("#term-header-close")!;
    }

    it("does nothing without confirmation", async () => {
      const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(false);
      const fetchMock = mockScreenFetch({});
      const viewport = mountTerminal();
      await settle();

      closeButton(viewport).click();

      expect(confirmSpy).toHaveBeenCalledTimes(1);
      expect(fetchMock.mock.calls.some(([, init]) => (init as RequestInit | undefined)?.method === "DELETE")).toBe(
        false,
      );
    });

    it("closes the pane and leaves the screen once confirmed", async () => {
      vi.spyOn(window, "confirm").mockReturnValue(true);
      const fetchMock = mockScreenFetch({});
      const onBack = vi.fn();
      const viewport = mountTerminal(onBack);
      await settle();

      closeButton(viewport).click();
      await settle();

      const deleteCalls = fetchMock.mock.calls.filter(
        ([, init]) => (init as RequestInit | undefined)?.method === "DELETE",
      );
      expect(deleteCalls).toHaveLength(1);
      expect(String(deleteCalls[0][0])).toBe(`/api/panes/${encodeURIComponent(agent.pane_id)}`);
      expect(onBack).toHaveBeenCalledTimes(1);
    });

    it("stays put and re-enables the button when the close request fails", async () => {
      vi.spyOn(window, "confirm").mockReturnValue(true);
      const alertSpy = vi.spyOn(window, "alert").mockImplementation(() => {});
      mockScreenFetch({ close: () => new Response(null, { status: 502 }) });
      const onBack = vi.fn();
      const viewport = mountTerminal(onBack);
      await settle();

      const btn = closeButton(viewport);
      btn.click();
      await settle();

      expect(onBack).not.toHaveBeenCalled();
      expect(btn.disabled).toBe(false); // re-enabled so the operator can retry
      expect(alertSpy).toHaveBeenCalledTimes(1);
    });

    it("treats an already-gone pane (404) as closed", async () => {
      vi.spyOn(window, "confirm").mockReturnValue(true);
      mockScreenFetch({ close: () => new Response(null, { status: 404 }) });
      const onBack = vi.fn();
      const viewport = mountTerminal(onBack);
      await settle();

      closeButton(viewport).click();
      await settle();

      expect(onBack).toHaveBeenCalledTimes(1);
    });

    it("stays reachable regardless of .term-header's own show/hide state", () => {
      const viewport = mountTerminal();
      const header = viewport.parentElement!.querySelector<HTMLDivElement>("#term-header")!;
      const btn = closeButton(viewport);

      // The close button must not be a descendant of the element R20's
      // show/hide-on-scroll fade applies to -- a sibling can't inherit an
      // ancestor's opacity/transform, which is exactly the point.
      expect(header.contains(btn)).toBe(false);
      expect(header.classList.contains("is-visible")).toBe(false);
    });
  });
});

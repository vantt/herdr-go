import { describe, it, expect, vi, afterEach } from "vitest";
import {
  computeKeyboardInset,
  stripAnsiLen,
  terminalHead,
  preserveScrollTop,
  renderTerminal,
} from "../src/views/terminal";
import type { AgentRow } from "../src/api";
import type { NewPaneRef } from "../src/main";

// xterm.js needs window.matchMedia (its CoreBrowserService reads it to track
// devicePixelRatio) -- jsdom has no real implementation, so renderTerminal's
// underlying `new Terminal()` throws without this stub.
if (!window.matchMedia) {
  window.matchMedia = ((query: string) =>
    ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }) as unknown as MediaQueryList) as typeof window.matchMedia;
}

// jsdom has no real canvas implementation. xterm's DOM renderer touches it
// only for a one-off default-color computation during import, and already
// tolerates the failure internally (it logs, doesn't throw) -- the resulting
// stderr noise is harmless and unrelated to anything under test here.

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
  }): ReturnType<typeof vi.fn> {
    const fn = vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
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

  function mountTerminal(): HTMLDivElement {
    const root = document.createElement("div");
    document.body.appendChild(root);
    renderTerminal(root, { agent, onBack: () => {} });
    return root.querySelector<HTMLDivElement>("#term-viewport")!;
  }

  /**
   * xterm's DOM renderer sizes rows/canvas from real layout metrics, which
   * jsdom never computes (scrollHeight/clientHeight always read 0). Override
   * them on the actual #term-viewport node: clientHeight is a fixed mock
   * "visible" height, and scrollHeight is derived from the real, already-
   * verified `.xterm-rows` child count -- so it stays in sync with whatever
   * applyScreen actually rendered, the same way a real browser's layout would.
   */
  function mockViewportMetrics(viewport: HTMLDivElement, clientHeight: number): void {
    Object.defineProperty(viewport, "clientHeight", { configurable: true, get: () => clientHeight });
    Object.defineProperty(viewport, "scrollHeight", {
      configurable: true,
      get: () => (viewport.querySelector(".xterm-rows")?.children.length ?? 0) * ROW_PX,
    });
  }

  function rowCount(viewport: HTMLDivElement): number {
    return viewport.querySelector(".xterm-rows")?.children.length ?? 0;
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

    // clamp(lines.length + 1, 4, 1000) === 601 -- unreachable under the old
    // 400 clamp, which would have silently discarded the oldest ~200 lines
    // instead (xterm drops rows past the grid's row count when scrollback: 0).
    expect(rowCount(viewport)).toBe(601);
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
    expect(rowCount(viewport)).toBe(501); // longText's 500 lines + 1
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

    expect(rowCount(viewport)).toBe(11); // reverted to live (shortText)
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

    expect(rowCount(viewport)).toBe(11); // reverted to live (shortText)
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

  // ── transcript-live-tail: the Log view ──────────────────────────────────

  /**
   * Mocks both GET /screen (live default) and GET /activity. The activity
   * handler receives the decoded cursor (null on the first, cursorless call)
   * and returns the response body — letting each test script the tail.
   */
  function mockTailFetch(
    activity: (cursor: string | null) => {
      available: boolean;
      lines: string[];
      cursor: string | null;
    },
  ): ReturnType<typeof vi.fn> {
    const fn = vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/activity")) {
        const match = url.match(/[?&]cursor=([^&]*)/);
        const cursor = match ? decodeURIComponent(match[1]) : null;
        return Promise.resolve(
          new Response(JSON.stringify(activity(cursor)), { status: 200 }),
        );
      }
      if (url.includes("/screen")) {
        return Promise.resolve(
          new Response(JSON.stringify({ text: "live\n❯ ", revision: 1 }), { status: 200 }),
        );
      }
      return Promise.resolve(new Response("not found", { status: 404 }));
    });
    globalThis.fetch = fn as unknown as typeof fetch;
    return fn;
  }

  function logParts(root: HTMLElement) {
    return {
      toggle: root.querySelector<HTMLButtonElement>("#log-toggle")!,
      activityViewport: root.querySelector<HTMLDivElement>("#activity-viewport")!,
      termViewport: root.querySelector<HTMLDivElement>("#term-viewport")!,
      pre: root.querySelector<HTMLPreElement>("#activity-pre")!,
      empty: root.querySelector<HTMLDivElement>("#activity-empty")!,
    };
  }

  it("log toggle swaps viewports and opens the tail with a cursorless request", async () => {
    const fetchMock = mockTailFetch(() => ({ available: true, lines: [], cursor: "s1.jsonl:100" }));
    const viewport = mountTerminal();
    const parts = logParts(viewport.parentElement!);
    await settle();

    expect(parts.activityViewport.hidden).toBe(true);
    parts.toggle.click();
    await settle();

    expect(parts.activityViewport.hidden).toBe(false);
    expect(parts.termViewport.hidden).toBe(true);
    const activityCalls = fetchMock.mock.calls.filter(([i]) => String(i).includes("/activity"));
    expect(activityCalls).toHaveLength(1);
    // Open = now: the very first request carries no cursor (backend answers
    // with EOF, no backfill).
    expect(String(activityCalls[0][0])).not.toContain("cursor=");
  });

  it("appends tail lines and round-trips the cursor across polls, surviving a toggle away and back", async () => {
    const chunks: Record<string, { available: boolean; lines: string[]; cursor: string }> = {
      "": { available: true, lines: [], cursor: "s1.jsonl:0" },
      "s1.jsonl:0": { available: true, lines: ["> bash: cargo test", "  ok"], cursor: "s1.jsonl:80" },
      "s1.jsonl:80": { available: true, lines: ["done"], cursor: "s1.jsonl:99" },
    };
    mockTailFetch((cursor) => chunks[cursor ?? ""]);
    const viewport = mountTerminal();
    const parts = logParts(viewport.parentElement!);
    await settle();

    parts.toggle.click(); // open: cursorless → cursor s1.jsonl:0, no lines
    await settle();
    parts.toggle.click(); // back to screen
    await settle();
    parts.toggle.click(); // back to log: polls with the kept cursor
    await settle();

    expect(parts.pre.textContent).toBe("> bash: cargo test\n  ok");
    parts.toggle.click();
    await settle();
    parts.toggle.click();
    await settle();
    // Lines append — nothing lost, nothing repeated.
    expect(parts.pre.textContent).toBe("> bash: cargo test\n  ok\ndone");
  });

  it("keeps only the newest 200 lines in the ring", async () => {
    const burst = Array.from({ length: 250 }, (_, i) => `line ${i + 1}`);
    const responses: Record<string, { available: boolean; lines: string[]; cursor: string }> = {
      "": { available: true, lines: [], cursor: "s1.jsonl:0" },
      "s1.jsonl:0": { available: true, lines: burst, cursor: "s1.jsonl:9000" },
    };
    mockTailFetch((cursor) => responses[cursor ?? ""]);
    const viewport = mountTerminal();
    const parts = logParts(viewport.parentElement!);
    await settle();

    parts.toggle.click();
    await settle();
    parts.toggle.click();
    await settle();
    parts.toggle.click(); // second log poll delivers the 250-line burst
    await settle();

    const lines = parts.pre.textContent!.split("\n");
    expect(lines).toHaveLength(200);
    expect(lines[0]).toBe("line 51");
    expect(lines[199]).toBe("line 250");
  });

  it("shows the no-transcript note when the pane has nothing to tail", async () => {
    mockTailFetch(() => ({ available: false, lines: [], cursor: null }));
    const viewport = mountTerminal();
    const parts = logParts(viewport.parentElement!);
    await settle();

    expect(parts.empty.hidden).toBe(true);
    parts.toggle.click();
    await settle();
    expect(parts.empty.hidden).toBe(false);
  });

  it("returning to the screen view resumes the screen poll untouched", async () => {
    const fetchMock = mockTailFetch(() => ({ available: true, lines: [], cursor: "s1.jsonl:0" }));
    const viewport = mountTerminal();
    const parts = logParts(viewport.parentElement!);
    await settle();

    parts.toggle.click();
    await settle();
    const screenCallsWhileLog = fetchMock.mock.calls.filter(([i]) => String(i).includes("/screen")).length;
    parts.toggle.click(); // back to screen — polls immediately
    await settle();

    expect(parts.termViewport.hidden).toBe(false);
    expect(parts.activityViewport.hidden).toBe(true);
    const screenCallsAfter = fetchMock.mock.calls.filter(([i]) => String(i).includes("/screen")).length;
    expect(screenCallsAfter).toBe(screenCallsWhileLog + 1);
  });
});

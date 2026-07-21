import { describe, it, expect, vi, afterEach } from "vitest";
import {
  login,
  fetchAgents,
  fetchHealth,
  fetchScreen,
  sendReply,
  fetchCreateOptions,
  createPane,
  createAgent,
} from "../src/api";

describe("api", () => {
  const originalFetch = globalThis.fetch;

  afterEach(() => {
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  describe("login", () => {
    it("maps a 200 response to success", async () => {
      globalThis.fetch = vi.fn().mockResolvedValue(new Response(null, { status: 200 }));
      await expect(login("right-token")).resolves.toBe(true);
    });

    it("maps the opaque 404 (wrong token) to a generic failure, no detail leaked", async () => {
      globalThis.fetch = vi.fn().mockResolvedValue(new Response(null, { status: 404 }));
      await expect(login("wrong-token")).resolves.toBe(false);
    });

    it("maps any other non-200 status to failure too", async () => {
      globalThis.fetch = vi.fn().mockResolvedValue(new Response(null, { status: 500 }));
      await expect(login("token")).resolves.toBe(false);
    });
  });

  describe("fetchAgents", () => {
    it("parses the { agents, shells } response shape on success", async () => {
      const payload = {
        agents: [
          {
            pane_id: "w3:p6",
            workspace: "w3",
            display: "claude · Kiểm tra plan",
            kind: "claude",
            status: "working",
            title: "Kiểm tra plan",
            workspace_label: "backend-api",
            tab_label: "plan",
            workspace_status: "working",
          },
        ],
        shells: [
          {
            pane_id: "wB:p1",
            workspace_id: "wB",
            workspace_label: "scratch",
            tab_label: "shell",
            path: "/home/dev/scratch",
          },
          {
            pane_id: "wB:p2",
            workspace_id: "wB",
            workspace_label: "scratch",
            tab_label: "shell",
            path: null,
          },
        ],
      };
      globalThis.fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify(payload), { status: 200 }));
      await expect(fetchAgents()).resolves.toEqual(payload);
    });

    it("returns null on 404 (session expired) instead of throwing", async () => {
      globalThis.fetch = vi.fn().mockResolvedValue(new Response(null, { status: 404 }));
      await expect(fetchAgents()).resolves.toBeNull();
    });

    it("throws on an unexpected non-OK status", async () => {
      globalThis.fetch = vi.fn().mockResolvedValue(new Response(null, { status: 502 }));
      await expect(fetchAgents()).rejects.toThrow();
    });
  });

  describe("fetchHealth", () => {
    it("returns the health payload on success", async () => {
      const health = { version: "0.1.0", protocol: 16, herdr_up: true };
      globalThis.fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify(health), { status: 200 }));
      await expect(fetchHealth()).resolves.toEqual(health);
    });

    it("returns null (never throws) when the request fails outright", async () => {
      globalThis.fetch = vi.fn().mockRejectedValue(new Error("network down"));
      await expect(fetchHealth()).resolves.toBeNull();
    });
  });

  describe("fetchScreen", () => {
    it("returns the screen text + revision on success", async () => {
      const screen = { text: "\x1b[32mhello\x1b[0m", revision: 5 };
      globalThis.fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify(screen), { status: 200 }));
      await expect(fetchScreen("w3:p6")).resolves.toEqual(screen);
    });

    it("returns null on 404 (pane gone)", async () => {
      globalThis.fetch = vi.fn().mockResolvedValue(new Response(null, { status: 404 }));
      await expect(fetchScreen("gone")).resolves.toBeNull();
    });
  });

  describe("fetchCreateOptions", () => {
    it("parses the destination + preset list on success", async () => {
      const payload = {
        destinations: [
          { workspace_id: "w1", label: "frontend-app", path: "/home/dev/frontend-app", path_is_live: true },
          { workspace_id: "w3", label: "backend-api", path: null, path_is_live: false },
        ],
        presets: [{ label: "Claude" }],
      };
      globalThis.fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify(payload), { status: 200 }));
      await expect(fetchCreateOptions()).resolves.toEqual(payload);
    });

    it("returns null on 404 (session expired) instead of throwing", async () => {
      globalThis.fetch = vi.fn().mockResolvedValue(new Response(null, { status: 404 }));
      await expect(fetchCreateOptions()).resolves.toBeNull();
    });

    it("throws on an unexpected non-OK status", async () => {
      globalThis.fetch = vi.fn().mockResolvedValue(new Response(null, { status: 502 }));
      await expect(fetchCreateOptions()).rejects.toThrow();
    });
  });

  describe("createPane", () => {
    it("posts the workspace id and resolves ok with tab_id + pane_id", async () => {
      const spy = vi
        .fn()
        .mockResolvedValue(
          new Response(JSON.stringify({ tab_id: "w1:t-new", pane_id: "w1:p-new" }), { status: 200 }),
        );
      globalThis.fetch = spy;
      await expect(createPane("w1")).resolves.toEqual({ ok: true, tab_id: "w1:t-new", pane_id: "w1:p-new" });
      const [, init] = spy.mock.calls[0];
      expect(JSON.parse(init.body)).toEqual({ workspace_id: "w1" });
    });

    it("resolves ok:false with the backend's error message on 409, never throwing", async () => {
      globalThis.fetch = vi
        .fn()
        .mockResolvedValue(
          new Response(JSON.stringify({ error: "destination w1 no longer exists" }), { status: 409 }),
        );
      await expect(createPane("w1")).resolves.toEqual({
        ok: false,
        error: "destination w1 no longer exists",
      });
    });

    it("resolves ok:false with the backend's error message on 502", async () => {
      globalThis.fetch = vi
        .fn()
        .mockResolvedValue(new Response(JSON.stringify({ error: "terminal host unreachable" }), { status: 502 }));
      await expect(createPane("w1")).resolves.toEqual({ ok: false, error: "terminal host unreachable" });
    });
  });

  describe("createAgent", () => {
    it("posts the workspace id + preset and resolves ok with tab_id, pane_id, name", async () => {
      const spy = vi.fn().mockResolvedValue(
        new Response(JSON.stringify({ tab_id: "w1:t-new", pane_id: "w1:p-new", name: "claude-1" }), {
          status: 200,
        }),
      );
      globalThis.fetch = spy;
      await expect(createAgent("w1", "Claude")).resolves.toEqual({
        ok: true,
        tab_id: "w1:t-new",
        pane_id: "w1:p-new",
        name: "claude-1",
      });
      const [, init] = spy.mock.calls[0];
      expect(JSON.parse(init.body)).toEqual({ workspace_id: "w1", preset: "Claude" });
    });

    it("resolves ok:false with the backend's error message on 400 (unknown preset)", async () => {
      globalThis.fetch = vi
        .fn()
        .mockResolvedValue(
          new Response(JSON.stringify({ error: "unknown agent preset: Bogus" }), { status: 400 }),
        );
      await expect(createAgent("w1", "Bogus")).resolves.toEqual({
        ok: false,
        error: "unknown agent preset: Bogus",
      });
    });

    it("resolves ok:false with the backend's error message on 409 (unresolved anchor)", async () => {
      globalThis.fetch = vi.fn().mockResolvedValue(
        new Response(
          JSON.stringify({ error: "destination w1 has no resolved path; refusing to start an agent" }),
          { status: 409 },
        ),
      );
      await expect(createAgent("w1", "Claude")).resolves.toEqual({
        ok: false,
        error: "destination w1 has no resolved path; refusing to start an agent",
      });
    });
  });

  describe("sendReply", () => {
    it("posts the reply and resolves true on success", async () => {
      const spy = vi.fn().mockResolvedValue(new Response(null, { status: 200 }));
      globalThis.fetch = spy;
      await expect(sendReply("w3:p6", "do it", true)).resolves.toBe(true);
      const [, init] = spy.mock.calls[0];
      expect(JSON.parse(init.body)).toEqual({ text: "do it", submit: true });
    });

    it("resolves false on failure", async () => {
      globalThis.fetch = vi.fn().mockResolvedValue(new Response(null, { status: 502 }));
      await expect(sendReply("w3:p6", "x")).resolves.toBe(false);
    });
  });
});

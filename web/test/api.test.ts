import { describe, it, expect, vi, afterEach } from "vitest";
import { login, fetchAgents, fetchHealth } from "../src/api";

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
    it("parses the agent row shape on success", async () => {
      const rows = [
        {
          workspace: "herdr-gateway",
          tab: "main",
          pane_id: "pane-1",
          display: "herdr-gateway › main › claude",
          kind: "claude",
          status: "working",
        },
      ];
      globalThis.fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify(rows), { status: 200 }));
      await expect(fetchAgents()).resolves.toEqual(rows);
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
});

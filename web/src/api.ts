// Typed fetch wrappers for the herdr-gateway HTTP API. Every protected route
// answers a session failure with an opaque 404 (see src/web/auth.rs) — callers
// treat that uniformly as "not authenticated", never surfacing more detail.

export type AgentStatus = "working" | "blocked" | "done" | "idle";

export interface AgentRow {
  workspace: string;
  tab: string;
  pane_id: string;
  display: string;
  kind: string;
  status: AgentStatus;
}

export interface HealthInfo {
  version: string;
  protocol: number;
  herdr_up: boolean;
}

function request(path: string, init?: RequestInit): Promise<Response> {
  return fetch(path, { credentials: "same-origin", ...init });
}

/**
 * POST /api/login. Resolves true only on a 200 response; any other status
 * (404 for a wrong/missing token, or anything else) resolves false with no
 * further detail — the backend is deliberately opaque about why it failed.
 */
export async function login(token: string): Promise<boolean> {
  const res = await request("/api/login", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ token }),
  });
  return res.ok;
}

/** POST /api/logout. Always resolves (server treats it as idempotent). */
export async function logout(): Promise<void> {
  await request("/api/logout", { method: "POST" });
}

/**
 * GET /api/agents. Resolves the row list on success, or `null` on a 404,
 * meaning the session has expired/is missing — the caller should return to
 * login. Throws on any other non-OK status or malformed payload.
 */
export async function fetchAgents(): Promise<AgentRow[] | null> {
  const res = await request("/api/agents");
  if (res.status === 404) return null;
  if (!res.ok) throw new Error(`agents request failed: ${res.status}`);
  const data: unknown = await res.json();
  if (!Array.isArray(data)) throw new Error("agents payload was not an array");
  return data as AgentRow[];
}

/**
 * GET /api/health. Unauthenticated. Resolves `null` on any failure (network
 * error or non-OK status) since this only drives a small status dot — never
 * worth surfacing as an error to the user.
 */
export async function fetchHealth(): Promise<HealthInfo | null> {
  try {
    const res = await request("/api/health");
    if (!res.ok) return null;
    return (await res.json()) as HealthInfo;
  } catch {
    return null;
  }
}

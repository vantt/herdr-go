// Typed fetch wrappers for the herdr-go HTTP API. Every protected route
// answers a session failure with an opaque 404 (see src/web/auth.rs) — callers
// treat that uniformly as "not authenticated", never surfacing more detail.

export type AgentStatus = "working" | "blocked" | "done" | "idle" | "unknown";

export interface AgentRow {
  pane_id: string;
  workspace: string;
  display: string;
  kind: string;
  status: AgentStatus;
  title: string;
  workspace_label: string;
  tab_label: string;
  workspace_status: AgentStatus;
}

export interface HealthInfo {
  version: string;
  protocol: number;
  herdr_up: boolean;
}

export interface Destination {
  workspace_id: string;
  label: string;
  path: string | null;
  path_is_live: boolean;
}

export interface PresetOption {
  label: string;
}

export interface CreateOptions {
  destinations: Destination[];
  presets: PresetOption[];
}

export type CreatePaneResult =
  | { ok: true; tab_id: string; pane_id: string }
  | { ok: false; error: string };

export type CreateAgentResult =
  | { ok: true; tab_id: string; pane_id: string; name: string }
  | { ok: false; error: string };

export interface ScreenRead {
  text: string;
  revision: number;
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
 * GET /api/create-options. Resolves the destination + preset list on success,
 * or `null` on a 404, meaning the session has expired/is missing — mirrors
 * fetchAgents. Throws on any other non-OK status or malformed payload.
 */
export async function fetchCreateOptions(): Promise<CreateOptions | null> {
  const res = await request("/api/create-options");
  if (res.status === 404) return null;
  if (!res.ok) throw new Error(`create-options request failed: ${res.status}`);
  return (await res.json()) as CreateOptions;
}

/** Parses the backend's `{ error: string }` body off a non-OK create response. */
async function createErrorResult(res: Response): Promise<{ ok: false; error: string }> {
  const body = (await res.json().catch(() => null)) as { error?: string } | null;
  return { ok: false, error: body?.error ?? `request failed: ${res.status}` };
}

/**
 * POST /api/panes. Opens a plain shell in `workspaceId`. A 409 (stale
 * destination) or 502 (other terminal host failure) never throws — it
 * resolves `{ ok: false, error }` carrying the backend's message so the
 * caller can render it inline without a try/catch (CONTEXT.md S3). Reachable
 * only already-authenticated, so no 404-means-logged-out handling here.
 */
export async function createPane(workspaceId: string): Promise<CreatePaneResult> {
  const res = await request("/api/panes", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ workspace_id: workspaceId }),
  });
  if (!res.ok) return createErrorResult(res);
  const data = (await res.json()) as { tab_id: string; pane_id: string };
  return { ok: true, tab_id: data.tab_id, pane_id: data.pane_id };
}

/**
 * POST /api/agents. Starts `preset` in `workspaceId`. A 400 (unknown preset),
 * 409 (stale destination or unresolved anchor), or 502 never throws — it
 * resolves `{ ok: false, error }` carrying the backend's message (CONTEXT.md
 * S3). Reachable only already-authenticated, so no 404-means-logged-out
 * handling here.
 */
export async function createAgent(
  workspaceId: string,
  preset: string,
): Promise<CreateAgentResult> {
  const res = await request("/api/agents", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ workspace_id: workspaceId, preset }),
  });
  if (!res.ok) return createErrorResult(res);
  const data = (await res.json()) as { tab_id: string; pane_id: string; name: string };
  return { ok: true, tab_id: data.tab_id, pane_id: data.pane_id, name: data.name };
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

/**
 * GET /api/panes/:pane/screen. The pane's current rendered screen (ANSI) for a
 * zoom/pan view. Resolves `null` on 404 (pane gone / session expired). Throws
 * on other transport errors so the caller can show a retry.
 */
export async function fetchScreen(paneId: string): Promise<ScreenRead | null> {
  const res = await request(`/api/panes/${encodeURIComponent(paneId)}/screen`);
  if (res.status === 404) return null;
  if (!res.ok) throw new Error(`screen request failed: ${res.status}`);
  return (await res.json()) as ScreenRead;
}

/**
 * POST /api/panes/:pane/input. Send a reply into the pane. `submit` (default
 * true) sends Enter after the text. Resolves true on success.
 */
export async function sendReply(
  paneId: string,
  text: string,
  submit = true,
): Promise<boolean> {
  const res = await request(`/api/panes/${encodeURIComponent(paneId)}/input`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ text, submit }),
  });
  return res.ok;
}

/**
 * POST /api/panes/:pane/keys. Send raw key presses (herdr names, e.g. "down",
 * "enter", "escape") so the human can drive a TUI option menu. Resolves true on
 * success.
 */
export async function sendKeys(paneId: string, keys: string[]): Promise<boolean> {
  const res = await request(`/api/panes/${encodeURIComponent(paneId)}/keys`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ keys }),
  });
  return res.ok;
}

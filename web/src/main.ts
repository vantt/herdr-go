import "./styles.css";
import { fetchAgents } from "./api";
import type { AgentRow, AgentsResponse, ShellRow } from "./api";
import { renderLogin } from "./views/login";
import { renderSwitcher } from "./views/switcher";
import { renderTerminal } from "./views/terminal";

// The minimal reference needed to open a just-created pane's terminal detail
// (S5). A plain shell can never produce a full AgentRow, so post-create
// navigation carries only what is in hand at creation: the response's pane_id
// (plus, for an agent, the generated name) and the destination's
// workspace_id/label. This is the single source of truth for that shape — no
// other module re-derives its field set.
export interface NewPaneRef {
  pane_id: string;
  workspace_id: string;
  label: string;
  name?: string;
}

export type Route =
  | { name: "login" }
  | { name: "switcher" }
  | { name: "terminal"; agent: AgentRow | NewPaneRef };

const TERMINAL_PATH_RE = /^\/terminal\/([^/]+)$/;

/**
 * Builds the URL for a route (D1): `/terminal/<pane_id>` for terminal detail, `/` for everything
 * else (D4). ':' is left unescaped -- it's the stable workspace:pane separator and is legal in a
 * URI path segment (RFC 3986 pchar), so undoing encodeURIComponent's %3A keeps the URL readable
 * without weakening the escaping of '/' or anything else parseTerminalPaneId depends on.
 */
export function pathForRoute(route: Route): string {
  return route.name === "terminal"
    ? `/terminal/${encodeURIComponent(route.agent.pane_id).replace(/%3A/gi, ":")}`
    : "/";
}

/** Extracts a pane_id out of a `/terminal/<pane_id>` pathname, or null for any other shape (including an undecodable percent-escape, which falls back the same as a non-matching path -- D3). */
export function parseTerminalPaneId(pathname: string): string | null {
  const match = TERMINAL_PATH_RE.exec(pathname);
  if (!match) return null;
  try {
    return decodeURIComponent(match[1]);
  } catch {
    return null;
  }
}

/**
 * Resolves a URL's pane_id against an /api/agents snapshot (D3). An AgentRow
 * hit is returned as-is; a ShellRow hit is reshaped into the same NewPaneRef
 * shape switcher.ts builds for onSelect. No match resolves to null, meaning
 * the caller falls back to switcher, silently.
 */
export function resolvePaneRef(
  paneId: string,
  agents: AgentRow[],
  shells: ShellRow[],
): AgentRow | NewPaneRef | null {
  const agent = agents.find((row) => row.pane_id === paneId);
  if (agent) return agent;
  const shell = shells.find((row) => row.pane_id === paneId);
  if (shell) {
    return {
      pane_id: shell.pane_id,
      workspace_id: shell.workspace_id,
      label: shell.path ?? shell.workspace_label,
    };
  }
  return null;
}

/**
 * The route to land on after resolving a pane_id against a (possibly absent)
 * agents snapshot: shared by bootstrap's initial-load fallback (D3) and a
 * post-login redirect (D5) so both silently fall back to switcher the same
 * way when there is no intended pane or it no longer resolves.
 */
export function resolveLoginRedirect(
  intendedPaneId: string | null,
  snapshot: AgentsResponse | null,
): Route {
  if (intendedPaneId && snapshot) {
    const ref = resolvePaneRef(intendedPaneId, snapshot.agents, snapshot.shells);
    if (ref) return { name: "terminal", agent: ref };
  }
  return { name: "switcher" };
}

const root = document.getElementById("app");

function showLogin(intendedPaneId: string | null): void {
  if (!root) return;
  root.innerHTML = "";
  renderLogin(root, { onSuccess: () => void handleLoginSuccess(intendedPaneId) });
}

function applyRoute(route: Route): void {
  if (!root) return;
  root.innerHTML = "";
  switch (route.name) {
    case "login":
      showLogin(null);
      break;
    case "switcher":
      renderSwitcher(root, {
        onSelect: (agent) => navigate({ name: "terminal", agent }),
        onLoggedOut: () => navigate({ name: "login" }),
        onCreated: (ref) => navigate({ name: "terminal", agent: ref }),
      });
      break;
    case "terminal":
      renderTerminal(root, {
        agent: route.agent,
        onBack: goBack,
      });
      break;
  }
}

// In-app Back pops the existing back-stack (D2) rather than pushing a fresh
// forward entry, so browser/phone Back stays a single consistent stack with
// in-app Back instead of diverging.
export function goBack(): void {
  history.back();
}

// Moves forward to a new route (D2): pushes a fresh history entry when the
// URL actually changes, or replaces the current entry's state in place when
// it doesn't (switcher and login share '/' per D4) so popstate never restores
// a stale route for the path it's landing on.
export function navigate(route: Route): void {
  const path = pathForRoute(route);
  if (location.pathname === path) {
    history.replaceState({ route }, "", path);
  } else {
    history.pushState({ route }, "", path);
  }
  applyRoute(route);
}

function handlePopState(event: PopStateEvent): void {
  const state = event.state as { route?: Route } | null;
  applyRoute(state?.route ?? { name: "switcher" });
}

window.addEventListener("popstate", handlePopState);

// A login prompted by opening a stale-session /terminal/<pane_id> link
// redirects straight back into that same terminal on success, if it still
// resolves (D5) -- intendedPaneId is carried across the round trip as a plain
// closure variable, never a change to LoginProps' onSuccess: () => void.
async function handleLoginSuccess(intendedPaneId: string | null): Promise<void> {
  let snapshot: AgentsResponse | null = null;
  try {
    snapshot = await fetchAgents();
  } catch {
    snapshot = null;
  }
  navigate(resolveLoginRedirect(intendedPaneId, snapshot));
}

// A page load/refresh may already carry a valid session cookie, and the URL
// may already point at a specific terminal (D1) -- probing /api/agents once
// resolves both: which screen to land on, and (for a /terminal/<pane_id> URL)
// which pane to open directly, without ever visiting switcher first.
async function bootstrap(): Promise<void> {
  const intendedPaneId = parseTerminalPaneId(location.pathname);
  let snapshot: AgentsResponse | null = null;
  try {
    snapshot = await fetchAgents();
  } catch {
    snapshot = null;
  }

  if (snapshot === null) {
    showLogin(intendedPaneId);
    return;
  }

  const route = resolveLoginRedirect(intendedPaneId, snapshot);
  history.replaceState({ route }, "", pathForRoute(route));
  applyRoute(route);
}

void bootstrap();

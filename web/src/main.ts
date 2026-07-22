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
 * Builds the URL for a route: `/login`, `/switcher`, or `/terminal/<pane_id>` (D1) -- every route
 * now has its own concrete path, `/` is no longer any screen's own URL (D2). ':' is left unescaped
 * in the pane_id segment -- it's the stable workspace:pane separator and is legal in a URI path
 * segment (RFC 3986 pchar), so undoing encodeURIComponent's %3A keeps the URL readable without
 * weakening the escaping of '/' or anything else parseTerminalPaneId depends on.
 */
export function pathForRoute(route: Route): string {
  switch (route.name) {
    case "login":
      return "/login";
    case "switcher":
      return "/switcher";
    case "terminal":
      return `/terminal/${encodeURIComponent(route.agent.pane_id).replace(/%3A/gi, ":")}`;
  }
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

/** A recognized incoming path (D1), or null for '/' and anything else unrecognized (D2). */
export type ParsedPath =
  | { name: "login" }
  | { name: "switcher" }
  | { name: "terminal"; paneId: string };

/**
 * Parses a pathname into a recognized route (D1): `/login`, `/switcher`, or a `/terminal/<pane_id>`
 * link (delegated to parseTerminalPaneId, so a malformed percent-escape still falls back to null --
 * D3). `/` and any other unrecognized path return null; bootstrap() canonicalizes those away (D2).
 */
export function parseRoutePath(pathname: string): ParsedPath | null {
  if (pathname === "/login") return { name: "login" };
  if (pathname === "/switcher") return { name: "switcher" };
  const paneId = parseTerminalPaneId(pathname);
  if (paneId !== null) return { name: "terminal", paneId };
  return null;
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

/** What bootstrap() should render/store for a given incoming path + session probe result. */
export interface BootstrapDecision {
  /** The route to render. */
  route: Route;
  /** Non-null only for the PBI-025 D5 carve-out: a /terminal/<pane_id> link whose session probe
   *  failed, so login is shown but the intended pane must survive a later login round-trip. */
  intendedPaneId: string | null;
  /** Whether bootstrap() should rewrite the visible URL to pathForRoute(route) via replaceState.
   *  False only for the carve-out above, which must leave the URL exactly as typed (PBI-025 D5). */
  canonicalize: boolean;
}

/**
 * Pure decision function for what a page load renders, given the parsed incoming path (D1/D2) and
 * the (possibly absent) session snapshot. Kept free of DOM/history/network side effects so it's
 * directly unit-testable; bootstrap() is the sole caller and owns the actual rendering/history
 * writes.
 *
 * - A /terminal/<pane_id> link keeps PBI-025's exact behavior: with a session, it resolves straight
 *   into that pane (falling back to switcher if it no longer resolves); without one, login is shown
 *   and the URL is left untouched so the intended pane survives a later login (D5's carve-out).
 * - Every other case -- /login, /switcher, '/', or any unrecognized path -- canonicalizes the URL:
 *   with a session, straight to switcher (D2-D4, including an already-authenticated /login visit
 *   never showing the login form -- D3); without one, to /login (D5).
 */
export function resolveBootstrapDecision(
  parsed: ParsedPath | null,
  snapshot: AgentsResponse | null,
): BootstrapDecision {
  if (parsed?.name === "terminal") {
    if (snapshot === null) {
      return { route: { name: "login" }, intendedPaneId: parsed.paneId, canonicalize: false };
    }
    return { route: resolveLoginRedirect(parsed.paneId, snapshot), intendedPaneId: null, canonicalize: true };
  }

  if (snapshot === null) {
    return { route: { name: "login" }, intendedPaneId: null, canonicalize: true };
  }

  return { route: { name: "switcher" }, intendedPaneId: null, canonicalize: true };
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
// it doesn't, so popstate never restores a stale route for the path it's
// landing on. D7 overrides this for login: entering the 'login' route, or
// leaving it (e.g. a successful login), always replaces instead -- checked
// against the actual stored route, not a path-string comparison -- so
// '/login' can never become a back-stack entry an already-authenticated
// operator could land back on via Back.
export function navigate(route: Route): void {
  const path = pathForRoute(route);
  const currentRoute = (history.state as { route?: Route } | null)?.route;
  const entersOrLeavesLogin = route.name === "login" || currentRoute?.name === "login";
  if (entersOrLeavesLogin || location.pathname === path) {
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
// may already point at a specific route -- /login, /switcher, or
// /terminal/<pane_id> (D1) -- probing /api/agents once resolves both: which
// screen to land on, and (for a /terminal/<pane_id> URL) which pane to open
// directly, without ever visiting switcher first. resolveBootstrapDecision
// carries the actual D2-D5 logic; this just runs its side effects.
async function bootstrap(): Promise<void> {
  const parsed = parseRoutePath(location.pathname);
  let snapshot: AgentsResponse | null = null;
  try {
    snapshot = await fetchAgents();
  } catch {
    snapshot = null;
  }

  const decision = resolveBootstrapDecision(parsed, snapshot);

  if (decision.canonicalize) {
    history.replaceState({ route: decision.route }, "", pathForRoute(decision.route));
  }

  if (decision.route.name === "login") {
    showLogin(decision.intendedPaneId);
  } else {
    applyRoute(decision.route);
  }
}

void bootstrap();

import "./styles.css";
import { fetchAgents } from "./api";
import type { AgentRow } from "./api";
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

type Route =
  | { name: "login" }
  | { name: "switcher" }
  | { name: "terminal"; agent: AgentRow | NewPaneRef };

const root = document.getElementById("app");

function navigate(route: Route): void {
  if (!root) return;
  root.innerHTML = "";
  switch (route.name) {
    case "login":
      renderLogin(root, { onSuccess: () => navigate({ name: "switcher" }) });
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
        onBack: () => navigate({ name: "switcher" }),
      });
      break;
  }
}

// A page load/refresh may already carry a valid session cookie — probing
// /api/agents (rather than defaulting to the login screen) lets a returning
// user land straight on the switcher without re-entering the token.
async function bootstrap(): Promise<void> {
  try {
    const agents = await fetchAgents();
    navigate(agents ? { name: "switcher" } : { name: "login" });
  } catch {
    navigate({ name: "login" });
  }
}

void bootstrap();

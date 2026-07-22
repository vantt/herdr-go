import { fetchAgents, fetchHealth, logout } from "../api";
import type { AgentRow, AgentStatus, ShellRow } from "../api";
import { renderCreateSheet } from "./create-sheet";
import type { NewPaneRef } from "../main";

export interface SwitcherProps {
  onSelect: (target: AgentRow | NewPaneRef) => void;
  onLoggedOut: () => void;
  onCreated: (ref: NewPaneRef) => void;
}

export interface WorkspaceGroup {
  workspace_id: string;
  workspace_label: string;
  workspace_status: AgentStatus;
  rows: AgentRow[];
}

// A single home-screen row: either an agent card or a shell entry (D1). Both
// render at the same granularity and share the workspace grouping below.
type HomeRow =
  | { type: "agent"; agent: AgentRow }
  | { type: "shell"; shell: ShellRow };

// A rendered group. `workspace_status` is null for a shell-only group, whose
// header shows no status badge at all (D7) — the badge is hidden on an
// agent-row count, never on any status value.
interface HomeGroup {
  workspace_id: string;
  workspace_label: string;
  workspace_status: AgentStatus | null;
  rows: HomeRow[];
}

const STATUS_LABEL: Record<AgentStatus, string> = {
  working: "Working",
  blocked: "Blocked",
  done: "Done",
  idle: "Idle",
  unknown: "Unknown",
};

const PULL_THRESHOLD = 64;

/**
 * Deterministically hashes a `kind` string to a stable HSL accent color (D4).
 * Same input always produces the same output; no per-kind lookup table, so an
 * unfamiliar `kind` still gets a valid, stable color with no code change.
 */
export function kindAccentColor(kind: string): string {
  let hash = 0;
  for (let i = 0; i < kind.length; i++) {
    hash = (hash * 31 + kind.charCodeAt(i)) | 0;
  }
  const hue = Math.abs(hash) % 360;
  return `hsl(${hue}, 45%, 50%)`;
}

/**
 * Groups rows by workspace_id, sorted alphabetically by workspace_label (D7).
 * Rows keep their original relative order within each group (D7, no new sort).
 */
export function groupByWorkspace(rows: AgentRow[]): WorkspaceGroup[] {
  const byId = new Map<string, WorkspaceGroup>();
  for (const row of rows) {
    let group = byId.get(row.workspace);
    if (!group) {
      group = {
        workspace_id: row.workspace,
        workspace_label: row.workspace_label,
        workspace_status: row.workspace_status,
        rows: [],
      };
      byId.set(row.workspace, group);
    }
    group.rows.push(row);
  }
  return Array.from(byId.values()).sort((a, b) => a.workspace_label.localeCompare(b.workspace_label));
}

/**
 * Builds the rendered home groups from the two lists. Agent rows reuse
 * `groupByWorkspace` unchanged; shell rows form their own groups keyed on
 * `ShellRow.workspace_id` (a different field name than `AgentRow.workspace`,
 * same concept). A shell pane's own `workspace_id` never also has an agent
 * (that overlap is already filtered out server-side), but a shell group DOES
 * merge into an agent group when their `workspace_label` strings are exactly
 * equal (D1/D2): the shell rows are appended onto the agent group's own rows,
 * and the agent group's non-null `workspace_status` is always kept, never the
 * shell group's null. Since `workspace_label` is not a unique key, two
 * unrelated workspaces that happen to share a label get merged into one
 * section -- an accepted risk (D1), not a bug. A shell group whose label
 * matches no agent group stays its own separate group, unchanged from before.
 * Combined groups sort by label, matching `groupByWorkspace`'s own ordering.
 */
export function buildHomeGroups(agents: AgentRow[], shells: ShellRow[]): HomeGroup[] {
  const agentGroups: HomeGroup[] = groupByWorkspace(agents).map((group) => ({
    workspace_id: group.workspace_id,
    workspace_label: group.workspace_label,
    workspace_status: group.workspace_status,
    rows: group.rows.map((agent): HomeRow => ({ type: "agent", agent })),
  }));

  const shellById = new Map<string, HomeGroup>();
  for (const shell of shells) {
    let group = shellById.get(shell.workspace_id);
    if (!group) {
      group = {
        workspace_id: shell.workspace_id,
        workspace_label: shell.workspace_label,
        workspace_status: null,
        rows: [],
      };
      shellById.set(shell.workspace_id, group);
    }
    group.rows.push({ type: "shell", shell });
  }

  // D1/D2: fold each shell-only group into the agent group sharing its exact
  // workspace_label, instead of leaving it as its own group. The shell rows
  // land inside the matched agent group's own rows array, so its non-null
  // workspace_status (and renderGroupBadge's agent-row check) stay untouched.
  const agentGroupByLabel = new Map(agentGroups.map((group) => [group.workspace_label, group]));
  const remainingShellGroups: HomeGroup[] = [];
  for (const shellGroup of shellById.values()) {
    const matchedAgentGroup = agentGroupByLabel.get(shellGroup.workspace_label);
    if (matchedAgentGroup) {
      matchedAgentGroup.rows.push(...shellGroup.rows);
    } else {
      remainingShellGroups.push(shellGroup);
    }
  }

  return [...agentGroups, ...remainingShellGroups].sort((a, b) =>
    a.workspace_label.localeCompare(b.workspace_label),
  );
}

export function renderSwitcher(root: HTMLElement, props: SwitcherProps): void {
  root.innerHTML = `
    <div class="view view-switcher">
      <header class="switcher-header">
        <div class="switcher-brand">
          <span class="health-dot" id="health-dot" aria-hidden="true"></span>
          <h1 class="switcher-title">herdr<span class="brand-dot">&middot;</span>gateway</h1>
        </div>
        <div class="switcher-actions">
          <button type="button" class="icon-btn" id="refresh-btn" aria-label="Refresh agent list">
            <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
              <path d="M17.65 6.35A8 8 0 1 0 19.9 13h-2.1a6 6 0 1 1-1.43-6.14L13 10h7V3l-2.35 3.35z" fill="currentColor"/>
            </svg>
          </button>
          <button type="button" class="icon-btn" id="logout-btn" aria-label="Log out">
            <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
              <path d="M10 17v-2H4V9h6V7l5 5-5 5zm-1-14h8a2 2 0 0 1 2 2v3h-2V5H4v14h13v-3h2v3a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z" fill="currentColor"/>
            </svg>
          </button>
        </div>
      </header>
      <main class="switcher-body" id="switcher-body">
        <p class="switcher-status" id="switcher-status">Loading agents&hellip;</p>
        <ul class="agent-list" id="agent-list" hidden></ul>
      </main>
      <button type="button" class="fab" id="create-fab" aria-label="New shell or agent" disabled>
        <svg viewBox="0 0 24 24" width="24" height="24" aria-hidden="true">
          <path d="M11 5h2v6h6v2h-6v6h-2v-6H5v-2h6V5z" fill="currentColor"/>
        </svg>
      </button>
      <div id="create-sheet-root"></div>
    </div>
  `;

  const body = root.querySelector<HTMLElement>("#switcher-body")!;
  const list = root.querySelector<HTMLUListElement>("#agent-list")!;
  const status = root.querySelector<HTMLParagraphElement>("#switcher-status")!;
  const healthDot = root.querySelector<HTMLSpanElement>("#health-dot")!;
  const refreshBtn = root.querySelector<HTMLButtonElement>("#refresh-btn")!;
  const logoutBtn = root.querySelector<HTMLButtonElement>("#logout-btn")!;
  const fabBtn = root.querySelector<HTMLButtonElement>("#create-fab")!;
  const createSheetRoot = root.querySelector<HTMLDivElement>("#create-sheet-root")!;

  // Same show/hide overlay pattern as terminal.ts's reply-sheet (D2), not a
  // route change — the switcher's own list/scroll state is untouched by open().
  const createSheet = renderCreateSheet(createSheetRoot, { onCreated: props.onCreated });

  // Session-only collapse state (D6): lives for as long as this view instance
  // is mounted, never persisted — a fresh renderSwitcher() call (page reload)
  // starts every section expanded again.
  const collapsedWorkspaces = new Set<string>();

  async function load(): Promise<void> {
    refreshBtn.classList.add("is-spinning");
    try {
      const snapshot = await fetchAgents();
      if (snapshot === null) {
        props.onLoggedOut();
        return;
      }
      renderList(snapshot.agents, snapshot.shells);
    } catch {
      status.hidden = false;
      status.textContent = "Could not reach the gateway.";
      list.hidden = true;
    } finally {
      refreshBtn.classList.remove("is-spinning");
    }
  }

  function renderAgentCard(row: AgentRow, index: number): string {
    const title = row.title || row.kind;
    const caption = row.tab_label ? `${row.kind} · ${row.tab_label}` : row.kind;
    const monogram = row.kind.charAt(0).toUpperCase();
    return `
        <li>
          <button type="button" class="agent-card" data-index="${index}">
            <span class="agent-watermark" aria-hidden="true" style="color: ${kindAccentColor(row.kind)}">${escapeHtml(monogram)}</span>
            <span class="agent-info">
              <span class="agent-path">${escapeHtml(title)}</span>
              <span class="agent-caption">${escapeHtml(caption)}</span>
            </span>
            <span class="status-badge status-${escapeHtml(row.status)}">
              <span class="status-dot" aria-hidden="true"></span>
              ${escapeHtml(STATUS_LABEL[row.status] ?? row.status)}
            </span>
          </button>
        </li>`;
  }

  // A shell entry (D1/D2/D6): the pane's folder as the primary line, a
  // "Shell · <tab>" caption, no kind watermark and no status badge at all.
  function renderShellRow(shell: ShellRow, index: number): string {
    const path = shell.path ?? "no folder yet";
    const caption = `Shell · ${shell.tab_label}`;
    return `
        <li>
          <button type="button" class="agent-card shell-row" data-index="${index}">
            <span class="agent-info">
              <span class="agent-path">${escapeHtml(path)}</span>
              <span class="agent-caption">${escapeHtml(caption)}</span>
            </span>
          </button>
        </li>`;
  }

  function renderRow(row: HomeRow, index: number): string {
    return row.type === "agent" ? renderAgentCard(row.agent, index) : renderShellRow(row.shell, index);
  }

  // D7: a group's header status badge is hidden entirely when the group has
  // zero agent rows — a client-side count over the rows, never a check on any
  // workspace_status value.
  function renderGroupBadge(group: HomeGroup): string {
    const status = group.workspace_status;
    if (status === null || !group.rows.some((row) => row.type === "agent")) return "";
    return `<span class="status-badge status-${escapeHtml(status)}">
                <span class="status-dot" aria-hidden="true"></span>
                ${escapeHtml(STATUS_LABEL[status] ?? status)}
              </span>`;
  }

  function renderWorkspaceSection(group: HomeGroup, indexOf: Map<HomeRow, number>): string {
    const collapsed = collapsedWorkspaces.has(group.workspace_id);
    return `
        <li class="workspace-group">
          <section class="workspace-section">
            <button
              type="button"
              class="workspace-header"
              data-workspace="${escapeHtml(group.workspace_id)}"
              aria-expanded="${collapsed ? "false" : "true"}"
            >
              <span class="workspace-header-label">
                <svg class="workspace-chevron" viewBox="0 0 24 24" width="14" height="14" aria-hidden="true">
                  <path d="M8 5l8 7-8 7V5z" fill="currentColor"/>
                </svg>
                ${escapeHtml(group.workspace_label)}
              </span>
              ${renderGroupBadge(group)}
            </button>
            <ul class="agent-list workspace-rows" ${collapsed ? "hidden" : ""}>
              ${group.rows.map((row) => renderRow(row, indexOf.get(row)!)).join("")}
            </ul>
          </section>
        </li>`;
  }

  function renderList(agents: AgentRow[], shells: ShellRow[]): void {
    if (agents.length === 0 && shells.length === 0) {
      status.hidden = false;
      status.textContent = "No active agents right now.";
      list.hidden = true;
      return;
    }
    status.hidden = true;
    list.hidden = false;

    const groups = buildHomeGroups(agents, shells);
    const flatRows = groups.flatMap((group) => group.rows);
    const indexOf = new Map(flatRows.map((row, i) => [row, i]));

    list.innerHTML =
      groups.length > 1
        ? groups.map((group) => renderWorkspaceSection(group, indexOf)).join("")
        : flatRows.map((row, i) => renderRow(row, i)).join("");

    list.querySelectorAll<HTMLButtonElement>(".agent-card").forEach((btn) => {
      btn.addEventListener("click", () => {
        const row = flatRows[Number(btn.dataset.index)];
        if (!row) return;
        if (row.type === "agent") {
          props.onSelect(row.agent);
        } else {
          const { pane_id, workspace_id, path, workspace_label } = row.shell;
          props.onSelect({ pane_id, workspace_id, label: path ?? workspace_label });
        }
      });
    });

    list.querySelectorAll<HTMLButtonElement>(".workspace-header").forEach((btn) => {
      btn.addEventListener("click", () => {
        const workspaceId = btn.dataset.workspace;
        if (!workspaceId) return;
        const rowsList = btn.parentElement?.querySelector<HTMLUListElement>(".workspace-rows");
        if (!rowsList) return;
        const nowCollapsed = !collapsedWorkspaces.has(workspaceId);
        if (nowCollapsed) {
          collapsedWorkspaces.add(workspaceId);
        } else {
          collapsedWorkspaces.delete(workspaceId);
        }
        rowsList.hidden = nowCollapsed;
        btn.setAttribute("aria-expanded", nowCollapsed ? "false" : "true");
      });
    });
  }

  async function loadHealth(): Promise<void> {
    const health = await fetchHealth();
    const up = !!health?.herdr_up;
    healthDot.classList.toggle("health-up", up);
    healthDot.classList.toggle("health-down", !!health && !up);
    healthDot.setAttribute(
      "title",
      health
        ? `herdr-go ${health.version} · ${up ? "herdr is up" : "herdr is unreachable"}`
        : "herdr is unreachable",
    );
    // S4: the FAB is disabled (not hidden) whenever herdr is unreachable,
    // driven by this same health check rather than a second one.
    fabBtn.disabled = !up;
  }

  fabBtn.addEventListener("click", () => createSheet.open());
  refreshBtn.addEventListener("click", () => void load());
  logoutBtn.addEventListener("click", () => {
    void (async () => {
      await logout();
      props.onLoggedOut();
    })();
  });

  // Pull-to-refresh: a downward drag past PULL_THRESHOLD while already
  // scrolled to the top re-fetches the list, mirroring native list apps.
  let touchStartY = 0;
  let pulling = false;
  body.addEventListener(
    "touchstart",
    (ev) => {
      pulling = body.scrollTop <= 0;
      touchStartY = ev.touches[0]?.clientY ?? 0;
    },
    { passive: true },
  );
  body.addEventListener(
    "touchmove",
    (ev) => {
      if (!pulling) return;
      const dy = (ev.touches[0]?.clientY ?? 0) - touchStartY;
      if (dy > PULL_THRESHOLD) {
        pulling = false;
        void load();
      }
    },
    { passive: true },
  );
  body.addEventListener("touchend", () => {
    pulling = false;
  });

  void load();
  void loadHealth();
}

function escapeHtml(value: string): string {
  const div = document.createElement("div");
  div.textContent = value;
  return div.innerHTML;
}

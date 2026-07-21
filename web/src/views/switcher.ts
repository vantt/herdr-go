import { fetchAgents, fetchHealth, logout } from "../api";
import type { AgentRow, AgentStatus } from "../api";
import { renderCreateSheet } from "./create-sheet";
import type { NewPaneRef } from "../main";

export interface SwitcherProps {
  onSelect: (agent: AgentRow) => void;
  onLoggedOut: () => void;
  onCreated: (ref: NewPaneRef) => void;
}

export interface WorkspaceGroup {
  workspace_id: string;
  workspace_label: string;
  workspace_status: AgentStatus;
  rows: AgentRow[];
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
      const rows = await fetchAgents();
      if (rows === null) {
        props.onLoggedOut();
        return;
      }
      renderList(rows);
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

  function renderWorkspaceSection(group: WorkspaceGroup, indexOf: Map<AgentRow, number>): string {
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
              <span class="status-badge status-${escapeHtml(group.workspace_status)}">
                <span class="status-dot" aria-hidden="true"></span>
                ${escapeHtml(STATUS_LABEL[group.workspace_status] ?? group.workspace_status)}
              </span>
            </button>
            <ul class="agent-list workspace-rows" ${collapsed ? "hidden" : ""}>
              ${group.rows.map((row) => renderAgentCard(row, indexOf.get(row)!)).join("")}
            </ul>
          </section>
        </li>`;
  }

  function renderList(rows: AgentRow[]): void {
    if (rows.length === 0) {
      status.hidden = false;
      status.textContent = "No active agents right now.";
      list.hidden = true;
      return;
    }
    status.hidden = true;
    list.hidden = false;

    const groups = groupByWorkspace(rows);
    const indexOf = new Map(rows.map((row, i) => [row, i]));

    list.innerHTML =
      groups.length > 1
        ? groups.map((group) => renderWorkspaceSection(group, indexOf)).join("")
        : rows.map((row, i) => renderAgentCard(row, i)).join("");

    list.querySelectorAll<HTMLButtonElement>(".agent-card").forEach((btn) => {
      btn.addEventListener("click", () => {
        const row = rows[Number(btn.dataset.index)];
        if (row) props.onSelect(row);
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

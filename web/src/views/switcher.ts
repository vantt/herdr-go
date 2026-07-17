import { fetchAgents, fetchHealth, logout } from "../api";
import type { AgentRow, AgentStatus } from "../api";

export interface SwitcherProps {
  onSelect: (agent: AgentRow) => void;
  onLoggedOut: () => void;
}

const STATUS_LABEL: Record<AgentStatus, string> = {
  working: "Working",
  blocked: "Blocked",
  done: "Done",
  idle: "Idle",
};

const PULL_THRESHOLD = 64;

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
    </div>
  `;

  const body = root.querySelector<HTMLElement>("#switcher-body")!;
  const list = root.querySelector<HTMLUListElement>("#agent-list")!;
  const status = root.querySelector<HTMLParagraphElement>("#switcher-status")!;
  const healthDot = root.querySelector<HTMLSpanElement>("#health-dot")!;
  const refreshBtn = root.querySelector<HTMLButtonElement>("#refresh-btn")!;
  const logoutBtn = root.querySelector<HTMLButtonElement>("#logout-btn")!;

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

  function renderList(rows: AgentRow[]): void {
    if (rows.length === 0) {
      status.hidden = false;
      status.textContent = "No active agents right now.";
      list.hidden = true;
      return;
    }
    status.hidden = true;
    list.hidden = false;
    list.innerHTML = rows
      .map(
        (row, i) => `
        <li>
          <button type="button" class="agent-card" data-index="${i}">
            <span class="agent-info">
              <span class="agent-path">${escapeHtml(row.display)}</span>
              <span class="agent-kind">${escapeHtml(row.kind)}</span>
            </span>
            <span class="status-badge status-${escapeHtml(row.status)}">
              <span class="status-dot" aria-hidden="true"></span>
              ${escapeHtml(STATUS_LABEL[row.status] ?? row.status)}
            </span>
          </button>
        </li>`,
      )
      .join("");

    list.querySelectorAll<HTMLButtonElement>(".agent-card").forEach((btn) => {
      btn.addEventListener("click", () => {
        const row = rows[Number(btn.dataset.index)];
        if (row) props.onSelect(row);
      });
    });
  }

  async function loadHealth(): Promise<void> {
    const health = await fetchHealth();
    healthDot.classList.toggle("health-up", !!health?.herdr_up);
    healthDot.classList.toggle("health-down", !!health && !health.herdr_up);
    healthDot.setAttribute("title", health?.herdr_up ? "herdr is up" : "herdr is unreachable");
  }

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

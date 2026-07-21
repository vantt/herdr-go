import { fetchCreateOptions, createPane, createAgent } from "../api";
import type { Destination, PresetOption } from "../api";
import type { NewPaneRef } from "../main";

export interface CreateSheetProps {
  onCreated: (ref: NewPaneRef) => void;
}

export interface CreateSheetControls {
  open: () => void;
  close: () => void;
}

/**
 * Returns the caveat text for a destination whose folder is missing or
 * stale (S2), or null for a destination that needs no caveat.
 */
function destinationCaveat(dest: Destination): string | null {
  if (dest.path === null) return "Folder not detected";
  if (!dest.path_is_live) return "Folder may be stale";
  return null;
}

/**
 * Renders the create bottom sheet (D2): a destination list, then a Shell
 * row and one row per agent preset. Mirrors terminal.ts's reply-sheet — a
 * hidden/shown div toggled by open()/close(), not a route change. The
 * destination list is re-fetched on every open() (never cached between
 * opens, per CONTEXT.md's Established Patterns).
 */
export function renderCreateSheet(root: HTMLElement, props: CreateSheetProps): CreateSheetControls {
  root.innerHTML = `
    <div class="create-sheet" id="create-sheet" hidden>
      <div class="sheet-head">
        <span class="reply-label">New shell or agent</span>
        <button type="button" class="sheet-x" id="create-sheet-close" aria-label="Close">✕</button>
      </div>
      <div class="create-sheet-body" id="create-sheet-body">
        <p class="create-sheet-status" id="create-sheet-status" hidden></p>
        <p class="create-sheet-error" id="create-sheet-error" role="alert" hidden></p>
        <ul class="destination-list" id="destination-list" hidden></ul>
        <ul class="action-list" id="action-list" hidden></ul>
      </div>
    </div>
  `;

  const sheet = root.querySelector<HTMLDivElement>("#create-sheet")!;
  const closeBtn = root.querySelector<HTMLButtonElement>("#create-sheet-close")!;
  const status = root.querySelector<HTMLParagraphElement>("#create-sheet-status")!;
  const errorEl = root.querySelector<HTMLParagraphElement>("#create-sheet-error")!;
  const destinationList = root.querySelector<HTMLUListElement>("#destination-list")!;
  const actionList = root.querySelector<HTMLUListElement>("#action-list")!;

  let destinations: Destination[] = [];
  let presets: PresetOption[] = [];
  let selectedIndex = -1;
  let submitting = false;

  function setStatus(text: string | null): void {
    status.hidden = text === null;
    status.textContent = text ?? "";
  }

  function setError(text: string | null): void {
    errorEl.hidden = text === null;
    errorEl.textContent = text ?? "";
  }

  function renderDestinations(): void {
    destinationList.innerHTML = destinations
      .map((dest, index) => {
        const caveat = destinationCaveat(dest);
        const selected = index === selectedIndex;
        return `
          <li>
            <button
              type="button"
              class="destination-row${selected ? " is-selected" : ""}"
              data-index="${index}"
              aria-pressed="${selected}"
            >
              <span class="destination-label">${escapeHtml(dest.label)}</span>
              <span class="destination-path">${escapeHtml(dest.path ?? "no folder yet")}</span>
              ${caveat ? `<span class="destination-caveat">${escapeHtml(caveat)}</span>` : ""}
            </button>
          </li>`;
      })
      .join("");

    destinationList.querySelectorAll<HTMLButtonElement>(".destination-row").forEach((btn) => {
      btn.addEventListener("click", () => {
        if (submitting) return;
        selectedIndex = Number(btn.dataset.index);
        renderDestinations();
      });
    });
  }

  function renderActions(): void {
    const rows = [
      `<li><button type="button" class="action-row" data-kind="shell">Shell</button></li>`,
      ...presets.map(
        (preset) =>
          `<li><button type="button" class="action-row" data-kind="preset" data-preset="${escapeHtml(preset.label)}">${escapeHtml(preset.label)}</button></li>`,
      ),
    ];
    actionList.innerHTML = rows.join("");

    actionList.querySelectorAll<HTMLButtonElement>(".action-row").forEach((btn) => {
      btn.addEventListener("click", () => {
        const preset = btn.dataset.kind === "preset" ? (btn.dataset.preset ?? null) : null;
        void handleAction(preset);
      });
    });
  }

  function setSubmitting(next: boolean): void {
    submitting = next;
    actionList.querySelectorAll<HTMLButtonElement>(".action-row").forEach((btn) => {
      btn.disabled = next;
    });
  }

  function finishCreate(ref: NewPaneRef): void {
    setSubmitting(false);
    close();
    props.onCreated(ref);
  }

  async function handleAction(preset: string | null): Promise<void> {
    if (submitting) return;
    const dest = destinations[selectedIndex];
    if (!dest) return;

    setSubmitting(true);
    setError(null);

    if (preset === null) {
      const result = await createPane(dest.workspace_id);
      if (!result.ok) {
        setSubmitting(false);
        setError(result.error);
        return;
      }
      finishCreate({ pane_id: result.pane_id, workspace_id: dest.workspace_id, label: dest.label });
      return;
    }

    const result = await createAgent(dest.workspace_id, preset);
    if (!result.ok) {
      setSubmitting(false);
      setError(result.error);
      return;
    }
    finishCreate({ pane_id: result.pane_id, workspace_id: dest.workspace_id, label: dest.label, name: result.name });
  }

  async function load(): Promise<void> {
    setError(null);
    selectedIndex = -1;
    destinationList.hidden = true;
    actionList.hidden = true;
    setStatus("Loading…");

    let options;
    try {
      options = await fetchCreateOptions();
    } catch {
      setStatus("Could not load destinations.");
      return;
    }

    if (options === null) {
      setStatus("Session expired. Log in again to continue.");
      return;
    }

    destinations = options.destinations;
    presets = options.presets;

    if (destinations.length === 0) {
      setStatus("No destinations available.");
      return;
    }

    selectedIndex = 0;
    setStatus(null);
    renderDestinations();
    renderActions();
    destinationList.hidden = false;
    actionList.hidden = false;
  }

  function open(): void {
    sheet.hidden = false;
    void load();
  }

  function close(): void {
    sheet.hidden = true;
  }

  closeBtn.addEventListener("click", close);

  return { open, close };
}

function escapeHtml(value: string): string {
  const div = document.createElement("div");
  div.textContent = value;
  return div.innerHTML;
}

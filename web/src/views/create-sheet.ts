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

type DropdownField = "destination" | "type" | null;

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
 * Computes the S1 disambiguating suffix per destination: the last 4
 * characters of its own workspace_id, but only for entries whose
 * {label, path} collides with another entry in the same list (including
 * two entries that both have path: null). Returns null for an index with
 * no collision, so its label renders exactly as Phase 1 shipped.
 */
function collisionSuffixes(destinations: Destination[]): (string | null)[] {
  return destinations.map((dest, index) => {
    const collides = destinations.some(
      (other, otherIndex) =>
        otherIndex !== index && other.label === dest.label && other.path === dest.path,
    );
    return collides ? dest.workspace_id.slice(-4) : null;
  });
}

/**
 * Renders the create bottom sheet: a Destination dropdown and a Type
 * dropdown (Shell + one row per agent preset), plus an explicit "New"
 * button that is the only action that creates anything (D1/D2). Mirrors
 * terminal.ts's reply-sheet — a hidden/shown div toggled by open()/close(),
 * not a route change. The destination list is re-fetched on every open()
 * (never cached between opens, per CONTEXT.md's Established Patterns).
 *
 * Each dropdown is a trigger button (aria-haspopup="listbox", aria-expanded)
 * showing a one-line summary of the current selection, plus a popup listbox
 * (the field's own <ul>) that opens over the sheet. Opening one dropdown
 * closes the other (D7) via the single `openDropdown` state below.
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
        <div class="dropdown-field" id="destination-field" hidden>
          <button
            type="button"
            class="dropdown-trigger"
            id="destination-trigger"
            aria-haspopup="listbox"
            aria-expanded="false"
          >
            <span class="dropdown-trigger-label">Destination</span>
            <span class="dropdown-trigger-value" id="destination-trigger-value"></span>
          </button>
          <ul class="destination-list dropdown-popup" id="destination-list" hidden></ul>
        </div>
        <div class="dropdown-field" id="type-field" hidden>
          <button
            type="button"
            class="dropdown-trigger"
            id="type-trigger"
            aria-haspopup="listbox"
            aria-expanded="false"
          >
            <span class="dropdown-trigger-label">Type</span>
            <span class="dropdown-trigger-value" id="type-trigger-value"></span>
          </button>
          <ul class="action-list dropdown-popup" id="action-list" hidden></ul>
        </div>
        <button type="button" class="btn btn-primary btn-block" id="create-sheet-new" hidden>New</button>
      </div>
    </div>
  `;

  const sheet = root.querySelector<HTMLDivElement>("#create-sheet")!;
  const closeBtn = root.querySelector<HTMLButtonElement>("#create-sheet-close")!;
  const status = root.querySelector<HTMLParagraphElement>("#create-sheet-status")!;
  const errorEl = root.querySelector<HTMLParagraphElement>("#create-sheet-error")!;
  const destinationField = root.querySelector<HTMLDivElement>("#destination-field")!;
  const destinationTrigger = root.querySelector<HTMLButtonElement>("#destination-trigger")!;
  const destinationTriggerValue = root.querySelector<HTMLSpanElement>("#destination-trigger-value")!;
  const destinationList = root.querySelector<HTMLUListElement>("#destination-list")!;
  const typeField = root.querySelector<HTMLDivElement>("#type-field")!;
  const typeTrigger = root.querySelector<HTMLButtonElement>("#type-trigger")!;
  const typeTriggerValue = root.querySelector<HTMLSpanElement>("#type-trigger-value")!;
  const actionList = root.querySelector<HTMLUListElement>("#action-list")!;
  const newButton = root.querySelector<HTMLButtonElement>("#create-sheet-new")!;

  let destinations: Destination[] = [];
  let presets: PresetOption[] = [];
  let selectedIndex = -1;
  let selectedPreset: string | null = null;
  let openDropdown: DropdownField = null;
  let submitting = false;

  function setStatus(text: string | null): void {
    status.hidden = text === null;
    status.textContent = text ?? "";
  }

  function setError(text: string | null): void {
    errorEl.hidden = text === null;
    errorEl.textContent = text ?? "";
  }

  /** Applies `openDropdown` to both popups' visibility and aria-expanded (D7). */
  function renderDropdownState(): void {
    const destOpen = openDropdown === "destination";
    const typeOpen = openDropdown === "type";
    destinationList.hidden = !destOpen;
    actionList.hidden = !typeOpen;
    destinationTrigger.setAttribute("aria-expanded", String(destOpen));
    typeTrigger.setAttribute("aria-expanded", String(typeOpen));
  }

  function setOpenDropdown(next: DropdownField): void {
    openDropdown = next;
    renderDropdownState();
  }

  /** Refreshes each trigger's one-line summary from the current selection. */
  function updateTriggerValues(): void {
    const dest = destinations[selectedIndex];
    if (dest) {
      const suffix = collisionSuffixes(destinations)[selectedIndex];
      const label = `${dest.label}${suffix ? ` · ${suffix}` : ""}`;
      destinationTriggerValue.textContent = `${label} — ${dest.path ?? "no folder yet"}`;
    } else {
      destinationTriggerValue.textContent = "";
    }
    typeTriggerValue.textContent = selectedPreset ?? "Shell";
  }

  function renderDestinations(): void {
    const suffixes = collisionSuffixes(destinations);
    destinationList.innerHTML = destinations
      .map((dest, index) => {
        const caveat = destinationCaveat(dest);
        const selected = index === selectedIndex;
        const suffix = suffixes[index];
        return `
          <li>
            <button
              type="button"
              class="destination-row${selected ? " is-selected" : ""}"
              data-index="${index}"
              aria-pressed="${selected}"
            >
              <span class="destination-label">${escapeHtml(dest.label)}${suffix ? ` · ${escapeHtml(suffix)}` : ""}</span>
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
        updateTriggerValues();
        setOpenDropdown(null);
      });
    });
  }

  function renderActions(): void {
    const rows = [
      `<li><button type="button" class="action-row${selectedPreset === null ? " is-selected" : ""}" data-kind="shell" aria-pressed="${selectedPreset === null}">Shell</button></li>`,
      ...presets.map((preset) => {
        const selected = selectedPreset === preset.label;
        return `<li><button type="button" class="action-row${selected ? " is-selected" : ""}" data-kind="preset" data-preset="${escapeHtml(preset.label)}" aria-pressed="${selected}">${escapeHtml(preset.label)}</button></li>`;
      }),
    ];
    actionList.innerHTML = rows.join("");

    actionList.querySelectorAll<HTMLButtonElement>(".action-row").forEach((btn) => {
      btn.addEventListener("click", () => {
        if (submitting) return;
        selectedPreset = btn.dataset.kind === "preset" ? (btn.dataset.preset ?? null) : null;
        renderActions();
        updateTriggerValues();
        setOpenDropdown(null);
      });
    });
  }

  function setSubmitting(next: boolean): void {
    submitting = next;
    newButton.disabled = next;
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
    selectedPreset = null;
    setOpenDropdown(null);
    destinationField.hidden = true;
    typeField.hidden = true;
    newButton.hidden = true;
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
    selectedPreset = null;
    setStatus(null);
    renderDestinations();
    renderActions();
    updateTriggerValues();
    destinationField.hidden = false;
    typeField.hidden = false;
    newButton.hidden = false;
  }

  function open(): void {
    sheet.hidden = false;
    void load();
  }

  function close(): void {
    sheet.hidden = true;
    setOpenDropdown(null);
  }

  destinationTrigger.addEventListener("click", () => {
    setOpenDropdown(openDropdown === "destination" ? null : "destination");
  });

  typeTrigger.addEventListener("click", () => {
    setOpenDropdown(openDropdown === "type" ? null : "type");
  });

  newButton.addEventListener("click", () => {
    void handleAction(selectedPreset);
  });

  // Clicking anywhere outside an open popup closes it.
  document.addEventListener("click", (event) => {
    if (openDropdown === null) return;
    const target = event.target as Node | null;
    if (target && (destinationField.contains(target) || typeField.contains(target))) return;
    setOpenDropdown(null);
  });

  closeBtn.addEventListener("click", close);

  return { open, close };
}

function escapeHtml(value: string): string {
  const div = document.createElement("div");
  div.textContent = value;
  return div.innerHTML;
}

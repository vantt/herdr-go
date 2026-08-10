function escapeHtml(value: string): string {
  const div = document.createElement("div");
  div.textContent = value;
  return div.innerHTML;
}

// Per-kind brand signature (design handoff docs/design/handoff/Shell.html):
// a light OKLCH hue for the card's background wash + big watermark, a
// lighter "mark" hue for the small inline caption icon, and the icon
// geometry shared by both. Hand-drawn placeholders per the handoff --
// swap for licensed vendor marks before ship. A kind not in this map falls
// back to the existing hashed-hue + first-letter watermark (kindAccentColor),
// unchanged from before this design pass.
export const KIND_MARKS: Record<string, { hue: string; mark: string; icon: string }> = {
  claude: {
    hue: ".7 .16 40",
    mark: ".82 .09 40",
    icon: `<path d="M12 3v18M4.2 7.5l15.6 9M19.8 7.5l-15.6 9"/>`,
  },
  codex: {
    hue: ".75 .13 190",
    mark: ".85 .07 190",
    icon: `<path d="M9 7l-5 5 5 5"/><path d="M15 7l5 5-5 5"/>`,
  },
  agy: {
    hue: ".7 .14 300",
    mark: ".82 .08 300",
    icon: `<path d="M12 4l8 14H4z"/><path d="M12 12l4 6H8z"/>`,
  },
  pi: {
    hue: ".72 .13 130",
    mark: ".84 .07 130",
    icon: `<path d="M4 7h16"/><path d="M9 7v11"/><path d="M16 7v9a2 2 0 0 0 3 1"/>`,
  },
};

export function renderAgentWatermark(kind: string): string {
  const style = KIND_MARKS[kind];
  if (!style) {
    const monogram = kind.charAt(0).toUpperCase();
    return `<span class="agent-watermark agent-watermark--letter" aria-hidden="true" style="color: ${kindAccentColor(kind)}">${escapeHtml(monogram)}</span>`;
  }
  return `<span class="agent-watermark agent-watermark--icon" aria-hidden="true" style="color: oklch(${style.hue})">
      <svg width="86" height="86" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">${style.icon}</svg>
    </span>`;
}

export function renderKindInlineMark(kind: string): string {
  const style = KIND_MARKS[kind];
  if (!style) return "";
  return `<span class="agent-kind-mark" aria-hidden="true" style="color: oklch(${style.mark})">
      <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">${style.icon}</svg>
    </span>`;
}

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

/** The gradient wash background for a known kind, same treatment as the home agent-card (D7 brand signature). Empty string for an unknown kind (caller falls back to its own default background). */
export function kindCardBackground(kind: string): string {
  const style = KIND_MARKS[kind];
  if (!style) return "";
  return `linear-gradient(100deg, var(--bg-elevated) 40%, oklch(${style.hue} / .07) 100%), var(--bg-elevated)`;
}

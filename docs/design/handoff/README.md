# Handoff: Shell card on Home (herdr·gateway)

## Overview

Home currently lists **agent panes** only. Panes running a plain shell are filtered out, so a workspace whose panes are all shells is invisible on Home. This change lists shells too.

A shell must **not** borrow the agent status scale. A shell has no status — nothing is working, blocked, or finishing. `IDLE` would be a lie about a real state; `UNKNOWN` is reserved for "the desktop reported a state we don't recognise". **A shell is the absence of an agent, not a state of one.**

Three things ship together:

1. **Shell card** — a new, quieter card type in the same list.
2. **Workspace rollup slot** — what the section header shows when a workspace has no agents.
3. **Workspace label disambiguation** — when two workspaces share a label.

Also included in the same pass (already approved in design): the **agent card gains a per-kind brand signature** and its **status pill moves to a bottom row** so the title gets full card width.

## About the design files

The files in this bundle are **design references created in HTML** — prototypes showing intended look and behaviour, not production code to copy directly. The task is to **recreate them in the target codebase's existing environment** (React, Vue, Svelte, whatever herdr·gateway uses) with its established patterns and component conventions. The inline styles in the prototype are an authoring artefact; port them to the project's stylesheet/CSS-module/utility conventions.

## Fidelity

**High-fidelity.** Colours, type, spacing, radii, motion and states are final and expressed in the existing herdr·gateway token scales. Implement pixel-faithfully.

One exception: the four agent **kind marks** (claude / codex / agy / pi) are hand-drawn placeholder SVGs in the product's existing icon register (1.4–2.2 stroke, round caps, no fill). Replace with licensed vendor marks before ship; keep the geometry rules and the opacity ceilings below.

## Design system note

This design is built on **herdr·gateway's own token set** (dark-only near-black slate + one blue accent + four reserved status colours), documented in Design Tokens below. Do not introduce a new saturated hue — the four status colours plus the blue accent are the entire saturated palette.

---

## Screens / views

### A. Home — flat mixed list

**Purpose:** the operator scans "is anything blocked / still working / done?" and taps to open a terminal full screen.

**Layout**
- Single column, phone-first. Real target 360–430px; content caps at **640px** and centres above that. No desktop layout.
- Sticky header: `padding 14px 16px`, `border-bottom 1px solid --border`, background `--bg`. Health dot 8px, wordmark mono `0.9375rem/600`, two 34px circular icon buttons (refresh, logout).
- List: `display:flex; flex-direction:column; gap:12px; padding:16px 16px 88px`.
- **Bottom-right is reserved** for a planned FAB (52px circle, inset 16px). List bottom padding is `88px` (= 52 + 16 + 20 clearance) so the last card never sits under it. Put nothing load-bearing in that corner.
- Pull-to-refresh on the body, unchanged.

**Order:** unchanged from today — shells appear inline in the same list, in the same sort position their pane already occupies.

### B. Home — workspace sections

Used when more than one workspace is present. Sections sorted alphabetically by label; collapsible.

**Section header** — full-width `<button aria-expanded>`, `padding:12px 0`, `gap:10px`:
- chevron 14px, `--text-muted`, rotates `0deg → 90deg` when expanded, `transform 220ms cubic-bezier(.4,0,.2,1)`
- label: sans `0.8125rem / 600 / --text-secondary`, `flex:1`, `min-width:0`
- right slot: rollup pill **or** shell count **or** nothing (see C)

### C. Edge states

- **Workspace with only shells** → right slot shows a neutral count, never a pill.
- **All-shells list** → renders normally, calm, no status colour anywhere. Not an empty state.
- **Single shell alone** → renders as one real tappable card. Do not substitute an empty-state message.
- The existing centred status lines (`Loading agents…` / `No active agents right now.` / `Could not reach the gateway.`) are unchanged and still mutually exclusive with the list.

---

## Components

### 1. Shell card (NEW)

Element: `<button>`. The whole card is one tap target.

| Property | Value |
|---|---|
| display | `flex; align-items:center; gap:11px` |
| background | `transparent` |
| border | `1px solid var(--border)` |
| border-radius | `12px` (radius `md`) |
| padding | `0 16px` |
| min-height | **`48px`** (single line; agent cards are ~85px) |
| box-shadow | **none** (agent cards carry `--shadow`) |

Children, left to right:

1. **Prompt glyph** — plain text character `>`, mono `0.9375rem / 500`, `--text-muted`, `flex:none`, `aria-hidden="true"`. Not an icon; no SVG.
2. **Label** — mono `0.875rem / 500`, `--text-secondary`; the tab label suffix (`· zsh`) is a nested span in `--text-muted`. `white-space:nowrap; overflow:hidden; text-overflow:ellipsis; min-width:0; flex:1`.
   - Content precedence: `workingFolderPath` → then ` · {tabLabel}` if present → else ` · {paneTitle}` if present → else path alone.
   - **Long paths:** single line, truncate. Prototype shows a head-truncation variant so the tail of the path survives: `direction:rtl; text-align:left` on the truncating span **plus** the text isolated as LTR inside it — `<bdi dir="ltr" style="unicode-bidi:isolate">…</bdi>`. The isolate is mandatory: without it the neutral leading `~/` is bidi-reordered to the end of the line and the path renders corrupted. Verify the rendered string starts with `…` and ends with the folder name. If the platform can't do this, plain tail ellipsis is acceptable.
3. **Caret block** — `7 × 15px`, `background --text-muted`, `border-radius 1px`, `flex:none`, `aria-hidden="true"`. Decorative only. **Render on the first shell card in a list only** if you want to keep the list quiet, or on all — design allows either; the prototype shows it on the first.

**Deliberately absent:** status pill, status colour, elevated surface, shadow, watermark, brand mark, kind caption. Their absence is the signal.

**Accessible name:** `Shell terminal · {path}{ · tabLabel}`. Never announce a status.

**Not colour-alone:** the shell/agent distinction rests on height (48 vs ~85), surface (transparent vs elevated+shadow), typeface (mono vs sans title), and the presence/absence of a pill — colour is not carrying it.

### 2. Agent card (MODIFIED)

Element: `<button>`. Layout changes from a two-column row to a **two-row column**:

```
[ title — full card width, up to 2 lines ]
[ caption + kind mark ............ status pill ]
```

| Property | Value |
|---|---|
| display | `flex; flex-direction:column; align-items:stretch; gap:10px` |
| background | `linear-gradient(100deg, var(--bg-elevated) 40%, {kindHue}/.07 100%), var(--bg-elevated)` |
| border | `1px solid var(--border)` |
| border-radius | `12px` |
| padding | `16px` |
| overflow | `hidden` |
| box-shadow | `var(--shadow)` |

- **Title** — sans `0.9375rem / 500 / 1.35`, `--text-primary`, clamped to 2 lines (`-webkit-line-clamp:2`), then ellipsis. Now spans the full card width.
- **Bottom row** — `display:flex; align-items:center; justify-content:space-between; gap:12px`.
  - **Caption** — mono `0.75rem`, `--text-muted`, preceded by an 11px kind mark at the kind's light hue. Truncates with ellipsis; `min-width:0`.
  - **Status pill** — unchanged vocabulary (see 4). `flex:none`.
- **Brand watermark** — the per-kind mark, `86 × 86`, `stroke-width:1.4`, positioned `absolute; right:-14px; top:50%; transform:translateY(-50%)`, `opacity:.13`, `pointer-events:none`, `aria-hidden`. **Replaces the old hashed letter watermark.** For an unrecognised kind, fall back to the existing hashed-hue + first-letter watermark.

**Brand colour ceiling — enforce this:** kind hues never exceed **7%** as a background wash and **13%** as a mark. Vendor brand colours are saturated and some sit close to the status colours (Claude's coral is ~8° from `--status-blocked`). Shape carries identity; colour only warms it.

### 3. Workspace header rollup slot (MODIFIED)

Three mutually exclusive right-slot states:

| Condition | Slot content |
|---|---|
| Workspace contains ≥1 agent | **Status pill**, unchanged. Worst active status wins: `blocked > working > done > idle > unknown`. |
| Workspace contains only shells | **Count label** — `{n} shells` (singular `1 shell`), mono `0.75rem / 500`, `--text-muted`. No dot, no wash, no pill shape, no border-radius. |
| Workspace is empty | Section is not rendered. |

The count is deliberately *typographic, not a pill* — it can never be misread as a status, and it fills a slot an empty gap would make look like a failed load.

### 4. Status pill (UNCHANGED — reference)

`display:inline-flex; align-items:center; gap:7px; border-radius:999px; padding:5px 10px;`
7px dot (`background:currentColor`) + label sans `0.6875rem / 700`, `letter-spacing:.04em`, UPPERCASE.
`color` = status colour; `background` = same colour at 14%.

| status | label | colour | dot animation |
|---|---|---|---|
| working | `WORKING` | `#f5b544` | `pulse 1.6s cubic-bezier(.4,0,.2,1) infinite` |
| blocked | `BLOCKED` | `#f2545b` | `blink 1.1s steps(1,end) infinite` |
| done | `DONE` | `#34d399` | none |
| idle | `IDLE` | `#7c879f` | none |
| unknown | `UNKNOWN` | `#7c879f` | none |

**Shells never render this component.**

### 5. Workspace label disambiguator (NEW)

Rendered inline after the label, **only when two or more visible workspaces share the same label string**:

```
background: var(--bg-elevated-2);
border: 1px solid var(--border);
border-radius: 4px;
padding: 2px 5px;
font: 500 0.6875rem/1 var(--mono);
color: var(--text-muted);
flex: none;
```

Content: `#` + the short form of the numeric workspace id (last 4 hex/digits, e.g. `#a3f2`). Unique labels show **no chip at all** — the common case stays clean.

Detection: group visible workspaces by label; if `group.length > 1`, every member of that group gets a chip. Do not chip the whole list.

---

## Interactions & behaviour

| Interaction | Behaviour |
|---|---|
| Tap card (agent or shell) | Opens that pane's terminal live, full screen. Identical for both types. |
| Press feedback (both types) | `transform: scale(.985)` + surface darkens to `--bg-pressed`; `150ms cubic-bezier(.4,0,.2,1)`. |
| Hover (both types) | `border-color: var(--border-strong)`. |
| Focus-visible | `outline: 2px solid var(--accent); outline-offset: 2px`. |
| Tap header | Toggles section collapse; chevron rotates `220ms`. `aria-expanded` reflects state. |
| Swipe / long-press | None. |
| Pull-to-refresh | Existing behaviour, unchanged. |

**Animations**
- `pulse` — `0%,100% {opacity:1; transform:scale(1)} 50% {opacity:.35; transform:scale(.82)}`, `1.6s`, ease.
- `blink` — `0%,49% {opacity:1} 50%,100% {opacity:.15}`, `1.1s`, `steps(1,end)`.
- `caret` — `0%,49% {opacity:.85} 50%,100% {opacity:0}`, `1.1s`, `steps(1,end)`. Shell caret only; decorative.
- **`prefers-reduced-motion: reduce` collapses every animation and transition** (`animation:none; transition:none`). The caret becomes a static block; the shell card loses nothing semantic.

---

## State / data

Per list item the client needs:

```
type Pane = {
  id: string
  workspaceId: number        // numeric, used for the collision chip
  workspaceLabel: string
  workingFolderPath: string
  tabLabel?: string
  paneTitle?: string
  kind: 'agent' | 'shell'
  agentKind?: string         // 'claude' | 'codex' | 'agy' | 'pi' | unrecognised string — agents only
  status?: 'working' | 'blocked' | 'done' | 'idle' | 'unknown'  // agents only
}
```

- The filter that dropped `kind === 'shell'` from the Home list is **removed**.
- A shell carries **no** `agentKind` and **no** `status`. Do not synthesise one; do not default to `idle`.
- Rollup per workspace: `panes.filter(p => p.kind === 'agent')` → if empty, render the shell count (`panes.length`), else reduce statuses by severity `blocked > working > done > idle > unknown`.
- Label collision: computed across the **visible** workspace list, recomputed on refresh.

---

## Design tokens

Existing tokens — no new values invented.

```
Surfaces   --bg #0b0e14   --bg-elevated #12161f   --bg-elevated-2 #171c28   --bg-pressed #1d2330
Borders    --border #232937   --border-strong #313a4d
Text       --text-primary #eef1f8   --text-secondary #9aa4ba   --text-muted #7c879f
Accent     --accent #4f8cff   --accent-strong #78a6ff   --accent-wash rgba(79,140,255,.14)
Status     working #f5b544   blocked #f2545b   done #34d399   idle/unknown #7c879f
Type       sans: system stack        mono: ui-monospace / JetBrains Mono / SF Mono
           2xs .6875 · xs .75 · sm .8125 · base .9375 · md 1 · lg 1.125 · xl 1.375 · 2xl 1.75 (rem)
Spacing    4 · 8 · 12 · 16 · 20 · 24 · 32 · 40 · 48 (px)
Radius     sm 8 · md 12 · lg 18 · full 999
Motion     ease cubic-bezier(.4,0,.2,1) · fast 150ms · normal 220ms
Shadow     0 1px 0 rgba(255,255,255,.02) inset, 0 8px 24px rgba(0,0,0,.28)
```

New values introduced by this design, all composed from the scales above:

| Name | Value | Where |
|---|---|---|
| shell card surface | `transparent` | shell card background |
| shell card border | `1px solid var(--border)` | shell card |
| shell card height | `min-height: 48px` | shell card (agent cards are ~85px) |
| shell card padding | `0 16px` | shell card |
| shell card gap | `11px` | shell card children |
| shell label colour | `var(--text-secondary)` | mono `0.875rem/500` |
| shell glyph / caption / caret | `var(--text-muted)` | prompt `>`, `· zsh`, caret |
| shell caret size | `7 × 15px`, radius `1px` | decorative |
| rollup count | mono `0.75rem/500`, `var(--text-muted)` | shells-only header slot |
| workspace-id chip | bg `--bg-elevated-2`, border `1px --border`, radius `4px`, pad `2px 5px`, mono `0.6875rem/500`, `--text-muted` | collision only |
| agent card row gap | `10px` | title ↔ bottom row |
| kind wash | `linear-gradient(100deg, --bg-elevated 40%, {hue}/.07 100%)` | agent card bg |
| kind watermark | `86px`, stroke `1.4`, `opacity .13`, `right:-14px`, centred Y | agent card |
| kind inline mark | `11px`, stroke `2.2` | agent caption |
| list bottom padding | `88px` | FAB reserve (52 + 16 + 20) |

**Agent kind hues** (oklch, chosen ≥25° from every status hue at usable chroma):

| kind | base hue | mark colour |
|---|---|---|
| claude | `oklch(.7 .16 40)` | `oklch(.82 .09 40)` |
| codex | `oklch(.75 .13 190)` | `oklch(.85 .07 190)` |
| agy | `oklch(.7 .14 300)` | `oklch(.82 .08 300)` |
| pi | `oklch(.72 .13 130)` | `oklch(.84 .07 130)` |
| unknown kind | existing hash → hue | existing hashed letter watermark |

---

## Accessibility

- Both card types are `<button>`. Accessible names: `Shell terminal · {path} · {tab}` / `{Kind} agent · {title} · {status}`.
- Shell cards must never announce a status.
- Section headers carry `aria-expanded`; the chevron is `aria-hidden`.
- Every decorative element (prompt glyph, caret, watermark, inline kind mark, FAB placeholder) is `aria-hidden="true"`.
- Tap targets: shell card 48px, agent card ~85px, header ~44px, icon buttons 34px in a ≥44px row — all at or above the 44px minimum in their hit area.
- Contrast at the quiet end: `--text-secondary #9aa4ba` on `--bg #0b0e14` ≈ 8.6:1; `--text-muted #7c879f` on `--bg` ≈ 6.2:1. Both clear AA for the sizes used.
- All animation is decorative and collapses under `prefers-reduced-motion: reduce`.

## Assets

No image assets. Every glyph is inline SVG drawn to the product's existing register: **1.3–2.2 stroke, round caps and joins, no fill, `currentColor`**. No icon library.

The four agent kind marks are **placeholders** — swap for licensed vendor marks at the same box size, stroke weight and opacity ceilings. Everything else (chevron, refresh, logout, terminal prompt `>`, caret) is final.

## Files

| File | Contents |
|---|---|
| `Shell Card — Handoff.dc.html` | **Implement from this.** UI only, no annotations: mixed list, workspace-sectioned list, shell-card state board (default / hover / pressed / focus / long path / no tab label), workspace header variants, agent kind reference, and the two edge lists. |
| `Shell Card.dc.html` | Full exploration with all directions and rationale. Reference only — do **not** implement from this file. |
| `support.js` | Runtime required to open the `.dc.html` files locally. Not part of the design. |

Open either HTML file directly in a browser.

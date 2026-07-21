# Design brief — shell panes on the Home screen

Paste everything below the line into Claude Design.

---

## What I need

A visual design for **one new card type** on the Home screen of an existing mobile web app, plus the two knock-on states it creates. This is an addition to a live design system, not a redesign — everything else on the screen stays as it is.

## The product, in one paragraph

**herdr·gateway** is a phone-first web app that lets an operator reach their desktop coding sessions from their phone. On the desktop there is a terminal multiplexer called herdr, organised as *workspaces* (roughly: one project folder), each containing *panes* (terminals). Some panes run an AI coding agent that works autonomously for minutes at a time; the operator wants to check on them from their phone — "is it still working, is it stuck waiting for me, is it done". That is what Home is for. Tapping a card opens that terminal live, full screen.

## The screen today

Dark-only, single column, phone-first. Top to bottom:

1. **Sticky header** — a small health dot, the wordmark `herdr·gateway` in monospace, then two circular icon buttons on the right (refresh, logout).
2. **Body** — either a single centred status line (`Loading agents…` / `No active agents right now.` / `Could not reach the gateway.`), or the list. Never both.
3. **The list** — if all sessions are in one workspace, a flat list of cards. If more than one workspace, the list splits into collapsible sections, one per workspace, sorted alphabetically.

Pull-to-refresh works on the body. Above 640px wide the content just caps at 640px and centres — there is no desktop layout.

### Anatomy of an agent card (today)

The whole card is one big tap target: a rounded rectangle, 12px radius, elevated near-black surface on a darker near-black page, 1px subtle border, 16px padding, flex row with `space-between`.

- **Watermark** — a single large letter (the first letter of the agent's kind, e.g. `C` for Claude), 48px, weight 700, opacity 0.12, absolutely positioned against the right edge and vertically centred. It sits *behind* the content as decoration. Its colour is derived by hashing the agent kind into a hue — so each agent kind gets its own stable, muted colour.
- **Info column** (left, flexible width)
  - Title line: the working folder or session title. Sans-serif, 0.9375rem, weight 500, wraps to a maximum of 2 lines then ellipsises.
  - Caption line: `{agent kind} · {tab label}`, monospace, 0.75rem, muted grey.
- **Status pill** (right) — a fully rounded pill: a 7px dot plus an UPPERCASE label at 0.6875rem, weight 700, letter-spacing 0.04em. The pill's text colour is the status colour and its background is a 14%-opacity wash of the same colour.

Press feedback: the card scales to 0.985 and its surface darkens. Hover brightens the border. No swipe, no long-press.

### Workspace section header (when there is more than one workspace)

A full-width button: a chevron that rotates 90°→0° between expanded and collapsed, the workspace label in secondary grey at 0.8125rem weight 600, and — on the right — a **rollup status pill using the exact same visual vocabulary as the card pill**, summarising the whole workspace.

### The status scale — reserved, load-bearing colour

The four status colours are the only saturated colours in the app besides one blue accent. They mean:

| status | label | colour | animation |
|---|---|---|---|
| working | WORKING | amber `#f5b544` | dot pulses, 1.6s ease-in-out |
| blocked | BLOCKED | red `#f2545b` | dot blinks, 1.1s hard steps |
| done | DONE | green `#34d399` | none |
| idle | IDLE | grey `#7c879f` | none |
| unknown | UNKNOWN | grey `#7c879f` | none |

`unknown` means "the desktop reported a state this app does not recognise" — it is an error-ish state, not a neutral one.

### Design tokens in use

```
Surfaces      --bg #0b0e14   --bg-elevated #12161f   --bg-elevated-2 #171c28   --bg-pressed #1d2330
Borders       --border #232937   --border-strong #313a4d
Text          --text-primary #eef1f8   --text-secondary #9aa4ba   --text-muted #7c879f
Accent        --accent #4f8cff   --accent-strong #78a6ff   --accent-wash rgba(79,140,255,0.14)
Type          sans: system stack   mono: ui-monospace / JetBrains Mono / SF Mono
              2xs .6875 · xs .75 · sm .8125 · base .9375 · md 1 · lg 1.125 · xl 1.375 · 2xl 1.75 (rem)
Spacing       4 · 8 · 12 · 16 · 20 · 24 · 32 · 40 · 48 (px)
Radius        sm 8 · md 12 · lg 18 · full 999
Motion        ease cubic-bezier(.4,0,.2,1) · fast 150ms · normal 220ms
Shadow        0 1px 0 rgba(255,255,255,.02) inset, 0 8px 24px rgba(0,0,0,.28)
```

Dark only, deliberately — the embedded terminal's colours are hardcoded dark, so the app ignores the OS light/dark preference. `prefers-reduced-motion` collapses every animation. Icons are hand-drawn inline SVG; there is no icon font or icon library, so any new icon must be a simple, small, single-path-ish shape that fits that hand-drawn minimal register.

## The problem to solve

Home currently lists **agents**. A pane running a plain shell — no agent, just a terminal a human typed into — is filtered out entirely. So a workspace whose panes are all plain shells is **completely invisible on Home**, including, in a real captured example, the very workspace the operator was sitting in at that moment.

We are fixing that: shells will be listed too. But a shell must not borrow the agent status scale. A shell has no status — nobody is working, nothing is blocked, nothing will finish. Showing it as `IDLE` would be a lie about a real state, and `UNKNOWN` is worse because that word is reserved for "we didn't understand what the desktop said". **A shell is the absence of an agent, not a state of one.** It also has nothing to monitor, so a card shaped like a status card sets the wrong expectation before the user has read a word.

## What to design

### 1. The shell card

A card that reads as *the same family, different species* as the agent card. It must:

- be **instantly distinguishable from an agent card at a glance** — before reading any text, while scrolling — so a user scanning for "is anything blocked?" never pauses on a shell;
- carry **no status pill and no status colour**. None of amber/red/green/grey-as-status may appear on it;
- be **quieter than an agent card but not disabled-looking**. A shell is fully functional and tappable — it opens the terminal exactly like an agent card does. It is lower priority, not degraded;
- work when a shell card sits **directly between two agent cards** in the same list, and also when the whole list is shells;
- keep the same tap-target generosity and press feedback.

The data a shell actually has, and nothing more:
- the working folder path (may be long; today's cards wrap to 2 lines)
- optionally a tab label
- optionally a pane title
- that's it — no agent name, no kind, no status

Note this kills the watermark as-is: the watermark letter is derived from the agent kind, and a shell has no kind. Decide what replaces it — a `$` or `>` glyph, a terminal-ish mark, nothing at all — and say why.

Give me at least **two distinct directions** for this card, differing in *how* they signal shell-ness, not just in colour. Suggested axes to explore: surface treatment (flat vs elevated, outlined vs filled), density (shorter card, single line), the left/right mark, and typography (a shell is arguably a monospace-first object). Show each direction in a list alongside real agent cards, not in isolation.

### 2. The workspace rollup badge when a workspace has no agents

The section header today always shows a rollup status pill. A workspace containing only shells has nothing to roll up. Design what that header shows instead — the pill's slot cannot simply hold `UNKNOWN`, and an empty gap may read as a loading failure. Consider a count (`3 shells`), a neutral typographic label, or removing the pill entirely and rebalancing the header.

### 3. Two workspaces with the same name

Real case: two different workspaces are both labelled `forgent` and both point at the same folder. Today only one is visible, so nobody notices; once shells are listed, both appear and become indistinguishable. There is a numeric workspace identifier available as a disambiguator. Design how a section header disambiguates without adding noise to the common case where labels are unique — ideally the disambiguator appears **only** when there is a collision.

## Constraints

- Dark theme only. No light variant needed.
- Do not introduce a new saturated hue that could be mistaken for a status. The palette is near-black slate + one blue accent + four reserved status colours; a shell should live in the neutrals.
- No icon library — any glyph must be drawable as simple inline SVG or be a plain text character.
- Phone-first, single column, ~360–430px wide is the real design target; 640px is the hard cap.
- Everything must degrade gracefully with `prefers-reduced-motion`.
- Accessibility: the shell/agent distinction must not rest on colour alone; the card is a `<button>` and needs a sensible accessible name; contrast must hold at the "quieter" end of the treatment.
- **Reserve the bottom-right corner.** A floating action button for "create new session" is planned for this screen shortly. Don't put anything load-bearing there, and don't let the last card in a long list sit under where it will land.

## Deliverables

1. The two-plus shell card directions, each rendered in a realistic mixed list (working agent, blocked agent, shell, done agent) at phone width.
2. Your recommended direction, with the reasoning — specifically why it survives a fast scroll.
3. The workspace header variants: normal, shells-only, and duplicate-label.
4. The empty-ish edge cases: a list that is entirely shells; a single shell alone.
5. Exact token values for anything new (colours, sizes, spacing), expressed in the existing scales above so it drops into the stylesheet without invention.

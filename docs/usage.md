# Using herdr-gateway

A day-to-day guide to the web UI. For install and configuration, see the [README](../README.md).

## 1. Logging in

Open the gateway's address (your Tailscale IP, e.g. `http://100.x.y.z:8787`, or `http://127.0.0.1:8787` locally). You'll see a single token field.

- Enter the token you set in `HERDCTL_WEB_SECRET` (or `demo` in `--demo` mode) and tap **Connect**.
- On success the browser holds an httpOnly session cookie for a week; returning later drops you straight onto the switcher.
- A wrong token gives a generic "access denied" — by design the gateway never confirms whether the token, the route, or the app itself is what you think. That silence is the security posture, not a bug.

## 2. The switcher

The switcher is the home screen: a scannable, portrait-friendly list of every agent across your herdr workspaces, read fresh from herdr each time.

Each card shows the path `workspace › tab › agent` and a **status badge**:

| Badge | Meaning |
|---|---|
| 🟡 **working** (pulsing) | the agent is actively doing something |
| 🔴 **blocked** | the agent is waiting on you — a question, a permission, a choice |
| 🟢 **done** | the agent finished its turn |
| ⚪ **idle** | nothing running (note: herdr also reports `idle` for some first-run prompts) |

The small dot in the header is herdr's health — green means the gateway can reach herdr. Tap **refresh** to re-read the list; tap **log out** to drop your session.

Tap any agent card to open its terminal.

## 3. The screen, typing, and keys

Tapping an agent opens its **screen** — the agent's current terminal, rendered with xterm.js. herdr's runtime keeps terminals at their desktop width (often 200+ columns), and its API has no way to resize one to a phone. So instead of cramming a wide terminal onto a phone, the gateway shows you the real screen and lets you **zoom and pan** it like a document:

- **Pan** by scrolling (drag / two-finger) inside the screen area.
- **Zoom** with the **A−/A+** buttons in the top bar (or pinch-zoom the page).
- The screen **refreshes every ~1.5s** — the small "Live" indicator confirms it's polling. It's a read-only view; you never accidentally type into the agent by scrolling.

The bottom bar has two actions: **Type** (send text) and **Keys** (tap-to-press keys). Opening either one slides a panel up from the bottom and **scrolls the screen up so its last lines stay visible above the panel** — the agent's question is usually at the very bottom, so you can still read it while you answer. A one-tap switch inside each panel jumps to the other, so "navigate the menu, then type your answer" needs no closing and reopening.

### Typing a reply

When the agent needs an answer (it's **blocked**, or you just want to send an instruction), tap **Type**:

1. A textarea slides up — type comfortably, paste, use autocorrect.
2. **Press Enter (submit)** is checked by default, so **Send** submits the reply (sends the text, then Enter). Uncheck it to insert text *without* submitting — useful for composing a multi-line answer, or for filling a field you'll submit some other way.
3. Tap **Send**. The text goes into the agent's composer exactly as if you'd typed it; the screen refreshes to show it landed.

You decide when to reply by looking at the screen — the gateway does not guess whether the agent is ready.

### Driving a menu with the Keys pad

Many agents ask with an arrow-key menu ("▸ Yes / No", a list to scroll) rather than a free-text prompt. Tap **Keys** to open the key pad and press keys one tap at a time:

- **↑ / ↓** move the selection, **Enter** confirms — these are the big buttons, since most menus are vertical.
- **← / →**, **␣ Space** (toggle a checkbox), and **Esc** (cancel / back) are one tap away for the menus that need them.
- Every press re-reads the screen, so you see the menu react before the next tap. The pad stays open across presses — navigate, then confirm.
- When a menu ends in a free-text field, tap the **⌨ Type** switch to jump straight to the reply panel without losing your place.

**Back** returns to the switcher without disturbing the agent — it keeps running in herdr regardless of your connection. If the screen can't be reached (pane closed, network blip), the indicator shows "Disconnected"; going back and reopening from a fresh switcher list recovers it.

## 4. Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| Login always denied | `HERDCTL_WEB_SECRET` unset or mismatched | Set it in `~/.config/herdr-gateway/herdctl.env`, then `systemctl --user restart herdr-gateway` |
| Health dot is red | herdr not running / gateway can't reach it | Check `journalctl --user -u herdr-gateway -f`; the supervisor retries automatically |
| Switcher is empty | herdr has no agents in the configured `--session` | Confirm `herdr_session` in the config matches the session your agents run in |
| Terminal opens but stays blank | pane id went stale (agent closed) | Go back and reopen from a fresh switcher list |
| Can't reach the page from your phone | not on the tailnet, or bound to loopback | Bind `bind_addr` to your Tailscale IP and join the same tailnet |
| "protocol mismatch" in logs | herdr version changed its wire protocol | The gateway pins an exact protocol number; upgrade the gateway to match herdr |

## 5. What this gateway does not do

- It does not manage herdr's sessions, worktrees, or agent lifecycle — herdr owns all of that.
- It does not sandbox agents. An agent's terminal can run anything your account can; the gateway only controls *which paths* it hands to herdr, not what the agent does afterward.
- It is not meant for the public internet. Keep it on your tailnet or behind a personal tunnel.

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

## 3. The screen + reply

Tapping an agent opens its **screen** — the agent's current terminal, rendered with xterm.js. herdr's runtime keeps terminals at their desktop width (often 200+ columns), and its API has no way to resize one to a phone. So instead of cramming a wide terminal onto a phone, the gateway shows you the real screen and lets you **zoom and pan** it like a document:

- **Pan** by scrolling (drag / two-finger) inside the screen area.
- **Zoom** with the **A−/A+** buttons in the top bar (or pinch-zoom the page).
- The screen **refreshes every ~1.5s** — the small "Live" indicator confirms it's polling. It's a read-only view; you never accidentally type into the agent by scrolling.

### Replying

When the agent needs an answer (it's **blocked**, or you just want to send an instruction), tap **Reply**:

1. A textarea slides up — type comfortably, paste, use autocorrect.
2. Leave **Press Enter (submit)** checked to submit the reply (send the text, then Enter); uncheck it to insert text without submitting (for multi-line composing).
3. Tap **Send**. The text goes into the agent's composer exactly as if you'd typed it; the screen refreshes to show it landed.

You decide when to reply by looking at the screen — the gateway does not guess whether the agent is ready.

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

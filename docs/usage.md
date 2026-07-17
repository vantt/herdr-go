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

## 3. The terminal

Tapping an agent opens a full-screen terminal rendered with xterm.js. **Rotate your phone to landscape** for a full-width view — a wide terminal is far easier to read.

- **You are typing live into the real agent.** Whatever you type goes straight to the agent as if you were at the keyboard: answer its questions, approve or decline, type instructions. Arrow keys, Enter, Ctrl-C, and Tab all pass through.
- The gateway is a transparent pipe here — it does not interpret or filter what you see, so you get the agent's screen exactly as it is.
- **Control vs. observe:** by default you open in *control* mode (you can type). herdr allows only one writer per terminal, so opening the same agent as a controller elsewhere will hand the write role over. Multiple *observers* (read-only) can watch at once.
- **Back** returns to the switcher without closing the agent — the agent keeps running in herdr regardless of your connection.

### Rotation

The terminal resizes with your phone:

- While **watching** (observe), rotating just re-fits your own view — nothing changes for anyone else.
- While **typing** (control), rotating reflows the real terminal to your phone's dimensions, so what you see is what the agent's program sees.

### If the connection drops

Mobile networks hiccup. If the terminal disconnects, you'll see a reconnect affordance — reconnecting redraws the full screen from scratch, so you never see a half-torn view. The agent itself is untouched by the drop; it lives in herdr, not in your browser.

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

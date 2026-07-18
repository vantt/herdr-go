# Deployment & use cases

How to run herdr-gateway for different situations — from a 30-second local demo to a self-healing service reachable from your phone. Each scenario says **when to use it**, the **exact steps**, and the **security posture**.

First, the one rule that shapes everything below:

> **herdr's socket has no authentication.** Anyone who can reach the gateway's port can drive your agents. So the gateway's own login token — and *where you let the port be reached from* — is the entire security boundary. The scenarios are ordered from most private to most exposed; pick the least-exposed one that works for you.

The listen address is set two ways:

- Config file: `"bind_addr": "127.0.0.1:8787"`.
- `--bind <addr>` on the command line, which **overrides** the config for that run.

Binding to anything other than `127.0.0.1` (loopback) makes the gateway reachable from other machines and prints a warning at startup.

---

## 1. Quick demo on this machine (no herdr, no risk)

**When:** you just want to see the UI and click around. Nothing is exposed; no herdr needed.

```bash
cd web && npm install && npm run bundle && cd ..
cargo run -- --demo
```

Open <http://127.0.0.1:8787>, log in with the token **`demo`**. Four fake agents appear — one per status — that you can open and type into. Only herdr is faked; the terminal relay is the real one.

**Security:** bound to loopback, only this machine can reach it. The `demo` token is fine here and only here.

---

## 2. Demo from your phone on the same Wi-Fi

**When:** you want to *feel* the mobile UI on a real phone, still without herdr.

```bash
cd web && npm run bundle && cd ..
cargo run -- --demo --bind 0.0.0.0:8787
```

The startup log prints your reachable address; find this machine's LAN IP with `hostname -I` (e.g. `192.168.20.243`). On your phone (same Wi-Fi), open `http://192.168.20.243:8787` and log in with `demo`.

**Security:** now reachable by **everyone on that Wi-Fi**. Fine for a quick look on a home network; the `demo` token is weak, so don't leave it running. For anything real, use scenario 4 or 5.

---

## 3. Real gateway, private to you (loopback + SSH tunnel)

**When:** you run real herdr on a server/desktop and want to reach it from your phone **without opening any port**, over SSH you already trust.

Config (`~/.config/herdr-gateway/config.json`) keeps the default loopback bind:

```json
{
  "bind_addr": "127.0.0.1:8787",
  "herdr_session": "gateway",
  "allowed_roots": ["/home/you/projects"]
}
```

Set a real token: `HERDCTL_WEB_SECRET=$(openssl rand -hex 24)` in `~/.config/herdr-gateway/herdctl.env`. Start it (scenario 7 makes this a service). Then from your phone's SSH app forward the port:

```bash
ssh -L 8787:127.0.0.1:8787 you@your-server
```

Open <http://127.0.0.1:8787> on the phone — the tunnel carries it.

**Security:** strongest option. The port is never exposed; SSH is the transport and auth; the web token is a second gate. No TLS needed (SSH encrypts).

---

## 4. Real gateway over Tailscale (recommended)

**When:** you want to open the app on your phone anywhere, with no manual tunnel, and no port on the public internet. This is the intended setup.

Install Tailscale on both the server and the phone, join the same tailnet. Find the server's tailnet IP (`tailscale ip -4`, e.g. `100.101.102.103`) and bind to it:

```json
{ "bind_addr": "100.101.102.103:8787", "herdr_session": "gateway", "allowed_roots": ["/home/you/projects"] }
```

(or run with `--bind 100.101.102.103:8787`). Open `http://100.101.102.103:8787` from the phone while on the tailnet.

**Security:** the tailnet is a private WireGuard network — only your devices can reach the address, and traffic is already encrypted. The web token remains the app-level gate. No public exposure, no reverse proxy required. If you want HTTPS in the browser too, enable [Tailscale Serve](https://tailscale.com/kb/1312/serve).

---

## 5. Real gateway on the LAN (`0.0.0.0`)

**When:** you deliberately want it reachable by any device on a **trusted** local network and aren't using Tailscale.

```json
{ "bind_addr": "0.0.0.0:8787", "herdr_session": "gateway", "allowed_roots": ["/home/you/projects"] }
```

or `--bind 0.0.0.0:8787`. The gateway prints a warning because this listens on every interface.

**Security — read this:**

- The web login token is now the **only** boundary. Use a long random one (`openssl rand -hex 24`), never `demo`.
- Traffic is **plain HTTP** — anyone on the LAN can sniff the token and the terminal contents. Put TLS in front (scenario 6) if the LAN is shared (office, café, dorm).
- Restrict with a firewall if you can, e.g. only your phone's IP: `sudo ufw allow from 192.168.20.50 to any port 8787`.
- Never port-forward this straight to the internet. For remote access use Tailscale (4) or a proxy with TLS (6).

---

## 6. Behind a reverse proxy with TLS

**When:** the gateway must be reached over a shared LAN or the internet and you want HTTPS + a real certificate. Keep herdctl on loopback and let the proxy terminate TLS.

Bind loopback: `"bind_addr": "127.0.0.1:8787"`. Then, with **Caddy** (automatic certificates):

```caddyfile
gateway.example.com {
    reverse_proxy 127.0.0.1:8787
}
```

Caddy handles WebSockets (`/ws/terminal`) transparently. With **nginx**, add the upgrade headers so the terminal WebSocket works:

```nginx
location / {
    proxy_pass http://127.0.0.1:8787;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_set_header Host $host;
    proxy_read_timeout 3600s;   # keep long-lived terminal sockets open
}
```

**Security:** TLS protects the token and terminal stream; the proxy is the only exposed port. Still use a strong web token. Consider adding the proxy's own auth (basic auth / mTLS / an SSO forward-auth) as a second layer for internet exposure.

---

## 7. As a self-healing systemd service

**When:** you want it always running, restarting on crash and after reboot — the normal production setup.

```bash
./install.sh
```

This installs a prebuilt herdctl binary from the latest GitHub release when one matches your platform (no Rust/Node toolchain needed), falling back to compiling herdctl + the web UI from source otherwise. Either way it installs the binary under `~/.local`, writes a starter config and a mode-600 secrets file under `~/.config/herdr-gateway/`, and installs a **systemd user unit** (`Restart=always`, lingering enabled so it survives logout/reboot). Then:

```bash
$EDITOR ~/.config/herdr-gateway/herdctl.env    # set HERDCTL_WEB_SECRET
$EDITOR ~/.config/herdr-gateway/config.json    # bind_addr, herdr_session, allowed_roots
systemctl --user start herdr-gateway
journalctl --user -u herdr-gateway -f
```

The gateway supervises herdr (relaunches it when down); systemd supervises the gateway. To change the bind address, edit `bind_addr` in the config and `systemctl --user restart herdr-gateway`.

---

## 8. Pointing at a real herdr

**When:** moving from `--demo` to the real thing.

Drop `--demo`. In the config, set `herdr_session` to the **exact** herdr `--session` name your agents run under (the gateway prepends `--session` to every herdr call), and `allowed_roots` to the absolute directories agents may be pointed at (empty = refused, by design):

```json
{
  "bind_addr": "100.101.102.103:8787",
  "herdr_session": "gateway",
  "allowed_roots": ["/home/you/projects", "/home/you/work"],
  "poll_interval_ms": 500,
  "herdr_protocol": 16
}
```

The switcher then lists your real agents; the health dot goes green when the gateway can reach herdr. If the log says `protocol mismatch`, herdr changed its wire protocol number — upgrade the gateway to match (the pin is exact by design).

---

## 9. Turning on Telegram notifications

**When:** you want a phone ping when an agent becomes `blocked` (needs you) or `done`.

1. Create a bot with [@BotFather](https://t.me/botfather), get its token.
2. Get the chat id you want messages in (message the bot, then read `getUpdates`, or use a chat-id bot).
3. Put the token in the environment (never the config): `HERDCTL_TELEGRAM_TOKEN=...` in `herdctl.env`.
4. Put the chat id in the config: `"telegram_chat_id": "123456789"`.
5. Restart. On a `blocked`/`done` transition you'll get a message; delivery is at-least-once and the text is passed through the redactor first.

Notifications run off the 500 ms status poll — they work without any of the event-subscription work, so they're reliable today.

---

## Quick reference

| Goal | Bind | Transport | Token strength |
|---|---|---|---|
| Look at the UI locally | `127.0.0.1` (`--demo`) | none | `demo` ok |
| Try it on your phone quickly | `0.0.0.0` (`--demo --bind`) | LAN, plain HTTP | keep it brief |
| Private remote access | `127.0.0.1` | SSH tunnel | strong |
| Anywhere, private (recommended) | tailnet IP | Tailscale (WireGuard) | strong |
| Trusted LAN | `0.0.0.0` | plain HTTP (add firewall) | strong |
| Shared LAN / internet | `127.0.0.1` | reverse proxy + TLS | strong + proxy auth |

See [usage.md](usage.md) for the day-to-day flow once you're in, and the [README](../README.md) for the overview.

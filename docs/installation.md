# Installation guide

From nothing to the app open on your phone. This is the single "how do I set it up" page; for the finer-grained hosting scenarios (SSH tunnel, reverse proxy + TLS, LAN firewalling) see [deployment.md](deployment.md).

## 0. Prerequisites

- **Rust** (stable) and **Node.js 20+** with npm — only needed to *build* herdctl.
- **herdr** installed and a server running. herdr owns the agents; the gateway is a client of it. Check with:
  ```bash
  herdr status server        # should say: status: running
  herdr session list --json  # note your session name (often "default")
  ```
  If no server is running, start one: `herdr --session default server`.
- *(Optional)* **Tailscale** on this machine and your phone, if you want to reach the gateway from anywhere without opening a LAN port.

## 1. Get the code

```bash
git clone https://github.com/vantt/herdr-gateway
cd herdr-gateway
```

## 2. Choose how to run it

### Option A — Dev-as-live (recommended while iterating)

One command builds everything and runs **this build** as a self-healing systemd user service:

```bash
./dev-deploy.sh
```

Each time you re-run it, the fresh build becomes the live instance — no copying, no reinstalling. On first run herdctl auto-creates a working config and a login token under `~/.config/herdr-gateway/` (the token is printed and saved to `herdctl.env`, mode 600).

### Option B — Just run it

No service, no arguments — herdctl auto-configures itself and runs against the local herdr:

```bash
cargo build --release
./target/release/herdctl        # prints the login token + URL, serves on 127.0.0.1:8787
```

### Option C — Production install

A one-time system install (prod systemd unit, prompts you for a token):

```bash
./install.sh
$EDITOR ~/.config/herdr-gateway/herdctl.env   # set HERDCTL_WEB_SECRET
systemctl --user start herdr-gateway
```

## 3. Check your setup with `doctor`

Before (or after) anything, run the built-in diagnosis:

```bash
herdctl doctor          # or: ./target/release/herdctl doctor
```

It prints a ✓/✗ line per check — herdr binary, config, socket, reachability + protocol, web token, allowed roots, built web UI, bind address, service state — with a one-line fix for anything wrong, and exits non-zero if something blocking is missing. A clean run looks like:

```
  ✓ herdr binary     herdr 0.7.4
  ✓ herdr reachable  protocol 16 (v0.7.4)
  ✓ web token        set
  ✓ web UI           static
  ✓ bind address     127.0.0.1:8787 — local only
  All good — you're ready to run herdctl.
```

## 4. Open it

Get your login token:

```bash
grep HERDCTL_WEB_SECRET ~/.config/herdr-gateway/herdctl.env
```

Open `http://127.0.0.1:8787` on this machine and log in with that token. You should see your agents in the switcher; tap one to see its screen and reply.

## 5. Reach it from your phone

By default the gateway binds `127.0.0.1` (this machine only). To open it on your phone, edit the bind address:

```bash
$EDITOR ~/.config/herdr-gateway/config.json
#   "bind_addr": "100.x.y.z:8787"   ← your Tailscale IP (recommended), or
#   "bind_addr": "0.0.0.0:8787"     ← any device on your Wi-Fi (trusted LAN only)
systemctl --user restart herdr-gateway-dev    # (or herdr-gateway for a prod install)
```

Then open that address from your phone (on the same tailnet / Wi-Fi) and log in with the same token. Binding beyond loopback prints a security reminder — the login token is then your only gate, so keep it on Tailscale or behind TLS (see [deployment.md](deployment.md)).

## Troubleshooting

Run `herdctl doctor` first — it names most problems and their fix. Common ones:

| doctor says | Fix |
|---|---|
| ✗ herdr socket … does not exist | Start herdr: `herdr --session default server` |
| ✗ herdr reachable: protocol mismatch | Upgrade herdctl (or herdr) so the wire protocol numbers match |
| ✗ web UI: no built UI | `cd web && npm install && npm run bundle` (or re-run `./dev-deploy.sh`) |
| ✗ web token | Run herdctl once to auto-generate one, or set `HERDCTL_WEB_SECRET` |
| Can't reach it from the phone | Bind a tailnet/LAN address (step 5) and join the same network |

See [usage.md](usage.md) for the day-to-day flow once you're in.

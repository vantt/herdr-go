# herdr-gateway (`herdctl`)

A **web-first remote gateway + supervisor** for [herdr](https://github.com/ogulcancelik/herdr): from your phone, watch and type directly into the AI coding agents (Claude Code / Codex) running inside herdr — and keep herdr alive automatically.

herdr deliberately isn't a web dashboard or a mobile app; it exposes a JSON/socket runtime and says "use SSH + the TUI". `herdr-gateway` fills exactly that gap: a mobile-first web terminal over that runtime, plus a watchdog that restarts herdr when it dies. It does **not** re-implement herdr's session/agent lifecycle — herdr stays the single source of truth.

> **Status:** M1 — the web terminal axis + supervisor foundation. Runnable and fully testable without a live herdr (a built-in fake). Telegram notify/provisioning are later slices.

## What you get

- **Mobile-first web UI** — a portrait agent switcher with live status badges (working / blocked / done / idle / unknown), tap an agent to see its screen in a **zoom/pan** view (poll-refreshed), then tap **Type** to send a message into the agent via a textarea, or **Keys** to drive its menu with on-screen arrow / Enter / Esc keys. (herdr's request API can't size a PTY to a phone, so the phone observes the screen and replies rather than driving a live terminal — see `docs/DISCOVERY.md`.)
- **Supervisor** — one systemd user unit runs the gateway; the gateway health-checks herdr and relaunches it when it's down. Self-heals across crashes and reboots.
- **A single security boundary** — herdr's socket has no auth; the gateway is the one gate in front of it (fail-closed token auth, silent to the unauthenticated, bound to your tailnet).

## Architecture in one breath

`systemd → herdctl (supervisor + web) → herdr → coding agents`. The Rust backend (tokio + axum) is hexagonal only at the seams that have two real implementations — the herdr port (`HerdrControl` for the control plane, `HerdrStream` for the terminal), the event source, and the store. The Tier 2 terminal relay is a **transparent pipe** between xterm.js and herdr — it deliberately never touches the control plane. The frontend is TypeScript + xterm.js. See `docs/PRD.md` for the full design and `docs/DISCOVERY.md` for the live-verified herdr behavior the design rests on.

## Try it in 30 seconds (no herdr needed)

```bash
# 1. bundle the web UI
cd web && npm install && npm run bundle && cd ..
# 2. run against an in-memory fake herdr
cargo run -- --demo
```

Open <http://127.0.0.1:8787>, log in with the token **`demo`**, and you'll see four fake agents — one in each status — that you can open and type into. This is the real relay path end to end; only herdr is faked.

## Install

**→ Full step-by-step: [docs/installation.md](docs/installation.md)** (prerequisites, the three ways to run it, `herdctl doctor`, phone access, troubleshooting). Quick paths below.

Run **`herdctl doctor`** any time to diagnose the setup — it checks herdr, the socket, protocol, config, token, the built UI, and the bind address, and prints a one-line fix for anything wrong.

### systemd user service

```bash
./install.sh
```

This compiles `herdctl` and the web UI, installs them under `~/.local`, writes a starter config and a mode-600 secrets file under `~/.config/herdr-gateway/`, and installs a self-healing `herdr-gateway.service` (systemd *user* unit, `Restart=always`, lingering enabled so it survives reboot). Then:

```bash
# 1. set a login token
$EDITOR ~/.config/herdr-gateway/herdctl.env      # HERDCTL_WEB_SECRET=$(openssl rand -hex 24)
# 2. review the config (bind address, allowed roots, herdr session)
$EDITOR ~/.config/herdr-gateway/config.json
# 3. start + watch
systemctl --user start herdr-gateway
journalctl --user -u herdr-gateway -f
```

## Reaching it from your phone

Set the listen address either in the config (`bind_addr`) or with `--bind <addr>` on the command line (which overrides the config for that run):

- **Recommended — Tailscale:** bind your tailnet IP (`--bind 100.x.y.z:8787`) and open it from the phone on the same tailnet. Private WireGuard network, encrypted, no public exposure.
- **Quick LAN look:** `herdctl --demo --bind 0.0.0.0:8787`, then open `http://<this-machine-LAN-IP>:8787` from the phone on the same Wi-Fi.
- **Private, no open port:** keep the default loopback bind and SSH-forward it (`ssh -L 8787:127.0.0.1:8787 …`).

The **default bind is `0.0.0.0:8787`** (all interfaces) so it's reachable across machines out of the box. Binding beyond loopback prints a startup warning: herdr has no auth of its own, so the web token becomes the only boundary. It is **not** meant to face the public internet — put TLS (a reverse proxy) in front, or bind a Tailscale address, for anything beyond a trusted network. To restrict to this machine only, set `bind_addr` to `127.0.0.1:8787`. Auth stays fail-closed regardless.

**→ Full step-by-step for every situation (demo, tailnet, LAN, reverse proxy + TLS, systemd, real herdr, Telegram): [docs/deployment.md](docs/deployment.md).**

## Configuration

| Key | Meaning | Default |
|---|---|---|
| `bind_addr` | Address to listen on (loopback, or your tailnet IP) | `127.0.0.1:8787` |
| `herdr_session` | The explicit herdr `--session` name the gateway owns | — (required) |
| `allowed_roots` | Absolute roots agents may be pointed at (empty = refused) | — (required) |
| `poll_interval_ms` | Status poll cadence | `500` |
| `herdr_protocol` | Pinned herdr wire protocol (exact match) | `16` |
| `static_dir` | Web UI assets directory | `static` |

Secrets are **never** config keys — they come from the environment only:

| Env var | Purpose |
|---|---|
| `HERDCTL_WEB_SECRET` | Web login token (required unless `--demo`) |
| `HERDCTL_GITHUB_TOKEN` | GitHub token for provisioning (later slice) |
| `HERDCTL_TELEGRAM_TOKEN` | Telegram bot token for notify (later slice) |

## Development

```bash
cargo test          # backend unit + integration (incl. the observe/reply e2e)
cargo clippy -- -D warnings
cd web && npm test  # frontend unit tests
cd web && npm run dev   # vite dev server, proxying /api to a running herdctl
```

Everything is tested against the fake herdr, so the whole app runs green with no herdr installed.

### Run this build as the live instance (dev = real)

```bash
./dev-deploy.sh          # build + bundle, then (re)start a systemd user service
                         # that runs THIS repo's build output as the live instance
./dev-deploy.sh --logs   # ... and follow the logs
```

Each run makes the fresh build live — no copy, no reinstall. On first run `herdctl` auto-creates a working config and a persistent login token under `~/.config/herdr-gateway/` (the token is printed and saved to `herdctl.env`, mode 600). A plain `herdctl` with **no arguments** does the same thing manually: auto-config, auto-token, run against the local herdr. To reach it from your phone, edit `bind_addr` in `~/.config/herdr-gateway/config.json` (e.g. your Tailscale IP) and `systemctl --user restart herdr-gateway-dev`.

## Usage guide

See **[docs/usage.md](docs/usage.md)** for the day-to-day flow: logging in, reading the switcher, driving a terminal, rotating your phone, and troubleshooting.

## License

MIT — see [LICENSE](LICENSE).

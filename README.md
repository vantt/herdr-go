# herdr-gateway (`herdctl`)

A **web-first remote gateway + supervisor** for [herdr](https://github.com/ogulcancelik/herdr): from your phone, watch and type directly into the AI coding agents (Claude Code / Codex) running inside herdr — and keep herdr alive automatically.

herdr deliberately isn't a web dashboard or a mobile app; it exposes a JSON/socket runtime and says "use SSH + the TUI". `herdr-gateway` fills exactly that gap: a mobile-first web terminal over that runtime, plus a watchdog that restarts herdr when it dies. It does **not** re-implement herdr's session/agent lifecycle — herdr stays the single source of truth.

> **Status:** M1 — the web terminal axis + supervisor foundation. Runnable and fully testable without a live herdr (a built-in fake). Telegram notify/provisioning are later slices.

## What you get

- **Mobile-first web UI** — a portrait agent switcher with live status badges (working / blocked / done / idle), tap an agent to drop into a full-screen landscape terminal and type live, full-fidelity, via xterm.js.
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

## Install (systemd user service)

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

Binding beyond loopback (`0.0.0.0`, a LAN IP) prints a startup warning: herdr has no auth of its own, so the web token becomes the only boundary. It is **not** meant to face the public internet — put TLS (a reverse proxy) in front for anything beyond a trusted network. Auth stays fail-closed regardless.

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
cargo test          # backend unit + integration (incl. the e2e relay over a real WebSocket)
cargo clippy -- -D warnings
cd web && npm test  # frontend unit tests
cd web && npm run dev   # vite dev server, proxying /api and /ws to a running herdctl
```

Everything is tested against the fake herdr, so the whole app — including the terminal relay — runs green with no herdr installed.

## Usage guide

See **[docs/usage.md](docs/usage.md)** for the day-to-day flow: logging in, reading the switcher, driving a terminal, rotating your phone, and troubleshooting.

## License

MIT — see [LICENSE](LICENSE).

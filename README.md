# Herdr Go

**Check on your coding agents from your phone — without SSH, without a laptop, without waiting.**

Your agents run for hours. You don't want to sit at a desk for hours. Herdr Go puts a live terminal into your pocket: see who's working, who's stuck, who's done — and reply right from the couch, the commute, or bed.

<!--
TODO(README imagery): capture 2 real screenshots from a running demo instance
(`herdr-go --demo`, phone-width viewport ~390px) and drop them here as
docs/assets/screenshot-agent-list.png and docs/assets/screenshot-terminal.png:
  1. The agent list screen (shows working/blocked/done/idle status at a glance)
  2. An open terminal with the Keys/Type reply bar visible
Reference them with standard markdown image syntax once captured - this note
is a placeholder, not a claim that images exist yet.
-->

## Why people keep this running

- **See at a glance.** Every agent's state — working, blocked, done, idle — in one list, scannable in two seconds.
- **Reply from anywhere.** Tap in, read the real terminal, type back. Full fidelity, not a summary.
- **Never babysit it.** If herdr goes down, Herdr Go brings it back. One less thing to remember.
- **Locked down by default.** One token gates everything. Nothing is exposed until you say so.

## Try it in 30 seconds — no install, no account

```bash
git clone https://github.com/vantt/herdr-go && cd herdr-go
cargo build --release && ./target/release/herdr-go --demo
```

Open <http://127.0.0.1:8787>, sign in with `demo`, and click around a sample agent list. This is the whole app, fed by fake data — nothing to configure, nothing to break.

Demo mode listens on loopback by default. To expose it intentionally, pass an explicit address, for example `herdr-go --demo --bind 0.0.0.0:8787`, and secure the network around it.

## Run it for real — one command, then forget about it

```bash
curl -fSL https://raw.githubusercontent.com/vantt/herdr-go/main/install.sh | bash
```

On macOS it's live immediately. On Linux, start it once:

```bash
systemctl --user start herdr-go.service
```

Either way, it's now a background service that survives reboots and restarts itself if it crashes, and it printed a login token the first time — you'll need that to sign in from your phone. Works on Linux (systemd) and macOS (launchd) today; Windows is on the way.

Open `http://<your-machine>:8787` from your phone on the same network (or your tailnet) and sign in.

**Full install details, upgrading, and uninstalling:** [docs/installation.md](docs/installation.md)

## What you're actually looking at

Herdr Go is a small, self-healing gateway that sits in front of [herdr](https://github.com/ogulcancelik/herdr) — the terminal multiplexer your coding agents already run inside — and gives it a mobile-first face. It doesn't manage your agents (herdr already does that perfectly); it just lets you *see* and *talk to* them from a phone.

Tap an agent, and its terminal opens full-screen, landscape, live — the same fidelity you'd get SSH'd in on a laptop, just in your hand.

## Configure it your way

The important settings — where it binds, what workspaces it can touch, how it protects your login token — are one small JSON file plus one secrets file, both created for you on first run with sane, safe defaults.

**Full settings reference, deployment patterns (LAN, tailnet, reverse proxy), and troubleshooting:** [docs/advanced/](docs/advanced/)

## Building from source / contributing

```bash
git clone https://github.com/vantt/herdr-go && cd herdr-go
cd web && npm ci && npm run bundle && cd ..
cargo build --release
```

**Full contributor workflow:** [docs/advanced/source-build.md](docs/advanced/source-build.md)

## Project docs

- [Installation details](docs/installation.md)
- [Advanced configuration, deployment, troubleshooting](docs/advanced/)
- [Product backlog](docs/backlog.md)
- [System overview](docs/specs/system-overview.md)
- [Discovery notes](docs/DISCOVERY.md)

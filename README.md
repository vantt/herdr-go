# Herdr Go

Your coding agents run for hours while you're away from your desk. Herdr Go puts them in your pocket: a mobile-first gateway in front of [herdr](https://github.com/ogulcancelik/herdr) that shows you who's working, who's stuck, who's done, and lets you read and reply to a real terminal — all from your phone, without SSH, without a laptop. If herdr goes down, Herdr Go brings it back on its own.

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

## Usage

Install with one command:

```bash
# macOS / Linux
curl -fSL https://raw.githubusercontent.com/vantt/herdr-go/main/install.sh | bash
```

```powershell
# Windows
irm https://raw.githubusercontent.com/vantt/herdr-go/main/install.ps1 | iex
```

On macOS and Windows it's live immediately. On Linux, start it once:

```bash
systemctl --user start herdr-go.service
```

Either way, it's now a background service that survives reboots and restarts itself if it crashes, and it printed a login token the first time — you'll need that to sign in from your phone. Open `http://<your-machine>:8787` from your phone on the same network (or your tailnet) and sign in.

**Full install details, upgrading, and uninstalling:** [docs/installation.md](docs/installation.md)

Want to try it first with no install and no account? `herdr-go --demo` runs the whole app against fake data on loopback. It stays local unless you pass an explicit address, for example `herdr-go --demo --bind 0.0.0.0:8787` — only do that once you mean to expose it.

### Check and configure with `doctor`

`herdr-go doctor` is the one command for checking and changing your setup:

```bash
herdr-go doctor
```

It runs every diagnostic check, offers an inline guided fix for anything it can fix (a missing config, an empty workspace-roots list, a missing login token), then asks once whether you want to edit any of the 8 config settings or the 3 secrets — no separate `config` command, no flags to remember. If a fix creates or replaces your login token, doctor offers to restart the running background service right away so your new token takes effect immediately; the manual **Restart** section below still applies when you change a setting or secret by hand or through the settings editor.

Want a read-only report instead (safe for scripts and CI)?

```bash
herdr-go doctor --check
```

### Restart

After changing a setting or rotating a secret by hand, restart the service:

```bash
# macOS
launchctl kickstart -k "gui/$(id -u)/io.github.vantt.herdr-go"
# Linux
systemctl --user restart herdr-go.service
# Windows
Stop-ScheduledTask -TaskName HerdrGo; Start-ScheduledTask -TaskName HerdrGo
```

## What you're actually looking at

Herdr Go is a small, self-healing gateway that sits in front of [herdr](https://github.com/ogulcancelik/herdr) — the terminal multiplexer your coding agents already run inside — and gives it a mobile-first face. It doesn't manage your agents (herdr already does that perfectly); it just lets you *see* and *talk to* them from a phone.

Tap an agent, and its terminal opens full-screen, landscape, live — the same fidelity you'd get SSH'd in on a laptop, just in your hand.

## Configure it your way

The important settings — where it binds, what workspaces it can touch, how it protects your login token — live in one small JSON file plus one secrets file, both created for you on first run with sane, safe defaults. Edit them by hand, or use `herdr-go doctor` above.

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

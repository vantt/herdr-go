# Installation

## Install

```bash
# macOS / Linux
curl -fSL https://raw.githubusercontent.com/vantt/herdr-go/main/install.sh | bash
```

```powershell
# Windows
irm https://raw.githubusercontent.com/vantt/herdr-go/main/install.ps1 | iex
```

Supports Linux (systemd, any account with a reachable user service manager), macOS (launchd, Apple Silicon), and Windows (per-user Scheduled Task, no administrator rights required). It downloads the matching published release, creates per-user config/data directories and a login token on first run, and registers a background service:

- **macOS** — a launchd `LaunchAgent` (`~/Library/LaunchAgents/io.github.vantt.herdr-go.plist`), loaded and started immediately.
- **Linux** — a systemd `--user` service (`~/.config/systemd/user/herdr-go.service`), enabled but not started — start it once with `systemctl --user start herdr-go.service`.
- **Windows** — a per-user, logon-triggered Scheduled Task named `HerdrGo`, started immediately.

Open `http://<your-machine>:8787` from a phone on the same trusted LAN or tailnet and sign in with the printed token.

Want to try the UI first, with no install and no account? Run `herdr-go --demo` (after a [source build](advanced/source-build.md)) and open <http://127.0.0.1:8787>, sign in with `demo`.

Intel Macs (`x86_64-apple-darwin`) have no published binary yet — the installer fails with a named error pointing you to a source build.

Intel Macs (`x86_64-apple-darwin`) have no published binary yet — the installer fails with a named error pointing you to a source build.

## Upgrade

Run the installer again — it preserves your existing config and token, and re-registers the service cleanly (no duplicate registration):

```bash
curl -fSL https://raw.githubusercontent.com/vantt/herdr-go/main/install.sh | bash
```

## Uninstall

```bash
curl -fSL https://raw.githubusercontent.com/vantt/herdr-go/main/install.sh | bash -s -- --uninstall
```

Removes the binary and the platform service registration (systemd unit or LaunchAgent). Your config, data, and login token are always left in place — delete them by hand if you want a truly clean slate:

```bash
# macOS
rm -rf "$HOME/Library/Application Support/herdr-go"
# Linux
rm -rf "${XDG_CONFIG_HOME:-$HOME/.config}/herdr-go" "${XDG_DATA_HOME:-$HOME/.local/share}/herdr-go"
```

## Login token

Only a first install creates and prints a login token. Repeat installs and migrations preserve the existing token and never print it into installer or service logs.

Retrieve it locally from the protected environment file:

```bash
# macOS
env_file="$HOME/Library/Application Support/herdr-go/herdr-go.env"
# Linux
env_file="${XDG_CONFIG_HOME:-$HOME/.config}/herdr-go/herdr-go.env"

sed -n 's/^HERDR_GO_WEB_SECRET=//p' "$env_file"
```

To rotate it, edit `HERDR_GO_WEB_SECRET` in that same file, keep it readable only by your user, and restart the service:

```bash
${EDITOR:-vi} "$env_file"
chmod 600 "$env_file"
# macOS
launchctl kickstart -k "gui/$(id -u)/io.github.vantt.herdr-go"
# Linux
systemctl --user restart herdr-go.service
```

## Where things live

| | macOS | Linux |
|---|---|---|
| Config + data | `~/Library/Application Support/herdr-go/` (one directory) | `${XDG_CONFIG_HOME:-~/.config}/herdr-go/` (config), `${XDG_DATA_HOME:-~/.local/share}/herdr-go/` (data) |
| Login token | `herdr-go.env` inside the config directory, mode `600` | same |
| Service definition | `~/Library/LaunchAgents/io.github.vantt.herdr-go.plist` | `~/.config/systemd/user/herdr-go.service` |

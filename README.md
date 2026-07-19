# Herdr Go

Keep an eye on your coding agents from your phone. Herdr Go gives you a
mobile-first view of agents running in
[herdr](https://github.com/ogulcancelik/herdr), lets you read their terminals
and reply, and restarts herdr when it goes down.

The executable is named `herdr-go` and the Rust crate is `herdr_go`.

## What you get

- See which agents are working, blocked, done, or idle.
- Open an agent's terminal, zoom in, and send text or keys from your phone.
- Keep the gateway and herdr healthy through a self-healing user service.
- Put one fail-closed web token in front of herdr's unauthenticated socket.

## Install

The one-command installer sets up `herdr-go`, creates per-user config and data
directories, installs the `herdr-go.service` systemd user service, and prints a
login token on first install:

```bash
curl -fSL https://raw.githubusercontent.com/vantt/herdr-go/main/install.sh | bash
systemctl --user start herdr-go.service
```

Open `http://<your-machine>:8787` from a phone on the same trusted LAN or
tailnet and sign in with the printed token.

The installer is for systemd-based Linux accounts with a reachable user service
manager. It supports the published `x86_64` and `arm64` Linux release archives,
preserves existing config and tokens on repeat installs, and migrates the old
`herdr-gateway` state directory only when the new `herdr-go` directory is not
already present.

Upgrade by running the installer again:

```bash
curl -fSL https://raw.githubusercontent.com/vantt/herdr-go/main/install.sh | bash
systemctl --user restart herdr-go.service
```

Uninstall the service and binary:

```bash
systemctl --user disable --now herdr-go.service
rm ~/.config/systemd/user/herdr-go.service ~/.local/bin/herdr-go
systemctl --user daemon-reload
```

Config and data remain in place until you deliberately remove them.

## Try the UI locally

```bash
herdr-go --demo
```

Open <http://127.0.0.1:8787> and sign in with `demo`.

Demo mode listens on loopback by default. To expose it intentionally, pass an
explicit address, for example `herdr-go --demo --bind 0.0.0.0:8787`, and secure
the network around it.

## Daily use

Open the gateway from your phone. The agent list shows working, blocked, done,
idle, and unknown states so you can scan before opening a terminal.

Tap an agent to open its terminal snapshot. Pinch or pan to inspect dense
output. Tap **Type** to send text, or **Keys** for Arrow, Enter, and Escape
controls. Confirm the selected agent before sending input.

Herdr remains the source of truth for agent and session lifecycle. Herdr Go is
the remote gateway and supervisor around it.

## Login token

Only a first install creates and prints a login token. Repeat installs and
migrations preserve the existing token and do not print it into installer or
service logs.

Retrieve it locally from the protected environment file:

```bash
env_file="${XDG_CONFIG_HOME:-$HOME/.config}/herdr-go/herdr-go.env"
sed -n 's/^HERDR_GO_WEB_SECRET=//p' "$env_file"
```

To rotate it, edit `HERDR_GO_WEB_SECRET`, keep the file readable only by your
user, and restart the service:

```bash
env_file="${XDG_CONFIG_HOME:-$HOME/.config}/herdr-go/herdr-go.env"
${EDITOR:-vi} "$env_file"
chmod 600 "$env_file"
systemctl --user restart herdr-go.service
```

## Configuration

The canonical config file is:

```text
${XDG_CONFIG_HOME:-$HOME/.config}/herdr-go/config.json
```

Durable SQLite data is stored under:

```text
${XDG_DATA_HOME:-$HOME/.local/share}/herdr-go
```

Secrets live in `herdr-go.env` beside the config file with mode `600`.

Common settings:

| Setting | Purpose |
|---|---|
| `bind_addr` | Gateway HTTP address, for example `0.0.0.0:8787` or a Tailscale IP. |
| `herdr_session` | Herdr session name used for every herdr invocation. |
| `allowed_roots` | Workspace roots Herdr Go is allowed to hand to herdr. Keep this narrow. |
| `poll_interval_ms` | Agent status polling interval. |
| `herdr_protocol` | Pinned herdr wire protocol version. |
| `static_dir` | Optional on-disk web UI override for local iteration. |
| `herdr_socket` | Optional explicit herdr local endpoint. |

## Deployment choices

Use the default service on a trusted LAN or tailnet. Prefer binding to a
Tailscale address for phone access across networks.

Do not expose Herdr Go directly to the public internet. If you need access
through an edge network, put TLS and access control at a trusted reverse proxy
and keep `HERDR_GO_WEB_SECRET` strong.

The production unit is `herdr-go.service`. Repository development can use
`herdr-go-dev.service`; deploying either mode stops the other mode and both
legacy units first.

## Develop from source

Install stable Rust and Node.js 22, then:

```bash
git clone https://github.com/vantt/herdr-go
cd herdr-go
cd web
npm ci
npm run bundle
cd ..
cargo build --release
```

Run `./target/release/herdr-go`, or use `./dev-deploy.sh` on Linux for the
development user service.

## Troubleshooting

Run the built-in doctor first:

```bash
herdr-go doctor
```

Inspect service logs:

```bash
journalctl --user -u herdr-go.service -f
```

If login fails, confirm `HERDR_GO_WEB_SECRET` exists in
`${XDG_CONFIG_HOME:-$HOME/.config}/herdr-go/herdr-go.env` and that the file is
readable only by your user. If startup reports both legacy and canonical state,
inspect them manually; Herdr Go intentionally never merges them.

## Project docs

- [Product backlog](docs/backlog.md)
- [System overview](docs/specs/system-overview.md)
- [Discovery notes](docs/DISCOVERY.md)

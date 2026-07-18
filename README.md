# Herdr Go (`herdctl`)

Keep an eye on your coding agents from your phone. Herdr Go gives you a mobile-first view of agents running in [herdr](https://github.com/ogulcancelik/herdr), lets you read their terminals and reply, and restarts herdr when it goes down.

## Be back at your desk only when it matters

- See which agents are working, blocked, done, or idle.
- Open an agent's terminal, zoom in, and send text or keys from your phone.
- Keep the gateway and herdr healthy through a self-healing user service.
- Put one fail-closed web token in front of herdr's unauthenticated socket.

## Install on systemd-based Linux

The supported one-command path needs a working systemd user service manager,
but no clone or Rust/Node toolchain:

```bash
curl -fSL https://raw.githubusercontent.com/vantt/herdr-go/main/install.sh | bash
systemctl --user start herdr-go.service
```

On a first install, the installer prints a new login token. Repeat and migrated
installs preserve the existing token; [retrieve or rotate it locally](docs/installation.md#login-token).
Open `http://<your-machine>:8787` from a phone on the same trusted LAN or
tailnet and sign in. For a safer remote setup, bind to a Tailscale address; do
not expose the service directly to the public internet.

See [installation](docs/installation.md) for upgrades, uninstall, and platform boundaries, then [usage](docs/usage.md) for the everyday workflow.

## Try the UI locally

```bash
herdctl --demo
```

Open <http://127.0.0.1:8787> and sign in with `demo`.
The memorable demo token is safe by default because demo mode listens only on
loopback. To expose it intentionally, pass an explicit address, for example
`herdctl --demo --bind 0.0.0.0:8787`, and secure the network around it.

## Learn more

- [Installation](docs/installation.md)
- [Using Herdr Go](docs/usage.md)
- [Advanced deployment](docs/advanced/deployment.md)
- [Configuration](docs/advanced/configuration.md)
- [Build from source](docs/advanced/source-build.md)
- [Troubleshooting](docs/advanced/troubleshooting.md)
- [Architecture](docs/advanced/architecture.md)

The executable and Rust crate remain named `herdctl`.

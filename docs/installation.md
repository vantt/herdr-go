# Install Herdr Go

## Recommended: Linux user service

```bash
curl -fSL https://raw.githubusercontent.com/vantt/herdr-go/main/install.sh | bash
systemctl --user start herdr-go.service
```

The installer downloads the matching release, creates state under `~/.config/herdr-go` and `~/.local/share/herdr-go`, and installs a systemd user unit. It preserves existing files on upgrades. Legacy `herdr-gateway` directories are renamed only when the new directory is absent; if both exist, the new directory wins and the old one is left untouched with a warning.

Check it:

```bash
systemctl --user status herdr-go.service
journalctl --user -u herdr-go.service -f
herdctl doctor
```

## Upgrade

Run the installer again, then restart:

```bash
curl -fSL https://raw.githubusercontent.com/vantt/herdr-go/main/install.sh | bash
systemctl --user restart herdr-go.service
```

## Uninstall

```bash
systemctl --user disable --now herdr-go.service
rm ~/.config/systemd/user/herdr-go.service ~/.local/bin/herdctl
systemctl --user daemon-reload
```

Your config and data remain until you deliberately remove them.

## Platform boundary

The no-clone service installer is verified for Linux on x86_64 and arm64. macOS and Windows release artifacts may exist, but this systemd installation flow is not offered there. See [build from source](advanced/source-build.md) for development and other platforms.

Next: [use Herdr Go](usage.md), or open [advanced configuration](advanced/configuration.md).

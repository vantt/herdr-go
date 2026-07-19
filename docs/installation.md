# Install Herdr Go

## No-clone installer: pending the first renamed release

The installer expects a published `herdr-go-<platform>` archive. No matching
renamed asset has been published and smoke-tested yet, so the curl command below
is a future install path, not a currently working fresh-install instruction.
For now, use [build from source](advanced/source-build.md); repository developers
can use `dev-deploy.sh` on systemd-based Linux after cloning the source.

Once the renamed asset is available, the command will be:

```bash
curl -fSL https://raw.githubusercontent.com/vantt/herdr-go/main/install.sh | bash
systemctl --user start herdr-go.service
```

This path requires `systemctl` and a reachable systemd user service manager.
The installer proves both prerequisites before migrating state, downloading or
installing files, or changing services. It then downloads the matching release,
creates state under `~/.config/herdr-go` and `~/.local/share/herdr-go`, and
installs a systemd user unit. It preserves existing files on upgrades. Legacy
`herdr-gateway` directories are renamed only when the new directory is absent;
if both exist, the new directory wins and the old one is left untouched with a
warning.

### Release checklist: remove the pending notice

Remove the pending-asset caveat from this guide and the README only after all
of the following evidence exists for a matching `herdr-go-<platform>` asset:

1. The asset is present on the renamed repository release.
2. `install.sh` downloads and extracts it on a fresh supported Linux account.
3. The extracted `herdctl` runs successfully.
4. The installer creates, enables, starts, and verifies the user service.

Publishing an asset without this real download/extract/run/service smoke is not
enough to promote the curl path.

## Login token

Only a first install creates and prints a login token. Repeat installs and
migrations preserve the existing token and do not print it into installer or
service logs. Retrieve it locally from the protected environment file at
`${XDG_CONFIG_HOME:-$HOME/.config}/herdr-go/herdctl.env`:

```bash
env_file="${XDG_CONFIG_HOME:-$HOME/.config}/herdr-go/herdctl.env"
sed -n 's/^HERDCTL_WEB_SECRET=//p' "$env_file"
```

To rotate it, replace `HERDCTL_WEB_SECRET` in that file without putting the
token in a journal or log command, keep the file readable only by your user,
and restart the service:

```bash
env_file="${XDG_CONFIG_HOME:-$HOME/.config}/herdr-go/herdctl.env"
${EDITOR:-vi} "$env_file"
chmod 600 "$env_file"
systemctl --user restart herdr-go.service
```

Check it:

```bash
systemctl --user status herdr-go.service
journalctl --user -u herdr-go.service -f
herdctl doctor
```

## Upgrade

After the renamed release path has passed the checklist above, run the installer
again, then restart:

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

The no-clone service installer is verified for systemd-based Linux on x86_64
and arm64 when the current login has a working user service manager. Other
Linux init/session setups, macOS, and Windows are outside this service-install
path. See [build from source](advanced/source-build.md) for development and
other platforms.

Next: [use Herdr Go](usage.md), or open [advanced configuration](advanced/configuration.md).

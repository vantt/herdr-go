# Use Herdr Go

The no-clone curl installer is still pending the first matching renamed release
asset and its real install smoke test. Until then, run Herdr Go from a source
build or use `herdctl --demo`; see [installation](installation.md).

## Notice what needs you

Open the gateway from your phone. The agent list shows working, blocked, done, idle, and unknown states so you can scan before opening a terminal.

## Read an agent

Tap an agent to open its terminal snapshot. Pinch or pan to inspect dense output. Herdr remains the source of truth for agent/session lifecycle.

## Reply

Tap **Type** to send a message, or **Keys** for Arrow, Enter, and Escape controls. Confirm the selected agent before sending input.

## Diagnose a problem

```bash
herdctl doctor
journalctl --user -u herdr-go.service -f
```

## Sign in again or rotate the token

The installer prints a token only when it creates one on first install. On a
repeat or migrated install, retrieve the preserved token locally (without
putting it in service logs) from
`${XDG_CONFIG_HOME:-$HOME/.config}/herdr-go/herdctl.env`:

```bash
env_file="${XDG_CONFIG_HOME:-$HOME/.config}/herdr-go/herdctl.env"
sed -n 's/^HERDCTL_WEB_SECRET=//p' "$env_file"
```

To rotate it, rewrite `HERDCTL_WEB_SECRET` in that file without putting the
token in a journal or log command, keep mode `600`, and restart the user
service:

```bash
env_file="${XDG_CONFIG_HOME:-$HOME/.config}/herdr-go/herdctl.env"
${EDITOR:-vi} "$env_file"
chmod 600 "$env_file"
systemctl --user restart herdr-go.service
```

For networking and exposure choices, see [advanced deployment](advanced/deployment.md). For detailed fixes, see [troubleshooting](advanced/troubleshooting.md).

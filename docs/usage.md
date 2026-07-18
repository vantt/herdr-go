# Use Herdr Go

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

For networking and exposure choices, see [advanced deployment](advanced/deployment.md). For detailed fixes, see [troubleshooting](advanced/troubleshooting.md).

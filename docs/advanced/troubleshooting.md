# Troubleshooting

Run the built-in doctor first:

```bash
herdr-go doctor
```

Inspect service logs:

```bash
# macOS
log show --predicate 'process == "herdr-go"' --last 1h
# Linux
journalctl --user -u herdr-go.service -f
```

If login fails, confirm `HERDR_GO_WEB_SECRET` exists in the config directory's `herdr-go.env` (see [Installation: Login token](../installation.md#login-token)) and that the file is readable only by your user.

If startup reports both legacy and canonical state, inspect them manually — Herdr Go intentionally never merges them.

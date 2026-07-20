# Troubleshooting

Run the built-in doctor first:

```bash
herdr-go doctor
```

At a real terminal, it offers to fix anything it finds broken, then offers
to let you edit any setting or secret. Use `herdr-go doctor --check` to
only see the report without being prompted (safe for scripts and CI).

Inspect service logs:

```bash
# macOS
log show --predicate 'process == "herdr-go"' --last 1h
# Linux
journalctl --user -u herdr-go.service -f
```

If login fails, confirm `HERDR_GO_WEB_SECRET` exists in the config directory's `herdr-go.env` (see [Installation: Login token](../installation.md#login-token)) and that the file is readable only by your user. The same applies to `HERDR_GO_GITHUB_TOKEN`/`HERDR_GO_TELEGRAM_TOKEN` if a GitHub or Telegram feature isn't working: `herdr-go` reads each token from the process environment first, then falls back to `herdr-go.env` — but only if that file is still owner-only. `herdr-go doctor` can create or fix any of these three secrets interactively without hand-editing the file.

If startup reports both legacy and canonical state, inspect them manually — Herdr Go intentionally never merges them.

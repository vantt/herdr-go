# Troubleshooting

Run `herdctl doctor` first. Inspect service logs with `journalctl --user -u herdr-go.service -f`. If login fails, confirm `HERDCTL_WEB_SECRET` exists in the canonical config directory. If startup reports both legacy and canonical state, inspect them manually; Herdr Go intentionally never merges them.

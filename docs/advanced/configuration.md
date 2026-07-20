# Configuration

The canonical config file:

```text
# macOS
~/Library/Application Support/herdr-go/config.json
# Linux
${XDG_CONFIG_HOME:-$HOME/.config}/herdr-go/config.json
```

Durable SQLite data:

```text
# macOS
~/Library/Application Support/herdr-go/  (same directory as config)
# Linux
${XDG_DATA_HOME:-$HOME/.local/share}/herdr-go
```

Secrets live in `herdr-go.env` beside the config file, mode `600`.

## Settings

| Setting | Purpose |
|---|---|
| `bind_addr` | Gateway HTTP address, for example `0.0.0.0:8787` or a Tailscale IP. |
| `herdr_session` | Herdr session name used for every herdr invocation. |
| `allowed_roots` | Workspace roots Herdr Go is allowed to hand to herdr. Keep this narrow. |
| `poll_interval_ms` | Agent status polling interval. |
| `herdr_protocol` | Pinned herdr wire protocol version. |
| `static_dir` | Optional on-disk web UI override for local iteration. |
| `herdr_socket` | Optional explicit herdr local endpoint. |

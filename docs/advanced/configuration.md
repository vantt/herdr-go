# Configuration

The canonical config is `${XDG_CONFIG_HOME:-~/.config}/herdr-go/config.json`; durable SQLite data is under `${XDG_DATA_HOME:-~/.local/share}/herdr-go`. Secrets live in `herdctl.env` beside the config with mode 600.

Key settings are `bind_addr`, `herdr_session`, `allowed_roots`, `poll_interval_ms`, `herdr_protocol`, `static_dir`, and optional `herdr_socket`. Keep `allowed_roots` narrow and use a strong `HERDCTL_WEB_SECRET`.

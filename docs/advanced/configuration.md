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

Secrets (the login token, and optionally a GitHub and a Telegram token) live
in `herdr-go.env` beside the config file, mode `600`. Both the config file
and the secrets file can be edited by hand at any time, or through
`herdr-go doctor` — run it interactively and it diagnoses your setup,
offers to fix anything broken, and (via a single end-of-run prompt) lets
you edit any setting below or any secret without opening a file.

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
| `telegram_chat_id` | Destination Telegram chat for notifications. Not a secret (the bot token is); leave unset to keep notifications off. |

## Secrets

| Secret (env var / `herdr-go.env` key) | Purpose |
|---|---|
| `HERDR_GO_WEB_SECRET` | Login token for the web UI. Auto-generated on first run if unset. |
| `HERDR_GO_GITHUB_TOKEN` | Optional GitHub token, enables the GitHub integration. |
| `HERDR_GO_TELEGRAM_TOKEN` | Optional Telegram bot token, enables Telegram notifications (paired with `telegram_chat_id` above). |

Each secret is read from the process environment first; if unset there,
`herdr-go` falls back to the protected `herdr-go.env` file, but only when
that file's permissions are still owner-only. A process-environment value
always wins over the file.

## Cloudflare Access (optional)

Skip the app's own login screen by letting Cloudflare Access authenticate at
the edge instead. Off by default — the token/cookie login above keeps
working unchanged whether or not this is set up, and CF Access is only ever
an *additional* way in, never a replacement.

1. **Put the gateway behind a Cloudflare Tunnel**, not just a public port
   plus an Access rule. A Tunnel (`cloudflared`) is what makes the origin
   unreachable by any path other than through Cloudflare — an Access rule
   alone doesn't stop someone who already has the plain address. Point the
   tunnel at the gateway's `bind_addr` (above). See Cloudflare's own Tunnel
   and Access Application docs for that walkthrough; this project doesn't
   wrap it.
2. In the Cloudflare Zero Trust dashboard, create an **Access Application**
   for the tunnel's public hostname, with whatever policy (email domain,
   group, etc.) should be allowed in.
3. Copy two values from that dashboard into `config.json` (hand-edit —
   `herdr-go doctor` doesn't cover these two fields yet):
   - `cf_access_team_domain` — your team's origin, e.g.
     `https://your-team.cloudflareaccess.com` (Zero Trust → Settings →
     Custom Pages, or the domain the dashboard itself is under).
   - `cf_access_aud` — the Application Audience (AUD) tag, on the Access
     Application's Overview tab.

   Both must be set together, or CF Access verification stays off.
4. Restart the gateway (`herdr-go service restart`, or your platform's
   equivalent). The startup log line `cloudflare access verification
   enabled` confirms it picked up both fields.
5. Visit the gateway through the tunnel's hostname — Cloudflare Access
   challenges you first (its own login, not this app's), and once past
   that you land straight in Herdr Go with no `/login` screen. Visiting by
   any other route (e.g. the tailnet IP directly) still needs the regular
   token login, unchanged.

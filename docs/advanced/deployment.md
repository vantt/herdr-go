# Advanced deployment

Herdr Go defaults to `0.0.0.0:8787`. Prefer a Tailscale address for phone access, or bind loopback and use SSH forwarding. Never expose the service directly to the public internet; use TLS at a trusted reverse proxy when needed.

The production unit is `herdr-go.service`; repository development uses `herdr-go-dev.service`. Deploying either mode stops the other mode and both legacy units first.

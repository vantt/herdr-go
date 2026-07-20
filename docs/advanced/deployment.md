# Deployment choices

Use the default service on a trusted LAN or tailnet. Prefer binding to a Tailscale address for phone access across networks.

Do not expose Herdr Go directly to the public internet. If you need access through an edge network, put TLS and access control at a trusted reverse proxy and keep `HERDR_GO_WEB_SECRET` strong.

The production unit is `herdr-go.service` (Linux) / the `io.github.vantt.herdr-go` LaunchAgent (macOS). Repository development can use `herdr-go-dev.service` on Linux; deploying either mode stops the other mode and legacy units first.

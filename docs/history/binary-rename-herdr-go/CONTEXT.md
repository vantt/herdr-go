# Context: Binary identity rename to herdr-go

## Goal

Make `herdr-go` the only active executable and Rust identity before the first
release. Retire `herdctl` everywhere current product behavior, packaging,
diagnostics, documentation, tests, and operator commands name the gateway.

## Locked Decisions

- **D1 - Single public executable identity.** The built binary is `herdr-go`
  on Unix-like targets and `herdr-go.exe` on Windows. Operator commands use
  `herdr-go`, not `herdctl`.
- **D2 - No compatibility alias.** The first public release has no existing
  user base to preserve, so no `herdctl` shim, symlink, alternate install name,
  fallback env var, legacy token filename, or compatibility docs are added.
- **D3 - Rust package/library follows the product.** The Cargo package,
  default run target, binary target, library crate, imports, test imports, and
  tracing filter namespace use `herdr_go`/`herdr-go` as appropriate for Rust and
  Cargo naming rules.
- **D4 - Runtime file/env names follow the product.** Token and state files are
  `herdr-go.env` and `herdr-go-state.sqlite`; public environment variables are
  `HERDR_GO_*`, including the web secret, provisioning token, Telegram token,
  supervisor Herdr binary override, and Windows smoke binary.
- **D5 - Release/install/CI contracts change atomically.** Release workflows,
  CI smoke paths, installer extraction checks, installed binary paths, service
  templates, and development deployment all expect the same executable name.
- **D6 - Historical evidence is not rewritten.** Old feature history,
  decisions, reports, and arbitrary fixture labels may keep `herdctl` where
  they describe prior reality. Current specs, README, install docs, runtime
  docs, code, tests, and packaging must not present `herdctl` as active.

## Boundaries

- Do not change the product home `herdr-go`; it is already canonical.
- Do not add migration or fallback reads for old token/state filenames.
- Do not rewrite past audit history only to erase old words.
- Do not change Herdr's own upstream binary name or protocol behavior.

## Success

- `cargo run --bin herdr-go -- --help` and `cargo run --bin herdr-go -- --demo`
  use the renamed binary surface.
- Rust tests import the renamed library crate and pass.
- Installer, release workflow, Windows smoke, service templates, README, and
  specs all agree on `herdr-go`.
- A scoped current-surface scan finds no active `herdctl`/`HERDCTL` references
  outside historical records or deliberately unrelated fixture strings.
- Full verify passes.

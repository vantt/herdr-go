---
status: Approved
feature: binary-rename-herdr-go
mode: high-risk
decisions: [D1, D2, D3, D4, D5, D6, 178345a6-768c-4645-909f-1ab0a61f523f]
---

# Implementation Plan: Binary identity rename to herdr-go

## Goal

Herdr Go ships with one active executable identity: `herdr-go`. The old
`herdctl` name is removed from active runtime, packaging, docs, CI, and tests
before the first public release.

## Scope In

- Cargo package, binary target, library crate, and Rust imports.
- CLI help, logs, doctor output, tracing namespace, token env vars, token file,
  sqlite state file, and supervisor override env var.
- Installer, dev deploy, service templates, release workflow, CI Windows smoke,
  and current docs/specs/tests.

## Scope Out

- Historical reports/decisions and arbitrary old feature records.
- Any compatibility alias, symlink, fallback env var, or migration from
  `herdctl` filenames.
- Release publication or tagging.

## Technical Design

Cargo uses `name = "herdr-go"` for the package and binary target, and
`herdr_go` for the library crate. Rust call sites import `herdr_go::...`, while
operator-facing command examples use `herdr-go`.

Runtime naming moves together: the login token comes from
`HERDR_GO_WEB_SECRET` or `herdr-go.env`; other public env vars use
`HERDR_GO_*`; persistent sqlite state is `herdr-go-state.sqlite`; and the
Windows proof passes `HERDR_GO_SMOKE_BINARY` into the smoke script. No old env
var or filename is accepted.

Packaging and docs consume the same executable name. The release workflow
packages `herdr-go`; the installer extracts and installs `herdr-go`; service
templates execute `herdr-go`; README/specs present only `herdr-go`.

## Affected Files

Primary files: `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `src/lib.rs`,
`src/config/mod.rs`, `src/doctor.rs`, `src/supervisor.rs`,
`src/notify/telegram.rs`, `.github/workflows/ci.yml`,
`.github/workflows/release.yml`, `install.sh`, `dev-deploy.sh`,
`packaging/herdr-go.service`, `packaging/herdr-go-dev.service`,
`scripts/windows-runtime-smoke.ps1`, `tests/rename_contract.sh`,
`tests/observe_reply_e2e.rs`, `README.md`, `docs/PRD.md`,
`docs/specs/system-overview.md`, and `docs/specs/installation.md`.

## Implementation Steps

1. Rename Cargo identity and Rust imports.
2. Rename runtime env/file strings and update unit tests.
3. Rename package/install/CI/release/smoke contracts.
4. Update current docs/specs and rename contract checks.
5. Run focused name checks and full verify.

## Validation Plan

Use the validation report at
`docs/history/binary-rename-herdr-go/reports/validation-current-slice.md`, then
run the cell's focused grep/runtime contracts and the full project verify.

## Risks & Mitigation

- Runtime name split: mitigated by one atomic cell and scoped current-surface
  scan.
- Packaging drift: mitigated by installer/release/CI grep contracts.
- Rust crate breakage: mitigated by full Rust tests and clippy.
- Hidden old env var: mitigated by rejecting any active `HERDCTL` scan hit.

## Security / Permissions

The web secret remains protected as before, but its active env/file names change
to `HERDR_GO_WEB_SECRET` and `herdr-go.env`. No token contents are logged or
read into planning artifacts.

## Rollback Plan

Revert the single rename cell commit and re-run full verify. Because D2 forbids
compatibility/migration work, rollback is a source-level revert only; there is
no mixed-name runtime mode to preserve.

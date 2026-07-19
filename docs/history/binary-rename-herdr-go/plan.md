---
artifact_contract: bee-plan/v1
artifact_readiness: implementation-ready
mode: high-risk
---

# Plan: Binary identity rename to herdr-go

Mode: `high-risk` - 4 risk flags: public contracts, cross-platform, existing
covered behavior, multi-domain. This touches the CLI contract, Cargo identity,
CI/release packaging, installer/service paths, runtime secrets/state naming,
Windows proof scripts, docs, and tests.

## Requirements

- D1: Build and document `herdr-go` / `herdr-go.exe` as the only executable.
- D2: Add no `herdctl` compatibility alias, shim, fallback, or docs.
- D3: Rename Cargo package/bin/lib and Rust imports to `herdr_go`.
- D4: Rename runtime env vars and files to `HERDR_GO_*`,
  `herdr-go.env`, and `herdr-go-state.sqlite`.
- D5: Keep release producer, installer consumer, CI smoke, service templates,
  and dev deploy in one atomic contract.
- D6: Leave historical evidence alone unless it is a current-facing spec/doc.

## Discovery

Current `herdctl` references are concentrated in:

- Cargo identity: `Cargo.toml`, `Cargo.lock`, `src/main.rs`,
  `tests/observe_reply_e2e.rs`.
- Runtime contract: CLI help/log text, tracing filter, sqlite state filename,
  doctor output, config/env var resolution, supervisor Herdr-binary override.
- Packaging: `.github/workflows/ci.yml`, `.github/workflows/release.yml`,
  `install.sh`, `dev-deploy.sh`, `packaging/herdr-go*.service`,
  `scripts/windows-runtime-smoke.ps1`.
- Documentation/specs/tests: README, installation/system specs, PRD/current
  docs, rename contract tests.

No external research is needed; this is a repository contract rename with
existing tests and shell contracts as proof.

## Approach

Perform one synchronized rename slice:

1. Rename Cargo package/bin/lib and Rust imports.
2. Rename runtime env vars, token file, sqlite file, CLI help, logs, doctor,
   supervisor override, and tests.
3. Rename install/release/CI/dev-deploy/service/Windows smoke references.
4. Update current docs/specs and contract tests; leave historical reports alone.
5. Run focused contract checks, then full verify.

Rejected alternatives:

- Binary-only rename while keeping crate/env/state as `herdctl`: rejected
  because it leaves two active identities.
- Compatibility alias/fallback: rejected by D2 before first release.
- Blind global replacement: rejected because historical evidence remains valid
  under D6.

## Risk Map

| Component | Risk | Proof Needed |
|---|---|---|
| Cargo/bin/lib rename | HIGH | `cargo test`, clippy, `cargo run --bin herdr-go -- --help` |
| Runtime env/token/state names | HIGH | config/doctor tests and current-surface scan |
| Installer/release/CI contract | HIGH | grep contract and shell syntax checks |
| Windows smoke path/env names | HIGH | script-level name contract and CI workflow references |
| Docs/spec consistency | MEDIUM | current docs scan excludes active `herdctl` |

## Current Slice

One cell, `binary-rename-herdr-go-1`, owns the complete active-surface rename.
This is intentionally atomic: splitting code/docs/packaging would create
temporary broken contracts between release producer, installer consumer, and
runtime names.

## Verification

- `cargo fmt --all --check`
- `cargo test --quiet`
- `cargo clippy --quiet -- -D warnings`
- `cd web && npm run bundle && npm run test -- --run`
- Focused rename contracts:
  - no active `herdctl` / `HERDCTL` references in current source, packaging,
    tests, README, and specs;
  - `cargo run --quiet --bin herdr-go -- --help` succeeds and prints
    `HERDR_GO_WEB_SECRET`;
  - release/install/CI scripts reference `herdr-go` executable paths.

## Out of Scope

- Rewriting historical reports and decisions.
- Publishing, tagging, or running remote GitHub release jobs.
- Changing Herdr upstream executable naming.

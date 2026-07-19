# Context: Rename herdr-gateway to herdr-go

## Goal

Rename the GitHub repository and current product surfaces to **herdr-go**, preserving a working upgrade path for existing installations.

## Locked decisions

- **D1 — Complete current-surface rename.** Current branding, repository URLs, package metadata, release artifact names, systemd unit names, operator documentation, and source-facing product strings become `herdr-go`.
- **D2 — Preserve historical truth.** Existing `.bee/` records, feature history, research reports, and arbitrary workspace-label fixtures retain `herdr-gateway` where it records past reality rather than current product identity.
- **D3 — Explicit state migration.** Canonical config and data directories become `herdr-go`; an existing `herdr-gateway` directory is migrated only when the new directory is absent. If both exist, prefer the new directory and leave the old one untouched rather than merging secrets or SQLite state.
- **D4 — Safe service migration.** Installation replaces the legacy systemd unit without allowing old and new gateway services to run simultaneously. Compatibility paths remain writable during the transition.
- **D5 — Atomic release contract.** The release workflow and installer change together from `herdr-gateway-<target>` to `herdr-go-<target>`, followed by a real artifact smoke test when the renamed repository publishes an asset.
- **D6 — Preserve executable identity.** The Rust crate and executable remain `herdctl`; this request renames the product/repository, not the CLI command.
- **D7 — Lead users to value and immediate use.** README becomes a sales-oriented landing page and the primary Linux install path works without cloning the repository. Installation and usage guides stay task-first; source builds, configuration internals, deployment variants, troubleshooting details, and architecture move into an advanced documentation layer.
- **D8 — Keep the promoted path safe and usable.** Demo mode binds to loopback by default because its credential is intentionally memorable. Installed services retain hardening without making supervised user workspaces read-only. The installer supports systemd-based Linux with a functioning user service manager and proves that prerequisite before migrating or installing anything.
- **D9 — Isolate migration and pre-release claims.** Legacy default-state migration runs exactly when doctor is off, demo is off, and no explicit config path was supplied (`!doctor && !demo && config_path.is_none()`); an explicit bind override remains normal startup and still migrates. Doctor, demo, explicit-config, and any mixed invocation containing one of those modes never move default legacy directories. Development deploy validates platform, toolchain, systemctl, and the user manager before any migration/build/filesystem/service mutation. Until a matching `herdr-go-<target>` asset is published and smoke-tested, README and installation guidance label the curl installer unavailable at or before any command block and point to truthful alternatives; the release checklist removes this caveat only after the real smoke test.

## Boundaries

- Rename GitHub repository `vantt/herdr-gateway` to `vantt/herdr-go` and update local `origin`.
- Do not rewrite git history, historical audit records, or observed workspace-name fixtures.
- Do not overwrite an existing new config/data directory during migration.
- Do not expose or copy secret contents into logs or documentation.
- Do not market a one-command path on a platform where it is not implemented and verified; the no-clone installer is Linux-first until equivalent macOS/Windows service integration exists.

## Success

- GitHub and `origin` resolve to `vantt/herdr-go`.
- No current product surface still presents `herdr-gateway` except deliberate legacy migration references.
- Existing config, secret file, and SQLite state remain discoverable after upgrade.
- Release producer and installer consumer agree on the new archive name.
- Rust, lint, web bundle, tests, shell syntax, and rename-specific checks pass.
- Once a matching renamed release asset is published, a fresh Linux user can start from the published installer URL in an empty directory with no checkout; before then, docs clearly mark that path unavailable and offer truthful alternatives.
- README communicates the mobile agent-monitoring value before implementation detail and links progressively to task guides and advanced operator/developer material.
- The promoted demo is unreachable from other hosts unless the user explicitly changes its bind address, supervised agents can write ordinary user workspaces, and unsupported Linux environments fail before filesystem/service mutation.

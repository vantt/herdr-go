---
artifact_contract: bee-plan/v1
artifact_readiness: implementation-ready
mode: high-risk
---

# Plan: Rename herdr-gateway to herdr-go

Mode: `high-risk` — 6 risk flags: data loss, external system, public contracts, cross-platform, existing covered behavior, multi-domain.
Why this is the least workflow that protects the work: the rename crosses GitHub, release assets, user state, system services, source metadata, and operator documentation.

## Requirements

- D1: Rename every current product/repository surface to `herdr-go`.
- D2: Keep historical evidence and arbitrary upstream workspace-label fixtures unchanged.
- D3: Migrate legacy config/data only when the canonical destination is absent; never merge two populated stores.
- D4: Move installations to the new service name without duplicate live processes.
- D5: Keep release archive producer and installer consumer atomic.
- D6: Keep the `herdctl` executable identity.
- D7: Make the first Linux experience no-clone and value-first; keep technical depth available in an advanced documentation layer.
- D8: Make the promoted demo loopback-only, preserve supervised workspace writes, and preflight the systemd user-service prerequisite before mutation.
- D9: Run legacy default-state migration only for the normal default service path; keep doctor, demo, and explicit-config modes side-effect free with respect to default legacy state. Apply preflight-before-mutation to dev deploy, and do not advertise the renamed quick install as available until a matching release asset exists.

## Discovery

A scoped repository scan classified references into current branding, GitHub URLs, package metadata, release contracts, service contracts, persistence paths, fixtures, and immutable history. The detailed implementation path and risks are in `approach.md`.

## Shape

The GitHub repository and `origin` are renamed first, following the user's requested sequence. One atomic local execution slice then updates runtime migration, packaging/release contracts, the self-contained Linux bootstrap, current branding, and a progressive documentation hierarchy.

## Test matrix

- Existing legacy config/data only: moves to canonical location and remains usable.
- Canonical config/data only: no migration or overwrite.
- Both names exist: canonical wins; legacy remains untouched and a warning is available.
- Missing legacy/new directories: fresh install creates only canonical paths.
- Production install: stop/disable legacy production, legacy development, and new development before new production starts.
- Development deploy: stop/disable legacy production, legacy development, and new production before new development starts.
- Installer/deploy rerun: service migration remains idempotent and legacy unit files are removed.
- Release archive: workflow producer and installer URL/extraction root use the same `herdr-go-<target>` name.
- Empty-directory Linux install: a remotely fetched installer needs no checkout-local config or service template.
- Download failure: exits with an actionable error; source-build instructions remain available in advanced docs rather than silently assuming a checkout.
- macOS/Windows: docs do not promise the Linux one-command service install; release assets are described only at their verified support level.
- Current-surface stale-name scan: only allowlisted migration/history references remain.
- README funnel: value and immediate action precede architecture/configuration; the primary path contains no `git clone`.
- Documentation links: README → installation/usage → advanced deployment/configuration/source-build/troubleshooting/architecture resolve in the repository and release archive.
- Full Rust and web verification remains green.
- Mode isolation: doctor, demo, and explicit-config invocations do not move default legacy directories.
- Development deployment: prerequisite/user-manager checks run before legacy migration, directory creation, compilation, or service mutation.
- Asset availability: current docs state that quick install becomes usable only after the first matching `herdr-go` release asset; no unperformed release smoke test is implied.

## Out of scope

- Renaming the `herdctl` binary or Rust crate.
- Rewriting historical docs, audit logs, decisions, or git history.
- Publishing a new release tag without a separate explicit release request.
- Removing legacy migration support immediately after this rename.

## Current slice

Final acceptance correction after the completed rename. Entry state: GitHub/origin and current surfaces are renamed, but mode selection can still move legacy default state during doctor/demo/explicit-config runs, dev deploy mutates before prerequisites, and the documented curl path has no matching renamed release asset. Exit state: migration is isolated to normal default-state startup (including bind-only override), dev deploy fails before mutation on unsupported environments, and docs truthfully gate curl installation on the first published/smoke-tested `herdr-go` asset.

Bounded files are declared in cell `rename-herdr-go-3`. Verification combines a main-wired mode matrix, dev-deploy preflight/mutation-class checks, asset-availability/documentation contracts, shell syntax, formatting, Rust tests/lints, web bundle/tests, and all prior rename contracts.

## Cells

- `rename-herdr-go-1` — atomic runtime, packaging, release, branding, and documentation rename.
- `rename-herdr-go-2` — acceptance correction for safe demo defaults, writable workspaces, systemd preflight, and repeat-install login guidance.
- `rename-herdr-go-3` — final correction for mode-isolated migration, dev-deploy preflight, and truthful pre-release install availability.

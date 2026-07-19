---
artifact_contract: bee-implement-plan/v1
feature: rename-herdr-go
lane: high-risk
status: Approved
updated: 2026-07-18
sources: [CONTEXT.md, approach.md, plan.md]
decisions: [D1, D2, D3, D4, D5, D6, D7]
---

# Implementation Plan: Herdr Go rename

> Human-layer projection of the truth artifacts. Truth lives in `CONTEXT.md`,
> `plan.md`, and the execution cell. Feedback flows back to those sources first.

## 1. Goal

Rename the product and repository to **Herdr Go** without losing existing user state, while making the first Linux experience feel like a product rather than a source project: understand the value, install without cloning, and open it from a phone.

**Success looks like**

- Current product surfaces use `herdr-go`; historical truth remains intact (D1–D2).
- Existing configuration, credentials, SQLite history, and service operation survive the migration safely (D3–D5).
- The executable remains `herdctl` (D6).
- A fresh Linux user installs from a published URL in an empty directory, and README leads with value before routing into task and advanced guides (D7).

## 2. Current State

The repository slug, release archives, systemd units, config/data directories, package metadata, and operator documentation still use `herdr-gateway`. The installer reads checkout-local configuration and service-template files, so the documented path starts with cloning the repository. README and the installation guide expose architecture and toolchain detail before a user can experience the product.

## 3. Scope

**In scope**

- Current branding, URLs, persistence migration, systemd migration, and release producer/consumer names (D1–D5).
- A self-contained, remotely fetched Linux installer with an explicit failure path when no compatible release asset is available (D7).
- A sales-oriented README, task-first installation/usage guides, and advanced guides for deployment, configuration, source builds, troubleshooting, and architecture (D7).
- Contract checks for relative links, release-packaged guides, platform claims, stale names, service ordering, and install behavior.

**Out of scope**

- Renaming `herdctl`, rewriting history, or publishing a release.
- Claiming one-command system-service installation for macOS or Windows.
- Removing legacy migration support immediately after the rename.

## 4. Proposed Approach

Follow the requested order: rename GitHub first, verify `origin`, then apply one atomic compatibility-aware local slice. The installer emits the required configuration and service contract itself, downloads the matching `herdr-go-<target>` release, and points to the advanced source-build guide if no suitable asset exists. Documentation becomes a progressive funnel: README value and quick start → installation/usage tasks → advanced operator/developer guides.

Rejected alternatives include blind replacement, clone-first primary installation, a remote source-build fallback that assumes a checkout, and unsupported cross-platform parity claims.

## 5. Technical Design

```text
published install URL -> self-contained installer -> release asset + config + unit
                      -> running Linux service -> open from phone

README value/quick start -> installation and usage -> advanced guides
```

The application resolves `herdr-go` as the canonical config/data identity. Before creating a canonical directory, it renames an old-only sibling without inspecting contents; a failed move aborts, while a both-exist state preserves both and selects the new path with an observable warning. Production and development deployment stop every conflicting legacy/new unit before starting the selected mode and remove legacy unit files idempotently. Release packaging and installer extraction use the same archive root and include each relative guide linked from the packaged README.

### Security and permissions

Migration never reads or logs secret values and never overwrites a canonical directory. Generated configuration and unit content contains no embedded credentials. Runtime, installer, and unit agree on default and custom XDG paths. A failed download or migration exits before enabling or starting a new unit.

## 6. Affected Files

The authoritative file boundary is cell `rename-herdr-go-1`. Its major surfaces are:

| Action | File / component | Purpose |
|---|---|---|
| Modify | `install.sh` | Self-contained no-clone Linux bootstrap and migration |
| Modify | `.github/workflows/release.yml` | Matching release artifact and packaged guides |
| Modify | `README.md` | Value-first landing page and immediate start |
| Modify | `docs/installation.md`, `docs/usage.md` | Task-first user journey |
| Create | `docs/advanced/*.md` | Progressive technical depth |
| Create | `tests/rename_contract.sh` | Installer, service, rename, and documentation contracts |
| Modify/create/remove | runtime and `packaging/` rename surfaces | Safe state/service compatibility migration |

## 7. Implementation Steps

- [ ] Rename current product surfaces with compatibility migration (`rename-herdr-go-1`, D1–D7).

## 8. Validation Plan

Automated verification will cover an empty-directory, network-stubbed installer run; no checkout-local template dependency; actionable missing-asset failure; fake-systemctl ordering and idempotency; custom-XDG agreement; producer/consumer archive naming; stale-name allowlisting; README value-first/no-clone assertions; local Markdown links; packaged guide presence; platform-claim boundaries; shell syntax; Rust format/tests/lints; and web bundle/tests.

A real renamed-release smoke test remains pending until a `herdr-go` asset is published. Current feasibility evidence is recorded in [`reports/validation-local-rename-and-docs.md`](reports/validation-local-rename-and-docs.md).

## 9. Risks and Mitigation

| Risk | Impact | Mitigation |
|---|---|---|
| State migration hides or overwrites user data | High | Content-opaque, fail-closed rename and old/new/both tests |
| Conflicting services bind the same port | High | Fake-systemctl conflict-matrix and ordering checks |
| Installer and release artifact drift | High | Producer/consumer contract assertions |
| Remote installer assumes a checkout | High | Empty-directory installer probe and fail-before-start behavior |
| Marketing copy overpromises or links break | Medium | Structure, link, package-content, and platform-claim checks |

## 10. Rollback Plan

Revert the atomic local cell, documentation, and installer together so the docs never advertise an incompatible installer. Before external adoption, restore the prior GitHub name and `origin`. If state moved, stop the new service and move canonical directories back only when legacy destinations are absent; never overwrite either side. Restore the legacy unit before re-enabling it. After users adopt a renamed release, recover through a forward compatibility release rather than destructive directory moves.

## 11. Open Questions

No blocking design questions remain. The real published-asset smoke test is deferred until a release exists.

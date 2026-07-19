---
date: 2026-07-18
feature: rename-herdr-go
categories: [pattern, decision, failure]
severity: standard
tags: [installer, documentation, security, systemd, release]
---

# Learning: Verify the Promoted Path as a Product Contract

## Learning 1 — Derive checks from every quick-start promise

**Category:** failure
**Severity:** standard
**Applicable-when:** a README promotes a demo, installer, or first-run path.

### What Happened

The first rename commit passed its broad verification but still paired a known demo credential with an all-interface bind, described generic Linux support for a systemd-dependent installer, and omitted the repeat-install token journey. A focused acceptance pass caught the mismatch between what README promoted and what runtime/install branches actually did. `tests/rename_contract.sh` now checks the safe demo default, explicit override, platform boundary, and token lifecycle.

### Root Cause

The first contract test followed rename mechanics and links, not each user-visible promise and its failure states.

### Recommendation

When README promotes a path, turn every claim into an executable positive and negative check: fresh, repeat, migrated, unsupported environment, and deliberate exposure override.

## Learning 2 — Evaluate service hardening against the workload

**Category:** pattern
**Severity:** standard
**Applicable-when:** a background service must write user-owned workspaces.

### What Happened

`ProtectHome=read-only` prevented supervised agents from editing ordinary projects even though writable config/data paths were reopened. The correction removed that independent home restriction while retaining `NoNewPrivileges=true` and `ProtectSystem=strict` in both generated and packaged units. The contract test pins both forms together.

### Root Cause

Hardening directives were treated as independent checklist items without checking their combined effect on the product's core workload.

### Recommendation

When changing service hardening, test the actual workload under the composed policy, keep generated and packaged units identical, and retain unrelated protections explicitly.

## Learning 3 — Prove prerequisites before the first mutation

**Category:** pattern
**Severity:** standard
**Applicable-when:** installers or migrations depend on a platform/session service.

### What Happened

The initial installer could move legacy state and download files before discovering that the per-user service manager was unavailable. The correction runs command detection and a side-effect-free user-manager probe before migration, temporary directories, downloads, writes, removals, or service calls. The contract harness checks source ordering for each mutation class.

### Root Cause

Platform checks proved the operating system and architecture but not the session-level prerequisite that the installation actually needs.

### Recommendation

When an installer depends on a service manager or runtime, prove reachability before the first mutation and add an invariant for every new mutation class.

## Learning 4 — Keep coupled identities atomic and historical truth scoped

**Category:** decision
**Severity:** standard
**Applicable-when:** renaming a mature product with durable user state and historical records.

### What Happened

The rename changed runtime paths, service units, release producer/consumer names, repository URLs, and current docs together while leaving historical records and opaque fixtures intact. State migration uses a content-opaque, fail-closed sibling rename and never merges two populated stores. Scoped stale-name checks distinguish current surfaces from deliberate legacy/history references.

### Root Cause

Product names are operational contracts across persistence, packaging, deployment, and documentation—not merely branding strings.

### Recommendation

When renaming a product, map producer/consumer and persistence contracts first, migrate state fail-closed, and scan only current surfaces instead of globally rewriting history.

## Remaining proof

The no-clone installer is locally and synthetically verified, but a real `herdr-go-<target>` release asset does not exist yet. Smoke-test the published asset from an empty environment immediately after the first renamed release.

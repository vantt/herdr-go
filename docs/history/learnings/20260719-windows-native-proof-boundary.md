---
date: 2026-07-19
feature: windows-support
categories: [pattern, decision, failure]
severity: standard
tags: [windows, ipc, security, verification, offline]
---

# Learning: Keep Native Windows Implementation Separate from Windows Proof

## Learning: Reuse an Available Native Transport Before Adding an Offline Dependency

**Category:** pattern
**Severity:** standard
**Tags:** [windows, ipc, offline]
**Applicable-when:** A platform integration needs a new transport but the build environment cannot fetch packages.

## What Happened

The first transport implementation selected the same local-socket package used by upstream herdr, but the package was absent from the local cache and the environment could not resolve the package registry. The implementation was reshaped to use the already-present asynchronous runtime's Windows named-pipe client while preserving herdr's endpoint-name conversion and existing newline-delimited protocol. Focused socket, supervisor, migration, and lint checks then passed.

## Root Cause

The plan verified upstream compatibility but did not check package availability under the repository's offline execution constraints before making the dependency part of the implementation path.

## Recommendation

When network access is unavailable, check the local dependency cache before approving a new package; if an existing dependency exposes the required native primitive, preserve the upstream wire and naming contract with that primitive instead of blocking implementation on a download.

## Learning: Protect Token Storage Before Secret Bytes Exist

**Category:** decision
**Severity:** standard
**Tags:** [windows, acl, token]
**Applicable-when:** A secret file is created or reopened on a platform with inherited access-control rules.

## What Happened

The Windows state implementation creates a protected parent with inheritable owner-only access before atomically publishing token bytes. Every startup validates the effective protection of an existing token before reading it, and any protection failure propagates before the listener starts. A final security pass also caught that the protected directory access entries needed object and container inheritance flags.

## Root Cause

Hardening a file after writing leaves a disclosure window, while checking only creation-time intent does not prove that an existing file remains protected. Directory protection also does not automatically secure descendants unless inheritance semantics are explicit.

## Recommendation

When creating a local secret, establish and validate the protected container first, publish the secret atomically inside it, validate the effective protection again before every read, and make startup fail before serving if any step is uncertain.

## Learning: Treat Real Windows Execution as a Release Claim Boundary

**Category:** failure
**Severity:** standard
**Tags:** [windows, ci, verification]
**Applicable-when:** Platform-specific code is developed on a host that cannot compile or execute that platform branch.

## What Happened

Linux host tests and linting proved the portable seams and preserved Linux behavior, while an additive Windows Server 2022 job and fail-fast real-herdr smoke were added for native compile, named-pipe traffic, restart recovery, state roots, token preservation, ACL inspection, and a distinct ordinary user's denied read. The proof remains blocked because this host has neither PowerShell nor a Windows target and remote workflow execution was outside the authorized environment.

## Root Cause

Conditional platform code can be syntactically and behaviorally invisible to the host compiler. A script present in the repository is not evidence that the target runtime, external integration, and operating-system security boundary work.

## Recommendation

When the host cannot execute a target platform, isolate real target execution in a blocking proof unit and withhold every support or release claim until that unit passes on the pinned target runner.

## Analysis Coverage Gap

The compounding workflow's three read-only analyst roles were unavailable because this runtime exposes no read-only subagent type. This entry was synthesized from capped cell traces and completed worker reports; no write-capable analyst was substituted.

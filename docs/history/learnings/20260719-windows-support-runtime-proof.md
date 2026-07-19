---
date: 2026-07-19
feature: windows-support
categories: [failure, pattern]
severity: standard
tags: [windows, ci, external-binary, runtime-smoke]
---

# Learning: Windows Runtime Proof Must Cover Restarted Processes Too

**Category:** failure  
**Severity:** standard  
**Tags:** [windows, ci, external-binary, runtime-smoke]  
**Applicable-when:** a CI smoke test downloads or verifies a tool binary and the application later launches that tool itself

## What Happened

The Windows proof initially bound the smoke script's direct Herdr invocations to the checksum-verified executable, but gateway supervisor recovery still spawned the ambient `herdr` command. GitHub Actions run 29675584823 passed login, agent listing, terminal observation, input/reply, subscription, and token isolation, then failed waiting for recovery after the real Herdr server was stopped. Commit `f94a120` fixed the supervised restart path by letting the gateway inherit the verified Herdr binary path, and run 29675809304 passed the Windows runtime smoke.

## Root Cause

The proof treated "the test invokes the verified binary" as enough, but the product under test also invokes the external binary during recovery. That second launch path was outside the evidence boundary and therefore silently depended on runner PATH state.

## Recommendation

When a smoke test verifies an external executable by checksum, pass that exact executable path through every product launch/restart path exercised by the smoke. Include a restart/recovery assertion before declaring the binary binding complete.

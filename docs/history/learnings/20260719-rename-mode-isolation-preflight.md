---
date: 2026-07-19
feature: rename-herdr-go
categories: [pattern, decision, failure]
severity: standard
tags: [migration, cli-modes, preflight, verification]
---

# Learning: Isolate Default Migration and Prove Preflight Ordering

## Learning: Route migration through the real CLI-mode seam

**Category:** pattern
**Severity:** standard
**Tags:** [migration, cli-modes]
**Applicable-when:** a default-path migration coexists with diagnostic, demo, or explicit-configuration modes

### What Happened

The rename correction made the same mode decision used by normal startup control whether legacy default state moves. A table-driven mode matrix covers normal and bind-only startup as migration modes, while diagnostic, demo, explicit configuration, and their relevant combinations remain isolated.

### Root Cause

Testing a detached predicate would not prove that the application entry point actually observes it. Migration is safe only when the production startup path and the test matrix share the same decision seam.

### Recommendation

When adding a default-state migration, gate it through the production mode-selection seam and table-test every non-normal invocation that must remain read-only with respect to default state.

## Learning: Preflight must precede every mutation class

**Category:** decision
**Severity:** standard
**Tags:** [deployment, preflight]
**Applicable-when:** an installer or development deployment performs migration, compilation, filesystem writes, or service changes

### What Happened

The development deployment now checks platform, required tools, service-manager availability, and a reachable user service manager before migration, compilation, directory writes, unit changes, or service restarts. The contract test checks executed mutation classes and verifies that a failed prerequisite leaves legacy state untouched.

### Root Cause

Checking only the textual position of function definitions can produce false confidence: a script may still execute a mutation before its prerequisites are known to work.

### Recommendation

When validating installer safety, enumerate every executed mutation class and prove through a failing-prerequisite harness that none runs before preflight succeeds.

## Learning: Separate product failures from sandbox denials

**Category:** failure
**Severity:** standard
**Tags:** [verification, sandbox]
**Applicable-when:** integration tests create listeners or require filesystem capabilities unavailable to the runner

### What Happened

The verification runner denied listener creation with `Operation not permitted` before three end-to-end assertions could execute. Formatting, static analysis, shell contracts, 88 runnable Rust tests, the web bundle, and 20 web tests passed; Git commit creation was separately blocked because the runner exposes `.git` read-only.

### Root Cause

The execution environment withholds network-listener and Git-write capabilities that the project verification and one-commit-per-cell workflow normally expect.

### Recommendation

When a runner denies a required capability before product behavior executes, preserve the exact denial as an environmental exception, run all independent checks, and require a capable CI or host run before calling the affected end-to-end behavior or commit complete.

---
date: 2026-07-19
feature: binary-rename-herdr-go
categories: [failure, verification]
severity: medium
tags: [rename, product-identity, verify-commands]
---

# Active-source fixtures count as product surface during identity renames

## What Happened

The first rename pass changed the public binary, crate, env vars, token/state
files, release/install wiring, CI, docs, and tests from the retired executable
identity to `herdr-go`. The post-worker active-surface scan still found two
current Rust fixture strings under `src/web/mod.rs` using the retired product
name in fake missing-directory paths. The cell had to be reopened for a focused
cleanup commit.

## Root Cause

The negative scan originally covered the obvious product surfaces but did not
cover every current source file that could carry product-shaped fixture strings.
Those strings were not operator-visible, but they were still active source and
therefore violated the requested "no retired identity" rule.

## Recommendation

When renaming a product or executable identity, define the negative scan as
"current active source, packaging, tests, README, and specs", then explicitly
exclude only historical evidence. Treat source fixtures as active surface when
they embed the old product name, even if the string is only a fake path.

The check belongs in an executable contract test, not just a manual grep.

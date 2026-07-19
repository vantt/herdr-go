---
date: 2026-07-19
feature: release-packaging-p1-fix
categories: [failure, pattern]
severity: standard
tags: [release, packaging, documentation, verification]
---

# Learning: Release Package Documentation Needs a Contract Check

**Category:** failure
**Severity:** standard
**Tags:** [release, packaging, documentation, verification]
**Applicable-when:** a workflow packages documentation or other repo paths into
a release archive

## What Happened

The release workflow packaged deleted documentation paths, so a tag release
could pass normal Rust and web verification but fail while creating the
downloadable archive. The fix removed the stale documentation inputs from the
release package and added a guard in `tests/rename_contract.sh` that checks the
actual workflow Package step for documentation inputs. A follow-up tightened the
guard so markdown globs must resolve to at least one file, not just literal
markdown paths.

## Root Cause

The package manifest lived in workflow text and was not exercised by the normal
verification suite. Documentation consolidation changed the file set, but no
contract test proved that the release Package step still referenced only
existing files.

## Recommendation

When a release workflow copies repo files into an archive, validate the workflow
inputs directly in a contract test. Check literal paths for existence and check
globs for at least one match, so package drift fails before a tag release.

Mechanized check: `tests/rename_contract.sh`.

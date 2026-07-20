---
date: 2026-07-20
feature: cross-platform-install
categories: [failure, pattern]
severity: critical
tags: [security, macos, launchd, secrets, persona-panel]
---

# Learning: A plausible-sounding secret-transport design can be wrong for a platform that has no equivalent mechanism

**Category:** failure
**Severity:** critical
**Tags:** [security, macos, launchd, secrets, persona-panel]
**Applicable-when:** porting a systemd-based service definition (or any Linux-native lifecycle mechanism) to a different platform's native equivalent, specifically anywhere the Linux version relies on `EnvironmentFile=` or similar OS-level secret injection.

## What Happened

The original cell design for a macOS launchd LaunchAgent said: "since launchd plists cannot source an external env file at load time, read the token env file's key=value lines and populate the plist's EnvironmentVariables dict inline... never embed the token literal elsewhere." This sentence contradicts itself (populating EnvironmentVariables *is* embedding the literal elsewhere), and the persona-panel validation (security lens) caught that the resulting plist — written to `~/Library/LaunchAgents/*.plist` under a normal `umask 022` — would default to mode 644, readable by any other local user, directly violating the repo's own R12 rule ("the login token is protected for its owner before its bytes become visible").

The actual fix was not "add a chmod 600 step to patch the leak" — it was to notice the transport wasn't needed at all: the Rust binary already resolves its own token by reading the secrets file directly at startup (`ensure_web_secret()` falls back to reading `herdr-go.env` after checking the environment), exactly mirroring what systemd's `EnvironmentFile=` does for the Linux unit. The plist never needed to carry the secret in the first place.

## Root Cause

The design reasoned from "systemd has `EnvironmentFile=`, launchd doesn't have a direct equivalent, so I need to invent a substitute" instead of first checking whether the *consuming program* already solves the problem on its own. It does — `ensure_web_secret()` was written platform-agnostically from the start. A missing OS-level mechanism doesn't always need a workaround; sometimes the application layer already covers it.

## Recommendation

Before designing a platform-specific secret-transport workaround (env injection, wrapper scripts, keychain integration, etc.), check whether the consuming program already resolves the secret itself independent of how it was launched. If it does, the "problem" a workaround was about to solve doesn't exist — the platform-specific launcher only needs to invoke the program normally. Simplicity revealed a design flaw that a permissions patch (`chmod 600` on the plist) would have merely covered up, leaving the file-content-as-secret-copy that never needed to exist. This is also a case for a real persona-panel security lens on any cell touching secrets, even when the cell's own prohibitions already name the right constraint in prose — the *action* still needs to be checked for actually satisfying it, not just the must_haves.

**Full trace:** `docs/history/cross-platform-install/reports/validation-slice1-macos.md`, `.bee/cells/cross-platform-install-2.json` (patch history shows the before/after action text).

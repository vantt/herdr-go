---
date: 2026-07-20
feature: cross-platform-install
categories: [failure, pattern]
severity: critical
tags: [security, windows, acl, secrets, persona-panel]
---

# Learning: reimplementing a platform's file-permission API in an installer script is a boot-breaking risk, not just a leak risk

**Category:** failure
**Severity:** critical
**Tags:** [security, windows, acl, secrets, persona-panel]
**Applicable-when:** any installer script (shell, PowerShell, etc.) that creates a secret file the main program will later validate the permissions of at startup — on any platform with a non-trivial permission model (Windows ACLs/DACLs especially, but the same class applies to POSIX ACLs beyond plain mode bits).

## What Happened

This is the Windows sibling of the macOS launchd-plist finding from the same feature (`20260720-cross-platform-install-secret-in-plist.md`), but with a sharper consequence. The original `install.ps1` design would have created `herdr-go.env` directly with plain PowerShell file operations under `%APPDATA%\herdr-go`. A file created that way inherits the parent folder's default ACL, which includes SYSTEM and Administrators access entries. The Rust binary's own startup check (`validate_owner_only`, mirroring the POSIX mode-bit check but for Windows ACLs) rejects any token file whose DACL grants access to any principal other than the current user — so the installer-created file would have been rejected **on every subsequent launch**, not just readable by other users. This is a boot failure, one step worse than the macOS case (which was a confidentiality leak but the process still ran).

The fix was the same shape as the macOS one: don't reimplement the platform's permission API in the installer at all. The Rust binary already creates and correctly ACL-protects the token file on its own first run (`ensure_web_secret()` → `prepare_token_directory()` → `windows::protect_directory()`, which sets a protected DACL via `SetFileSecurityW` and an SDDL string). The installer only needed to start the program and, optionally, read the file afterward to echo the token once.

## Root Cause

Same as the macOS case: assuming a workaround was needed for a step the platform's own file-creation defaults don't handle safely, without checking whether the actual consuming program already has that exact logic, tested and working, one layer down. The mistake surfaces differently per platform depending on what "wrong" permissions actually do — silently leak (macOS default 644) vs. hard-reject at every startup (Windows ACL mismatch) — but the underlying design error is identical.

## Recommendation

When an installer needs to create a file the main program will validate the security of, prefer: **let the program create that specific file on its own first run**, and have the installer only start the program and optionally wait-and-read afterward. Never re-derive a platform's access-control API in installer-script code when the consuming binary already implements it correctly — a second implementation is a second chance to get subtly wrong, and on Windows specifically, "subtly wrong" here means the app refuses to boot, not just a security softness. This is now the second time in one feature this exact class of mistake was caught by the persona panel's security lens before execution — worth treating "does the installer duplicate the program's own secret/permission handling" as a standing question for any future installer-touching cell.

**Full trace:** `docs/history/cross-platform-install/reports/validation-slice2-windows.md`, `.bee/cells/cross-platform-install-4.json` (patch history), `src/config/mod.rs:754-798` (the logic `install.ps1` defers to instead of reimplementing).

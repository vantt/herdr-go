---
artifact_contract: bee-plan/v1
artifact_readiness: requirements-only
mode: standard
---

# Plan: windows-release-matrix

Mode: `standard` — 2 risk flags: cross-platform, existing-covered-behavior (release.yml already ships the Linux/macOS release path; a third-party download/checksum pattern is reused, not new, per decision b8c3d4bc)
Why this is the least workflow that protects the work: touches a shared, already-live release pipeline (real users install from its output) and must not regress the two P1s just fixed in review-v0-1-1-rc, but it is a single bounded slice with no gray-area product decisions left open — decisions edbb7a4e/86491143/cf2c1b3b/b8c3d4bc/15189a97 already pin every open question.

## Requirements (from CONTEXT.md / cited decisions)

- Per windows-support D8: target `x86_64-pc-windows-msvc`, pinned to Windows Server 2022 (not a floating runner alias).
- Per windows-support D4/D6: first distribution is a foreground-only ZIP — no Windows Service, auto-start, PowerShell installer, or dev-deploy material.
- Per windows-support D5: Linux/macOS release behavior and artifacts stay unchanged.
- Per decision edbb7a4e: Windows artifacts remain withheld until native runtime proof passes, and CI must execute only a checksum-verified pinned upstream binary — this is why the release-windows job's proof step downloads and verifies a pinned SHA256 before ever executing anything, mirroring ci.yml exactly.
- Per decision 86491143: the Windows runtime proof now passes on real Windows Server 2022 CI (run 29675809304, commit f94a120) — the D6 withholding condition is satisfied.
- Per decision b8c3d4bc: the release job proves itself inline (duplicates ci.yml's windows-server-2022 download/checksum/smoke steps) rather than a cross-workflow `needs:` — GitHub Actions has no native cross-workflow job dependency.
- Per decision 15189a97: the Windows archive gets its own file allowlist — binary, `config.example.json`, `README.md`, `docs/installation.md`, `docs/advanced/*.md` — and must never include `install.sh` or `packaging/herdr-go.service`.

## Discovery

L0/L1 — the pattern already exists in this repo. Inspected `.github/workflows/ci.yml`'s `windows-server-2022` job (download+SHA256-verify the upstream Herdr Windows preview, then `scripts/windows-runtime-smoke.ps1`) and `.github/workflows/release.yml`'s existing Linux/macOS matrix + Package step (`cp config.example.json install.sh README.md ...`, `packaging/herdr-go.service`). No external research needed; both source and target patterns are read directly from this repo's own files.

## Approach

Recommended path (revised after validating's plan-checker found the original "add a matrix entry" shape structurally broken — see below): add a **new, separate top-level job** `release-windows` to `release.yml`, running on `windows-2022`, with its own self-contained steps (checkout, Rust MSVC toolchain, web bundle, cargo build, an inline proof step reusing ci.yml's download-checksum-smoke sequence, package, upload). This job does not touch the existing `build` job's matrix or steps at all.

Rejected alternatives:
- **Add `x86_64-pc-windows-msvc` as a fourth entry in the existing `build` job's matrix** (original plan) — rejected: `build` has one shared `steps:` list applied to every matrix entry (the Package step at minimum copies `install.sh`/`packaging/herdr-go.service` unconditionally); making Windows co-exist there requires adding `if:` OS-guards to the Build/Package steps, which the plan's own prohibition ("do not modify Linux/macOS steps") then forbids — a contradiction a cold worker cannot resolve. A separate job has no such conflict: Linux/macOS's `build` job is untouched byte-for-byte.
- Cross-workflow `workflow_run` trigger (release reacts to CI's conclusion) — rejected: GitHub Actions' `workflow_run` fires on the *default branch*'s workflow completion, not cleanly on the tag being released; adds indirection for no real safety gain over proving the exact release commit inline.
- `gh api` polling for CI's check-run status on the tag's commit — rejected: adds a new external dependency (GitHub API auth/rate limits) to solve a problem the inline duplicate already solves more simply.
- Extracting the proof steps into a shared composite action now — deferred: real DRY win, but not required for this slice (YAGNI); file as backlog if the pattern is needed a third time.

Risk map:
| Component | Risk | Proof needed |
|---|---|---|
| Windows job packaging (no install.sh/service) | LOW | grep the produced archive contents in the cell's verify |
| Inline proof step (duplicated smoke) | LOW | same script already proven in ci.yml; reusing verbatim |
| Matrix `fail-fast: false` interaction | LOW | already set at job level; a Windows failure does not block Linux/macOS uploads |

## Shape

Single phase — this is one bounded slice, not a multi-milestone effort:

| Phase | What Changes | Why Now | Demo | Unlocks |
|---|---|---|---|---|
| 1 | `release.yml` gains a new, separate `release-windows` job (not a `build` matrix entry): its own build/package steps, an inline proof step reusing ci.yml's checksum+smoke pattern, and a Windows-only file allowlist | The withholding condition (D6) is now satisfied; this is the only remaining gap before Windows can ship in a tagged release | A tag push produces a `herdr-go-x86_64-pc-windows-msvc.zip` containing only the binary + docs, and the job fails closed if the inline proof fails, with the existing `build` job's Linux/macOS output completely untouched | Windows users can install from a published release asset instead of building from source |

## Test matrix

- **Cross-platform:** Windows job uses its own OS-conditional build/package/archive steps; Linux/macOS steps and output are byte-for-byte unchanged (verified by diffing the job's other matrix entries).
- **Failure path:** inline proof step (checksum mismatch or smoke failure) fails the Windows job and produces no artifact upload, while Linux/macOS entries still complete (`fail-fast: false`).
- **Contract:** archive contents allowlist — assert no `install.sh` or `packaging/herdr-go.service` present in the Windows archive; assert the binary is present and named `herdr-go.exe`.

## Out of scope

- Windows Service / auto-start / PowerShell installer / dev-deploy packaging — stays deferred per D4/D9 until a follow-up feature.
- Extracting the proof steps into a shared composite action — deferred backlog idea, not this slice.
- Any change to Linux/macOS release behavior.

## Current slice

**windows-release-matrix-1** — entry state: release.yml ships only Linux musl x2 + macOS arm64 (one `build` job, matrix-driven), no Windows target exists anywhere. Exit state: release.yml gains a new, separate top-level job `release-windows` (not a `build` matrix entry) running on `windows-2022`, gated by an inline proof step reusing ci.yml's checksum+smoke pattern, packaging a Windows-only archive with no Linux lifecycle material; the existing `build` job and its Linux/macOS output are byte-for-byte unchanged. Files: `.github/workflows/release.yml`. Verify: a structure-aware YAML check that locates the `release-windows` job specifically (not a positional grep) and asserts on its content in isolation — see the cell's `verify` command.

## Cells

- `windows-release-matrix-1`

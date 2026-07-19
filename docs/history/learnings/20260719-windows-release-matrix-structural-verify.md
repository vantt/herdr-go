---
date: 2026-07-19
feature: windows-release-matrix
categories: [pattern, decision, failure]
severity: critical
tags: [github-actions, ci, cell-authoring, verify-commands, plan-checker]
---

# Learning: Adding a platform to a shared-matrix CI job needs a separate job, and its verify must parse the config

**Category:** failure, pattern
**Severity:** critical
**Tags:** [github-actions, ci, cell-authoring, verify-commands, plan-checker]
**Applicable-when:** planning or authoring a cell that adds a new OS/target/platform to an existing CI or release workflow with a matrix, or authoring any cell whose `verify` inspects a YAML/JSON/TOML config file.

## What Happened

Planning's original design for adding a Windows target to `.github/workflows/release.yml` assumed a matrix entry could carry "its own OS-conditional steps." The actual file has one `build` job with a single shared `steps:` list applied to every matrix entry (Linux x2 + macOS). Making Windows co-exist there would have required adding `if:` OS-guards to the shared Build/Package steps — which the same cell's own prohibition ("do not modify Linux/macOS steps") then forbade. A cold-pickup worker could not have resolved this contradiction.

Separately, the cell's original `verify` command used a positional `grep -B2 -A40 'x86_64-pc-windows-msvc' file | grep -q 'install.sh\|herdr-go.service'` — a text-window heuristic that is order-dependent: a correct implementation placing Windows content near the retained Linux `install.sh` reference (physically, in YAML) would false-FAIL; a Windows step far enough away would false-PASS.

Both were caught by validating's plan-checker (`bee-review`, adversarial, read-only) BEFORE a worker was dispatched — never during execution.

## Root Cause

The plan was validated against intent and surface-level file inspection ("what does ci.yml's Windows job do", "what does release.yml's Package step copy") but never against the *actual parsed shape* of the target file — whether `build`'s `steps:` was per-matrix-entry or shared, and whether the verify's positional grep was equivalent to a real structural assertion. Both misses share one root cause: intent-level review, not structure-level review, until a dedicated adversarial pass forced the structural read.

## Recommendation

1. **New platform in an existing CI matrix → new top-level job, not a new matrix entry**, whenever the existing job has one shared `steps:` list (check this explicitly before authoring the cell: `python3 -c "import yaml; print(yaml.safe_load(open(path))['jobs']['<job>']['steps'])"` — if it's a single list applied via `strategy.matrix`, a same-job addition requires OS-guarding shared steps). A separate job leaves the existing platforms' steps byte-for-byte untouched and needs no conditional logic.
2. **Any cell `verify` targeting a `.yml`/`.json`/`.toml` artifact must parse it with the matching loader and assert on parsed keys/values — never a positional text window** (`grep -A/-B`, line-offset `sed`, etc.). Template: `python3 -c "import yaml,sys; d=yaml.safe_load(open(f)); job=d['jobs']['name']; assert <condition on job>; print('OK')"`. Before trusting a new structural verify, manually run it against the pre-change file and confirm it fails for the right reason (proves no false-pass).
3. When a cell's action targets a shared region (job/file) that a prohibition also names, check reachability first: can the action be done at all without touching the prohibited region? If not, the design is broken before a worker ever sees it — this is a five-minute YAML-shape check at plan time, not something to defer to validating.

**Full trace:** `docs/history/windows-release-matrix/plan.md` (Approach section documents both rejected/redesigned alternatives), `docs/history/windows-release-matrix/reports/validation-windows-release-matrix-1.md` (both plan-checker iterations), `.bee/cells/windows-release-matrix-1.json`.

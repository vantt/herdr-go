# Validation report: cross-platform-install Slice 2 (Windows)

## Reality gate

| Check | Verdict | Evidence |
|---|---|---|
| MODE FIT | PASS | high-risk (inherited from the epic; same flags as Slice 1) |
| REPO FIT | PASS | `install.sh`'s flow, `windows-release-matrix`'s published Windows asset, and `src/config/mod.rs`'s existing Windows path/secret logic all exist as the templates this cell extends |
| ASSUMPTIONS | PASS (after fix) | ScheduledTasks cmdlets are real; the token-transport and ACL assumptions were wrong on first pass and are now fixed (see Plan-checker below) |
| SMALLER PATH | PASS | single cell, single new file, matches Slice 1's granularity |
| PROOF SURFACE | PASS | verify is a real command, self-tested to fail pre-fix for the right reason |

## Feasibility matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| Task Scheduler cmdlets used are real and used correctly | MEDIUM | code inspection against known PowerShell ScheduledTasks module API | Confirmed real; 2 factual errors found and fixed (`RunLevel` belongs on `New-ScheduledTaskPrincipal`, not `New-ScheduledTaskSettingsSet`; `RestartInterval` has a 1-minute minimum, no literal systemd-parity claim) | **PASS** (after fix) |
| Windows token file can be created and protected safely from PowerShell | HIGH (security + correctness) | code inspection of `ensure_web_secret()`/`prepare_token_directory()`/`windows::protect_directory` | `src/config/mod.rs:754-798`: the binary already creates AND ACL-protects `herdr-go.env` on its own first run via `SetFileSecurityW`+SDDL; a PowerShell-created file under `%APPDATA%\herdr-go` would inherit SYSTEM/Administrators ACEs and be **rejected by `validate_owner_only` at every subsequent startup** — a boot-breaking bug found before execution | **PASS** — cell redesigned so `install.ps1` never touches `herdr-go.env`; the binary's own first run creates and protects it, exactly mirroring the already-proven logic instead of reimplementing Windows ACL manipulation in PowerShell |
| Roaming-path assumption (`$env:APPDATA\herdr-go`) matches what the binary actually resolves | MEDIUM | code inspection | Confirmed correct: `config_dir()` → `base_config_dir()` → `native_roots().roaming` (`mod.rs:626-632`) = `%APPDATA%`; `ensure_web_secret()` reads `config_dir().join("herdr-go.env")` — no path mismatch | PASS |
| The cell's verify can't be trivially fooled by a stub | LOW | direct inspection of the verify script | Original verify used 4 bare substring matches with no negative assertions — a stub could satisfy it as comments. Strengthened with negative assertions (no `herdr-go.env` reference, no `HERDR_GO_WEB_SECRET=` assignment, no `RunLevel` on the wrong cmdlet) plus a `Register-ScheduledTask` presence check | PASS (after fix) |

Schedule: single cell, no deps, trivial one-node wave.

## Persona panel (4 lenses, `bee-review`/opus, iteration 1)

- **Coherence:** PASS — D4/D5/D6/D10 all covered, no self-contradiction (confirmed the original design genuinely avoided repeating Slice 1's exact self-contradiction pattern).
- **Feasibility:** WARNING → fixed — `RunLevel` cmdlet placement error, and an overstated crash-restart-parity claim (Task Scheduler's 1-minute `RestartInterval` floor).
- **Security (load-bearing):** **CRITICAL** → fixed — token-injection avoidance itself was genuine and the roaming-path claim was correct, but the cell would have created `herdr-go.env` with inherited ACEs from PowerShell, which the binary's own `validate_owner_only` check rejects on every startup — a boot-breaking bug, not just a leak. Root-fixed by having `install.ps1` never touch that file at all, deferring entirely to the binary's own already-proven creation+ACL logic.
- **Scope-guardian:** WARNING → fixed — clean file ownership confirmed, but the original verify was too weak (positive substring matches only, no negative assertions on the load-bearing "no secret in the task" property). Strengthened.

**Fix applied:** cell `cross-platform-install-4` redesigned to remove all PowerShell-side secret creation, corrected the two API-placement facts, and the verify script (`.bee/spikes/cross-platform-install/check-install-ps1.py`) now asserts both presence (real cmdlets used) and absence (no `herdr-go.env` touch, no token assignment, no misplaced `RunLevel`). Self-verified by the orchestrator against each of the panel's 4 findings; no second panel dispatch needed since each fix directly and structurally resolves its finding.

## Cell review (cold pickup)

Covered inline by the persona panel (scope-guardian lens = cell review for this dispatch). No CRITICAL flags remain after the fix.

## Advisor consult (AO2b)

No advisor configured (same as Slice 1) — recorded as `none-configured` per AO2(b).

## Decision

**READY.** `cross-platform-install-4` is approved for execution as Slice 2.

## Approval block

- Lane: `high-risk`. Gate bypass level `full` — auto-approves Gate 3 at every lane including high-risk/hard-gate; the advisor-consult mechanical precondition is satisfied below before this approval, independent of bypass level.
- `approved_gates.execution` set via `bee.mjs state gate --lane cross-platform-install --name execution --approved true` (this is a fresh approval for this slice — the stale `true` left over from Slice 1's approval was not treated as covering this new cell).

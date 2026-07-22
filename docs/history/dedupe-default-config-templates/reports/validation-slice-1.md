# Validation Report — dedupe-default-config-templates, slice 1 (E1, all 6 cells)

**Date:** 2026-07-22
**Lane:** standard
**Decision:** READY

## Reality Gate

| Check | Result | Evidence |
|---|---|---|
| MODE FIT | PASS | 6 product files across Rust + bash + PowerShell + JSON (`src/config/mod.rs`, `src/main.rs`, `src/doctor/checks.rs`, `install.sh`, `install.ps1`, `config.example.json`) exceed `small`'s 3-file cap; 2 risk flags (cross-platform, multi-domain) per `plan.md`. |
| REPO FIT | PASS | `--internal-merge-config` precedent confirmed at `main.rs:29-32,86,168-171`; `config::default_config_json` confirmed canonical at `mod.rs:851-863`; doctor's duplicate confirmed at `checks.rs:902-920`; `install.sh:151` and `install.ps1:132-140` literals confirmed missing `agent_presets`; `config.example.json` confirmed missing `agent_presets`. |
| ASSUMPTIONS | PASS (post-fix) | Root-resolution logic confirmed identical between `ensure_config` and doctor's copy (safe to dedupe). BOM risk on `install.ps1` identified and fixed (see Findings). |
| SMALLER PATH | PASS | One cohesive slice, no artificial phases; `standard` is the minimum lane that fits 6 files/3 languages. |
| PROOF SURFACE | PASS (post-fix) | Every cell's `verify` runs a real command; two vacuous-pass gaps found and fixed (see Findings). |

## Feasibility Matrix

| Assumption | Risk | Proof Required | Evidence | Result |
|---|---|---|---|---|
| Hidden self-exec CLI verb pattern (`--internal-merge-config`) is real and copyable | LOW | grep `main.rs` | Confirmed lines 29-32, 86, 168-171 | PASS |
| Doctor's local `default_config_json` is byte-identical to canonical today | LOW | Read both functions | Confirmed identical field set, both already emit `agent_presets` | PASS |
| Linux `install.sh` does not auto-start the systemd unit (D4's rationale for keeping the installer write) | MEDIUM | Read `install.sh` service section | Confirmed: `systemctl --user enable` only, no `--now`; macOS/Windows do auto-start | PASS |
| `install.ps1` write must avoid a UTF-8 BOM or `config.json` fails to parse at first start | HIGH (silent breakage) | Inspect existing write pattern | cell-review CRITICAL finding — fixed: cell -5 now mandates `WriteAllText` + `UTF8Encoding($false)`, bans `Out-File`, verify checks for both | PASS (post-fix) |
| Cells -2/-3's new-test verify commands must not pass vacuously | MEDIUM | Run `cargo test` against a nonexistent filter | plan-checker + cell-review both confirmed vacuous pass (exit 0, "0 passed ... filtered out") — fixed via file-exists grep gates naming the exact required test function | PASS (post-fix) |
| Baseline repo state is green before any cell is claimed | REQUIRED | Run the full recorded `verify` command | Initially RED — `web/` deps had never been installed in this fresh worktree (`tsc: not found`). Fixed with `npm install`; full chain re-run green: 312+2+3 Rust tests, `cargo fmt --check`, `cargo clippy -D warnings`, `rename_contract.sh`, `npm run bundle`, 79 web tests (5 files) | PASS (post-fix, unrelated to this feature's code) |
| Cell dependency graph has no cycles | REQUIRED | `bee cells schedule` | 3 waves (`[-1,-6] → [-2,-3] → [-4,-5]`), zero cycles, zero unsatisfiable deps | PASS |

No spike was needed — every MEDIUM/HIGH item above resolved to direct evidence rather than an unproven assumption.

## Plan-Checker Findings (structural, 5 dimensions)

Verdict: STRUCTURALLY SOUND. Decision coverage complete (D1→scope, D2→cells -1/-2/-3, D3→-3, D4→-2/-4/-5, D5→correctly no cell, D6→-4/-5, D7→-6). Dependency chain and key-link checkability confirmed against live code.

- **WARNING 1** (cells -2, -3): verify's `cargo test <named-filter>` passed vacuously against the unmodified repo (no such test exists yet, exit 0). **Fixed**: each verify now greps for the exact required test function name first; each action pins that exact name.
- **WARNING 2** (cell -2): dispatch wiring (flag parsed → actually calls the function → exits on its code) isn't independently checked at cell -2's own gate. **Accepted as-is**: cells -4/-5 run the real binary with the flag and grep stdout for `agent_presets`, which fails if dispatch is missing — the gap is caught one wave later, before the slice caps.

## Cell Review Findings (cold pickup)

- **Cell -1**: COLD-PICKUP READY.
- **Cell -2**: was CRITICAL (same vacuous-verify issue as plan-checker WARNING 1). **Fixed** — see above; MINOR note added about the `Args` test literal at `main.rs:469-478` needing the new field (self-caught by `cargo build`, now mentioned).
- **Cell -3**: was MINOR (unpinned test name, same vacuous-pass class). **Fixed** — see above.
- **Cell -4**: COLD-PICKUP READY.
- **Cell -5**: was CRITICAL — `Out-File -Encoding utf8` writes a UTF-8 BOM under Windows PowerShell 5.1; `serde_json` does not skip it, so `config.json` could fail to parse at first service start; unprovable locally (no `pwsh` in this environment). **Fixed**: action now mandates `[System.IO.File]::WriteAllText($ConfigFile, $defaultConfig, (New-Object System.Text.UTF8Encoding($false)))`, mirroring the file's own existing no-BOM pattern; verify bans `Out-File` and requires `UTF8Encoding($false)` to appear. MINOR (exact-whitespace negative grep is fragile but currently correct) accepted as-is.
- **Cell -6**: COLD-PICKUP READY.

All CRITICAL flags fixed before this report. No CRITICAL flags remain open.

## Approval

Gate 3 (execution) — auto-approved under gate-bypass level `full` (every lane, high-risk/hard-gate included; not applicable here — `standard`, no hard-gate flag). Decision and audit trail: `.bee/decisions.jsonl`, tag `dedupe-default-config-templates`.

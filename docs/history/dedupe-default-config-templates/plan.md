---
artifact_contract: bee-plan/v1
mode: standard
approved_gate2: 2026-07-22
---

# Plan: Dedupe Default Config Templates

Mode: `standard` — 2 risk flags: cross-platform (Linux/macOS/Windows install scripts), multi-domain (Rust + bash + PowerShell + a static JSON sample)
Why this is the least workflow that protects the work: touches 6 files across 3 languages and a scheduled-task/systemd/launchd surface — too wide for `small`'s 3-file cap, but it is one cohesive dedup with no incremental user-demoable milestones, so an epic map with a single current slice (no future phases) fits better than forcing artificial phases.

## Requirements (from CONTEXT.md)

- D1: Scope is 5 duplicate/incomplete default-config artifacts (D1 named 4; D7 added a 5th found mid-planning): `config::default_config_json` (canonical), `doctor::default_config_json`, `install.sh`'s literal, `install.ps1`'s literal, `config.example.json`.
- D2: Canonical source stays `config::default_config_json` (`src/config/mod.rs`). Every other emitter must produce byte-identical content — directly (in-crate) or indirectly (installers, via the binary).
- D3: `doctor`'s local `default_config_json` is deleted; its call site (`checks.rs:562`) calls `config::default_config_json` directly.
- D4: `install.sh`/`install.ps1` obtain canonical JSON from the freshly-downloaded binary itself via a new CLI branch, capturing its stdout — same idempotent only-if-missing guard, same timing.
- D5: Fixes only the fresh-install/corrupt-recreate write paths; no backfill of already-installed machines (PBI-041 stays separate).
- D6: This closes a real functional gap (agent_presets never seeded via any installer today), not just DRY cleanup.
- D7: `config.example.json` also gets `agent_presets` added by hand (static doc sample, not wired through the CLI branch).

## Discovery

L0 — no new pattern needed. The exact mechanism D4 calls for already has a precedent in this codebase: `--internal-merge-config` (`src/main.rs:29-32,86,168-171`) is an existing hidden, self-exec-only CLI verb never documented in `print_help()`, dispatching into `herdr_go::config::merge::run_internal_merge_config` and returning an exit code via `std::process::exit`. The new `--internal-print-default-config` branch follows this exact shape. Root-selection (`home().join("projects")` if it exists, else `home()`) is today inlined in both `ensure_config` (`mod.rs:867-868`) and doctor's soon-to-be-deleted duplicate (`checks.rs:903-908`) — extracting it once (cell -1) resolves CONTEXT.md's deferred planning question about that near-duplicate too, and lets the new CLI branch reuse it rather than inlining a third copy.

## Approach

**Recommended path** (cites D2/D3/D4/D7): extract a shared root-resolution helper once (cell -1) → add the hidden CLI branch that prints canonical JSON (cell -2) → repoint doctor's corrupt-recreate path at the canonical function and add real integration coverage for a branch that currently has none (cell -3) → repoint `install.sh` and `install.ps1` to capture the new CLI branch's stdout instead of hand-writing JSON (cells -4, -5) → add `agent_presets` to `config.example.json` by hand (cell -6). Order follows the dependency chain: -2 needs -1's helper; -4/-5 need -2's CLI branch to exist; -3 and -6 are independent of the installer cells.

**Rejected alternatives:**
- Delete the installers' config-write block, rely on `ensure_config` at first service start — rejected at D4: breaks the "config.json exists right after install" guarantee on Linux, where `install.sh` enables but does not auto-start the systemd unit.
- CI-only drift check comparing hand-written literals against the Rust source — rejected: keeps 3 literals alive and merely catches drift after the fact instead of removing the duplication itself, which is what PBI-045 actually asks for.
- Generate `install.sh`/`install.ps1` themselves at release-build time from a template — rejected: heavier (a codegen step in the release pipeline) for no benefit over calling the already-downloaded binary directly, which both installers already have on disk at the point they write `config.json`.

**Risk map:**

| Component | Risk | Reason | Proof needed |
|---|---|---|---|
| New hidden CLI branch (`--internal-print-default-config`) | LOW | Direct copy of the already-proven `--internal-merge-config` shape | Unit test comparing its stdout to `default_config_json`'s own output; `--help` omission check |
| `doctor`'s corrupt-recreate path | MEDIUM | Currently has **zero** integration test coverage of the unparseable-JSON branch (`checks.rs:555-570`) — only the local helper had a unit test, now deleted | New test driving `offer_config_fix` through the actual unparseable-JSON branch, per cell -3 |
| `install.sh`/`install.ps1` edits | MEDIUM | Can't run either script end-to-end locally (no network/systemd/no `pwsh` in this environment) | Static contract checks (negative-grep the old literal, positive-grep the new flag call) + a functional simulation of just the replaced line against a temp HOME; full real-world coverage already exists in CI (`release.yml`'s "macOS install.sh runtime smoke" and "Real install.ps1 -> Scheduled Task..." jobs run the actual scripts against a published release) |
| `config.example.json` | LOW | Static JSON field addition, no code path | `jq`/parse validity + field presence |

## Shape — epic map

**Feature outcome:** every default-config emitter in the repo produces (or, for the doc sample, documents) the same field set, with `agent_presets` no longer silently missing from 4 of the 5.

**Repo-reality basis:** `config::default_config_json` (`src/config/mod.rs:851-863`) is already the established canonical source (per `self-update-merge-config` D5); `--internal-merge-config` already proves the "hidden CLI verb the installer/updater calls into" shape works in this codebase; `release.yml` already runs real install.sh/install.ps1 smoke jobs against published releases, so this plan does not need to invent new e2e infrastructure.

| Epic | Capability/Risk Area | Why It Exists | Slices | Proof Needed |
|---|---|---|---|---|
| E1 | Single-source default config | Closes PBI-045: 5 emitters drift today because 4 of them hand-roll JSON instead of reusing the canonical one | 1 (current) | All 6 cells capped green; full `verify` command green |

**Slice queue:** one slice, no deps, feasibility already established by the discovery/approach above (no spike needed — every step reuses a proven in-repo pattern).

**Current slice to prepare:** E1's only slice — cells -1 through -6 below.

## Test matrix

One pass over all 12 edge dimensions (standard depth):

| Dimension | Applies | Note |
|---|---|---|
| Happy path | yes | Fresh install (all 3 OSes) and doctor corrupt-recreate all produce a config.json with `agent_presets` |
| Empty/missing input | yes | `HOME` with no `projects` subdir — root falls back to `home()` itself (existing behavior, preserved by cell -1) |
| Boundary values | no | No numeric/size boundaries involved |
| Invalid/malformed input | yes | Unparseable existing config.json — doctor's corrupt-recreate branch (cell -3), now with real test coverage for the first time |
| Concurrency | no | Single-process, single-write, no concurrent writers to config.json in scope |
| Idempotency | yes | Both installers' only-if-missing guard must be unchanged — re-running never overwrites an existing config.json (cells -4/-5 must_haves) |
| Permissions/auth | no | No auth surface touched |
| Cross-platform | yes | The core risk flag: Linux (`install.sh` systemd branch), macOS (`install.sh` Darwin branch), Windows (`install.ps1`) all touched |
| Backward compatibility | yes | D5: already-installed machines are explicitly out of scope, not migrated |
| Failure/rollback | yes | New CLI branch nonzero exit must not leave a partial/corrupt config.json — installers must `die`/fail rather than write empty output |
| Observability | no | No logging/metrics surface changes beyond existing installer `say`/`warn` messages |
| Resource limits | no | Not applicable — string formatting only |

## Out of scope

- Backfilling `agent_presets` into already-installed machines that haven't run `herdr-go update` (PBI-041, stays separate per D5).
- Any change to `config.json`'s schema, field names, or values beyond adding the already-existing `agent_presets` field where missing.
- Building new CI e2e installer smoke infrastructure — `release.yml` already runs real `install.sh`/`install.ps1` smokes against published releases; this plan only adds cheap local static/functional checks for the pre-merge `verify` command.

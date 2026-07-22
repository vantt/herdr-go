---
artifact_contract: bee-implement-plan/v1
feature: self-update-merge-config
lane: high-risk
status: Approved
updated: 2026-07-22
sources: [CONTEXT.md, approach.md, plan.md]
decisions: [D1, D2, D3, D4, D5, D6, D7, D8, D9, D10]
---

# Implementation Plan: herdr-go update (self-update + config merge)

> Human-layer projection of the truth artifacts. Truth lives in CONTEXT.md
> (decisions), plan.md + cells (work), and the validating report (evidence).
> Feedback on this document flows back to those artifacts, then this re-renders.

## 1. Goal

A single `herdr-go update` command takes a running installation from its current version to the latest published GitHub release, with no manual steps and no need to read repository instructions — end-user convenience was the explicit motivation.

**Success looks like**
- Running `herdr-go update` with a newer release available downloads, verifies, installs, merges config, restarts, and confirms health — all automatically (D1-D4).
- A config file from an older version gains any new fields the latest version defines, with every existing user value left untouched (D5-D7).
- A corrupt or unverifiable release is refused outright rather than silently installed (D8, D10).
- A release that installs but fails its post-restart health check is not left running broken — it is automatically reverted (D9).

## 2. Current State

- `install.sh` already does an equivalent download+swap manually: resolves `releases/latest` for `vantt/herdr-go`, detects OS/arch, downloads and unpacks the matching asset, and overwrites the binary — with no checksum verification and no running-process check (`install.sh:132-147`).
- `ensure_config` (`src/config/mod.rs:846-869`) only ever writes a default config when no file exists yet; an existing file, however old, is loaded untouched today — this is the exact gap the merge step fills.
- `herdr-go service restart` (PBI-033, `checks.rs:366-459`) is a working, thin pass-through to the native service manager (systemd/launchd/Windows) but has no built-in post-restart health check.
- `/api/health` already exists and reports the running build fingerprint (`src/lib.rs:19-22`).
- No release asset published by this repo's CI has a checksum today (`.github/workflows/release.yml` has zero hash-generation step for herdr-go's own assets — confirmed by direct inspection, not assumed).
- `reqwest` and `tokio` are already dependencies, already used elsewhere in the binary (`main.rs:138,299`) — no new HTTP/async dependency is required.

## 3. Scope

**In scope**
- Fetching the latest release and comparing it against the running version (D1, D2).
- Downloading the release asset and verifying its checksum, refusing when unverifiable (D8, D10).
- Adding a checksum-generation step to the release CI pipeline, since none exists (prerequisite for D8/D10).
- Merging the new version's default config fields into the user's existing config, with a backup taken first (D5, D6, D7).
- Swapping the binary, auto-restarting the service, and confirming health (D3, D4).
- Automatically rolling back binary + config if the post-restart health check fails (D9).

**Out of scope**
- Version pinning / choosing a specific release to install (D1) — always latest only.
- Rename-mapping for config fields renamed between versions (D6) — orphan fields are left as-is.
- Reconciling the 3 pre-existing duplicated default-config templates (`ensure_config`, doctor's `default_config_json`, `install.sh`'s literal) — tracked separately as `docs/backlog.md` PBI-044.
- A future settings-form UI built on the merged config file — stated future direction, no code here.
- Fixing the pre-existing "Windows service restart unproven against a real Scheduled Task" gap — inherited risk, not addressed by this feature.

## 4. Proposed Approach

Build a new `update` CLI verb, matching the existing manual-dispatch pattern used by `doctor`/`service` (`main.rs:70-87,160-164`), inside a new `src/update/` module. Order: (1) publish release-asset checksums from CI — currently missing entirely; (2) fetch the latest release and compare versions (D1, D2) via the GitHub REST API; (3) download the matching asset and verify its checksum, failing closed when none is published (D8, D10); (4) merge the new version's `ensure_config` default JSON into the user's existing config, backing it up first (D5-D7); (5) swap the binary, trigger the existing `service restart` (D3), poll `/api/health` (D4), and roll back binary + config on failure (D9).

**Why this approach** — reuses every existing mechanism that already works (`service restart`, `/api/health`, `ensure_config`'s default, `backup_and_recreate`, `reqwest`/`tokio`) rather than reinventing them, and isolates the one genuinely new piece of infrastructure (checksum publishing) as its own first, lowest-risk slice.

**Alternatives considered**
- Shelling out to `sha256sum`/`Get-FileHash` instead of a Rust `sha2` crate — rejected: per-OS command differences (macOS ships `shasum`, not `sha256sum`), extra process spawn.
- Per-asset `.sha256` sidecar files instead of one `checksums.txt` per release — rejected: one upload step and one file to fetch/parse beats guessing a sidecar filename per platform asset.
- A full `semver` crate for version comparison — deferred pending validating's confirmation of the actual release-tag naming convention, not locked yet.
- A separate fetched "default config template" file for the merge step — rejected per D5; the new binary's own compiled `ensure_config` default is the only source.

## 5. Technical Design

```text
user runs `herdr-go update`
  -> GitHub Releases API (GET latest release for vantt/herdr-go)
  -> compare semver prefix of running herdr_go::VERSION vs. latest tag (D2)
       -> already current: report "up to date", exit (no further steps)
       -> newer available:
            -> download matching platform asset + checksums.txt
                 -> no checksum entry for this asset: refuse, exit (D10)
                 -> checksum mismatch: refuse, exit (D8)
                 -> checksum verified: continue
            -> stop the service (existing service-stop mechanism)
            -> backup + swap the binary
            -> self-exec the NEWLY-SWAPPED binary with a hidden internal verb
               (`--internal-merge-config <path>`) so the config merge (D5-D7)
               runs under the NEW binary's own compiled defaults, not the old
               process's — corrected 2026-07-22 during EP5 planning, see
               approach.md "EP5 design correction"; a merge failure here is
               non-fatal (the original config is never touched on failure)
            -> start the service (existing service-start mechanism)
            -> poll /api/health (D4)
                 -> healthy: report success, done
                 -> unhealthy: restore previous binary + previous (backed-up)
                    config, start again, report the failure (D9)
```

**Data model** — no database; the only "stored element" affected is `config.json` on disk. The merge is additive-only for missing fields (D5, D6); the existing file's values are never rewritten. A timestamped backup of the pre-merge file is created before any write (D7), giving a natural undo path independent of D9's automated rollback.

**API / contract** — consumes the public, unauthenticated GitHub Releases REST API (`GET /repos/vantt/herdr-go/releases/latest`) and the existing local `/api/health` endpoint. No new endpoint is exposed by this feature; `update` is a CLI verb, not an API route.

**Security / Permissions** *(mandatory, high-risk lane)* — this feature replaces a running system binary and merges a local config file based on an unattended download from an external provider (GitHub). The checksum-verify-then-fail-closed path (D8, D10) is the sole integrity gate before any overwrite — there is no additional signature/GPG verification in scope, and the release pipeline itself becomes a trust boundary (whoever can push to the release workflow can affect what `update` installs; this is an accepted, pre-existing trust boundary for `install.sh` too, not new). The config backup (D7) exists specifically to bound the blast radius of a bad merge. No credentials are required for the GitHub API call (public repo, unauthenticated); no secrets are logged by any step in this flow.

## 6. Affected Files

**Current slice (EP1) — projected from cells `self-update-merge-config-1`, `self-update-merge-config-2`:**

| Action | File / Component | Purpose |
|--------|------------------|---------|
| Create | `scripts/generate-checksums.sh` | Standalone checksum-generation script for release assets (D8, D10) — cell `self-update-merge-config-1` |
| Modify | `.github/workflows/release.yml` | Wire checksum generation + publishing into every asset-producing job (D8, D10) — cell `self-update-merge-config-2` |

**EP2 — projected from cells `self-update-merge-config-3`, `self-update-merge-config-4`:**

| Action | File / Component | Purpose |
|--------|------------------|---------|
| Create | `src/update/mod.rs` | Version-compare logic, no network (D2) — cell `self-update-merge-config-3` |
| Create | `src/update/github.rs` | GitHub Releases API fetch, injectable transport (D1, D2) — cell `self-update-merge-config-4` |

**EP3 — projected from cells `self-update-merge-config-6`, `-7`, `-8`:**

| Action | File / Component | Purpose |
|--------|------------------|---------|
| Modify | `Cargo.toml` | Promote `sha2` (already transitive) to a direct dependency — cell `-6` |
| Create | `src/update/checksum.rs` | sha256 hex digest + checksums.txt parser (D8, D10) — cell `-6` |
| Create | `src/update/asset.rs` | Platform release-asset filename selection (D1) — cell `-7` |
| Modify | `src/update/github.rs` | Assets list, `find_asset`, `download_and_verify` with fail-closed (D8, D10) — cell `-8` |

**EP4 — projected from cells `self-update-merge-config-11`, `-12`, `-13`:**

| Action | File / Component | Purpose |
|--------|------------------|---------|
| Modify | `src/config/mod.rs` | Extract `default_config_json` helper (D5) — cell `-11` |
| Create | `src/config/merge.rs` | Pure additive merge (D5, D6) + compose with backup (D7) — cells `-12`, `-13` |

**EP5 — projected from cells `self-update-merge-config-14`, `-15`, `-16`:**

| Action | File / Component | Purpose |
|--------|------------------|---------|
| Modify | `src/main.rs` | Hidden `--internal-merge-config` verb (D5, D7) — cell `-14` |
| Modify | `src/config/merge.rs` | `run_internal_merge_config` — cell `-14` |
| Create | `src/update/swap.rs` | Binary backup + atomic swap (D3, D9) — cell `-15` |
| Create | `src/update/rollout.rs` | Stop/swap/merge/start/health/rollback orchestration (D3, D4, D9) — cell `-16` |
| Modify | `src/main.rs` | Wire the `update` verb into CLI dispatch + help text |
| Create | `scripts/update-smoke.sh` | End-to-end smoke test, mirroring `scripts/macos-install-smoke.sh` |

## 7. Implementation Steps

**EP1 — Release-checksum publishing (current slice, cells prepared and ready to execute)**
- [ ] Standalone checksum-generation script + local test (D8, D10) (`self-update-merge-config-1`)
- [ ] Wire the script into `release.yml` for every asset-producing job (D8, D10) (`self-update-merge-config-2`, deps: `self-update-merge-config-1`)

**EP2 — Version awareness (current slice, cells prepared and ready to execute)**
- [ ] Version-compare logic, no network (D2) (`self-update-merge-config-3`)
- [ ] Fetch latest GitHub release metadata (D1, D2) (`self-update-merge-config-4`, deps: `self-update-merge-config-3`)

**EP3 — Checksum-gated download (current slice, cells prepared and ready to execute)**
- [ ] Checksum computation + checksums.txt parsing, no network (D8, D10) (`self-update-merge-config-6`)
- [ ] Platform release-asset name selection, no network (D1) (`self-update-merge-config-7`)
- [ ] Download release asset + checksum verify with fail-closed (D8, D10) (`self-update-merge-config-8`, deps: `-6`, `-7`)

**EP4 — Config merge (current slice, cells prepared and ready to execute)**
- [ ] Extract `default_config_json` helper, pure refactor (D5) (`self-update-merge-config-11`)
- [ ] Pure additive-only config merge function (D5, D6) (`self-update-merge-config-12`)
- [ ] Compose merge with backup-before-write (D7) (`self-update-merge-config-13`, deps: `-11`, `-12`)

**EP5 — Restart, health, rollback (current slice, cells prepared and ready to execute; sequencing corrected — see approach.md's "EP5 design correction")**
- [ ] Hidden internal config-merge CLI hook (D5, D7) (`self-update-merge-config-14`)
- [ ] Binary backup and atomic swap (D3, D9) (`self-update-merge-config-15`)
- [ ] Orchestrate stop/swap/merge/start/health-check with rollback (D3, D4, D9) (`self-update-merge-config-16`, deps: `-14`, `-15`)

**Queued (not yet cells — later slices per `plan.md`'s slice queue)**
- [ ] EP6 / S8-S9: `update` verb wiring + end-to-end smoke test (D1-D9 integration)

## 8. Validation Plan

**Automated** — EP1's cells each carry their own `verify` command (a local script test for S1; a YAML-validity + non-tautological wiring check for S2) → expected: both pass before capping. Later epics' verify commands are cut when their slices are prepared.
**Manual** — [ ] a real end-to-end smoke run (`scripts/update-smoke.sh`, EP6/S9) against a tagged release before this feature is trusted in production, mirroring the rigor of the existing `scripts/macos-install-smoke.sh` / `windows-install-smoke.ps1` post-publish jobs.
**Evidence** — `bee-validating` ran against EP1: READY (1 CRITICAL + 1 MINOR found and repaired, `docs/history/self-update-merge-config/reports/validation-ep1.md`), shipped and capped, full repo verify green. EP2: READY (2 MINOR found and repaired), `docs/history/self-update-merge-config/reports/validation-ep2.md`, shipped and capped, full repo verify green. EP3 (security-critical D8/D10 slice, security persona panel): READY (0 BLOCKER/CRITICAL, verify commands tightened to force fail-closed test coverage, EP5 carry-forward invariant recorded), `docs/history/self-update-merge-config/reports/validation-ep3.md`. EP4: READY (0 BLOCKER/CRITICAL, added a proving test for a deny_unknown_fields/D6 interaction), `docs/history/self-update-merge-config/reports/validation-ep4.md`. EP5 (highest-risk epic, safety/reliability persona panel): READY (1 CRITICAL instruction-level defect fixed — wrong crate-path prefix; retry budget pinned; rollback exit code now captured), `docs/history/self-update-merge-config/reports/validation-ep5.md`.

## 9. Risks & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| No checksum infra exists anywhere in the release pipeline today | Medium | EP1 builds and proves this in isolation, as CI-only work, before any Rust code depends on it |
| `herdr_go::VERSION` is a composite fingerprint, not a bare semver; exact release-tag naming convention unconfirmed | Medium | Validating confirms the actual tag format before S3's version-parse logic is locked |
| New Rust-side checksum-verify path must correctly refuse, not silently proceed, when unverifiable | Medium | Unit tests for present+valid / present+invalid / absent, each asserting the expected path (D8, D10) |
| Config merge could lose or alter a user-owned field | Low | Additive-only merge (D5, D6) + mandatory backup before write (D7), mirroring already-tested `write.rs:407-437` patterns |
| Binary swap + restart + rollback has no existing precedent; a bad rollback could leave the service worse off than before `update` ran | High | Unit tests via the existing injectable-closure pattern (`checks.rs:793-817`) plus a real end-to-end smoke script (EP6/S9) before production trust |
| Windows service restart is pre-existing-unproven against a real Scheduled Task | Medium (inherited) | Out of this feature's scope to fix; flagged, not newly introduced |

## 10. Rollback Plan

**Rollback of this development work** (reverting the feature's own code, not to be confused with the feature's D9 operational rollback described in §5): each slice (S1-S9) is capped as its own commit per AGENTS.md's one-commit-per-cell rule. Reverting the feature is `git revert` of the relevant slice commits, in reverse dependency order (EP6 → EP5 → EP4 → EP3 → EP2 → EP1), since later epics depend on earlier ones' code. No database migration exists to reverse. EP1 (the CI workflow change) can be reverted independently and safely at any time — it has zero runtime dependents until EP3 consumes it.

**The feature's own operational rollback (D9)** is not a "how do we undo this dev work" mechanism — it is product behavior: if `update`'s post-restart health check fails in the field, the feature itself restores the previous binary + config and restarts again automatically, as described in §5's flow.

## 11. Open Questions

- Exact naming convention of existing GitHub release tags (`vX.Y.Z` vs `X.Y.Z`) — needed to lock S3's version-parse implementation; deferred to validating.
- Whether a cross-job `checksums.txt` merge (Linux/macOS matrix + separate Windows job) is straightforward in this repo's existing GitHub Actions structure, or whether two separate per-OS-job-group checksum files is the pragmatic fallback — deferred to EP1/S2's execution and validating's feasibility check.
- Whether unauthenticated GitHub API rate limits could meaningfully affect real-world `update` usage — minor, not yet assessed.

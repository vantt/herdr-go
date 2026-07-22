---
area: self-update
updated: 2026-07-22
sources: [self-update-merge-config]
decisions: [ec642cce-6b5b-4a77-a1d9-1977f2cf4819, aae8be8b-2b10-441f-a8ab-e178e16e676c, 1cf50ada-8545-4ddf-a360-a413c43a2f70, ad8dd610-5161-4a38-aff9-29c1cce4f1b4, 50fa8fe5-6849-4e62-8f6e-0eb86613e4cf, a2e00a6b-6860-49af-8cf1-5ea9fe05e8cc, dc6fe096-b691-447e-8138-82e9c09cd911, be8f0d8a-f762-4f0e-8a62-a61b76565c55, f074a86b-4e53-4f80-ba4a-e51b258c5a6d, 10f5961f-593f-4846-b9bf-54397b02e7ac, 1ac60a90-7762-44b1-9bae-0aff365499df]
coverage: partial
---

# Spec: Self-Update

How an already-installed operator takes their running copy from its current
published version straight to the latest one, with no manual download, no
config editing, and no need to remember the service-restart command. This
area covers the `update` command's whole lifecycle end to end: checking
whether a newer version exists, safely downloading and verifying it,
carrying forward the operator's settings into the new version's shape, and
bringing the service back up healthy — reverting cleanly if it doesn't. It
does not cover the first-time install (see `installation`) or the
`service`/`doctor` commands this area reuses rather than replaces (see
`service-lifecycle`).

## Entry Points & Triggers

- `herdr-go update` → checks for a newer published version and, if one
  exists, updates to it end to end (download, verify, install, carry forward
  settings, restart, confirm health).
- A hidden, undocumented step of the update itself re-invokes the
  just-installed program in a special internal-only mode to carry settings
  forward under the new version's own rules — this is never something an
  operator runs directly; it exists only so the update command can complete
  correctly (see Behaviors & Operations, "Carry forward settings").

## Data Dictionary

| # | Element | Meaning | Values | Required | Default |
|---|---------|---------|--------|----------|---------|
| 1 | Latest published version | The newest version publicly available to update to | any published version | — | — |
| 2 | Update outcome | What the operator is told happened | "already up to date" · "updated successfully" · "update failed, nothing changed" (see R7/R8) | — | — |
| 3 | Verification proof | The evidence the downloaded copy is genuine and untampered before it replaces the running program | a published integrity value matched against the download | yes for every update | — |
| 4 | Carried-forward setting | An existing setting in the operator's configuration that a new version might add, not already present | any setting the new version defines that the operator's file doesn't have yet | — | the new version's own default value for that setting |
| 5 | Health confirmation | Proof the newly-updated service is actually running correctly, not just installed | a successful response from the service's own health signal | yes before an update is called successful | — |

## Behaviors & Operations

### Check for and apply an update

- **Triggers:** operator runs `herdr-go update`.
- **Blocked when:** no existing installation's configuration can be found —
  this command carries forward an *existing* operator's settings, it never
  sets a fresh installation up (see `installation` for that path).
- **What changes when already up to date:** nothing — the operator is told
  their version is already current and no download, verification, or
  service change happens.
- **What changes when a newer version exists:** the newer version's program
  is downloaded and verified (see "Verify before installing"), the
  operator's configuration is carried forward under the new version's shape
  (see "Carry forward settings"), the running program is stopped, swapped
  for the new one, and started again, and the new version's health is
  confirmed.
- **Side effects:** the service is briefly unavailable while it restarts on
  the new version — the same brief interruption a manual restart already
  causes.
- **Afterwards:** the operator sees one of the three outcomes in Data
  Dictionary #2. On success, the newly running version answers to health
  checks and reflects the operator's carried-forward settings. On failure,
  the operator is told the update did not complete and nothing about their
  running installation changed for the worse (see R8).

### Verify before installing

- **Runs when:** a newer version is found and before any part of the
  running installation is touched.
- **Blocked when:** the newer version has no integrity proof published for
  it yet, or the downloaded copy's proof doesn't match what was expected —
  either way, the update stops here. Nothing about the running installation
  changes (R5/R6).
- **What changes:** nothing on the operator's machine until verification
  passes; only after it passes does anything proceed.
- **Side effects:** none beyond the check itself.
- **Afterwards:** either the operator is told the update could not be
  verified and nothing changed, or the verified copy proceeds to
  installation.

### Carry forward settings

- **Runs when:** a verified newer version is about to become the running
  program.
- **What changes:** any setting the new version defines that the operator's
  existing configuration doesn't already have is added, using the new
  version's own default value for it. Every setting the operator already
  had keeps the operator's own value, unchanged, no matter what the new
  version's default for that setting would have been. A setting the
  operator's file has that the new version no longer recognizes is left in
  place untouched — nothing is renamed or removed automatically (R3, R4).
- **Side effects:** a backup copy of the operator's configuration, exactly
  as it was right before this step, is kept before any change is written.
- **Afterwards:** in the ordinary case, the operator's configuration now
  also has whatever new settings the new version introduced, with their
  defaults, and everything the operator had already set is exactly as it
  was before. In the rare case where the operator's file has a setting so
  old the new version can no longer make sense of it at all, this step
  changes nothing and the update continues with the operator's
  configuration completely untouched (R4a) — the new version simply runs
  with whatever it can already understand from that file.

### Confirm health and recover from a failed update

- **Runs when:** immediately after the new version's service has been
  started.
- **Blocked when:** the new version's service does not answer a health
  check within a bounded waiting period.
- **What changes on success:** nothing further — the update is complete.
- **What changes on failure:** the previous version's program and the
  operator's pre-update configuration are put back exactly as they were,
  and the service is started again on that previous version.
- **Side effects:** the service restarts a second time during a failed-update
  recovery.
- **Afterwards:** on success, the operator is running the new version,
  confirmed healthy. On failure, the operator is running the same version
  and configuration they had before running `update` at all, and is told
  the update did not succeed (R8). Whether that final recovery restart
  itself succeeded is recorded alongside the failure so a second, deeper
  failure is never silently reported as a clean recovery.

## Actors & Access

Single-operator system — whoever can already run commands on the machine
(and therefore already has full control of the installation) can run
`herdr-go update`; there is no separate authentication or authorization
layer for it.

## Business Rules

- **R1.** `update` always targets the single latest published version —
  there is no way to choose or pin a different, older version to update to
  (per D `ec642cce`).
- **R2.** A published version that is the same as or older than the
  currently running one is never treated as an update — `update` only ever
  moves forward, never sideways or backward (per D `aae8be8b`).
- **R3.** A setting already present in the operator's configuration is never
  overwritten by carrying settings forward, regardless of what the new
  version's own default for that setting is (per D `50fa8fe5`).
- **R4.** Carrying settings forward never removes or renames a setting the
  new version no longer recognizes — it is left exactly as the operator had
  it (per D `a2e00a6b`).
- **R4a.** If the operator's configuration contains a setting so unfamiliar
  to the new version that the new version cannot make sense of the file at
  all, carrying settings forward changes nothing and the operator's file is
  left completely untouched — this is a currently unreachable case (no
  setting has ever been retired from a published version), but the rule
  holds regardless (per D `a2e00a6b`, tightened during implementation).
- **R5.** A verified download always happens before the running program is
  ever replaced — there is no path where an unverified copy is installed
  (per D `be8f0d8a`).
- **R6.** A newer version with no integrity proof published for it yet is
  never installed with a warning instead — `update` refuses outright and
  changes nothing (per D `10f5961f`).
- **R7.** The service is restarted automatically as part of a successful
  update, with no confirmation prompt — the operator never has to remember
  or run the restart command themselves (per D `1cf50ada`).
- **R8.** A failed update always restores the previous program and
  configuration and restarts the service on them, rather than leaving a
  broken or half-updated installation running (per D `f074a86b`).
- **R9.** Carrying settings forward always happens under the version that
  will actually run afterward, never under the version that is being
  replaced — this is what makes R3/R4's "the new version's own default"
  language correct rather than stale (per D `1ac60a90`).

## Edge Cases Settled

- Already running the latest published version → `update` reports up to
  date and makes no change of any kind.
- A published version exists but has no integrity proof yet → treated the
  same as "cannot verify," update refuses rather than installing anyway
  (R6).
- The operator's configuration has a setting the new version no longer
  understands at all → nothing about carrying settings forward is applied;
  the file is left exactly as it was (R4a).
- The new version's service does not come up healthy after restarting →
  automatic recovery restores the previous program and configuration and
  restarts again, rather than leaving the operator on a broken new version
  (R8).

## Open Gaps

- End-to-end proof of the whole `update` lifecycle against a real published
  release has not yet run — it is verified today by unit-level proof of
  each step, plus a smoke test wired into the release pipeline that has not
  yet executed against a real release (the next tagged release is its first
  live run). Until then, the whole-lifecycle claim rests on the individual
  steps' own proof plus structural review of the smoke test itself.
  Restoring the previous program on a machine whose current install is
  Windows has never been exercised for real — the underlying platform is
  known to keep a running program's file open in a way this feature has not
  yet proven a way around, so a failed-update recovery on Windows is an
  explicitly unproven path today (inherits, and adds a new instance of, the
  same "Windows unproven under real conditions" gap already recorded for
  installation and service-lifecycle).
- Health confirmation after a failed-update recovery restart is not itself
  re-checked — the recovery restart's own outcome is at least recorded
  alongside the failure report, but a second failure during recovery is not
  automatically retried or escalated further.
- No way exists yet for the operator to see progress mid-update (e.g. "now
  downloading," "now verifying") — the command either completes or fails,
  reported only at the end.

## Pointers (implementation)

- `src/update/mod.rs` — top-level `run()` orchestration entry point;
  `SemVer`/`UpdateStatus`/`compare` (Business Rules R1/R2).
- `src/update/github.rs` — `check_for_update`, `download_and_verify`,
  `find_asset`, checksum-gated download composition (R5, R6).
- `src/update/checksum.rs` — SHA-256 computation and checksum-manifest
  parsing.
- `src/update/asset.rs` — platform-to-release-asset name mapping.
- `src/update/swap.rs` — binary backup + atomic swap.
- `src/update/rollout.rs` — `perform_update`: stop/swap/carry-settings/
  start/health-poll/rollback composition (R7, R8, R9).
- `src/config/merge.rs` — `merge_config_on_upgrade`, `merge_missing_fields`,
  `run_internal_merge_config` (R3, R4, R4a); the hidden internal verb
  `--internal-merge-config` this last function backs, so settings are
  always carried forward under the newly-installed version's own compiled
  defaults, never the version being replaced's (R9).
- `src/main.rs` — the public `update` verb and its documented help text;
  the hidden `--internal-merge-config` verb (never documented, never a
  public entry point).
- `.github/workflows/release.yml` — the `checksums` job (publishes the
  integrity proof `update` verifies against, R5/R6) and the `update-smoke`
  job (the not-yet-executed end-to-end proof, Open Gaps).
- `scripts/update-smoke.sh` — the end-to-end proof script itself.
- `scripts/generate-checksums.sh` — computes the published integrity proof
  for every release asset.

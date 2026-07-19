# Validation: local rename, no-clone install, and documentation funnel

## Reality gate

- Mode fit: **PASS** — state migration, service migration, release contracts, public installation, cross-platform boundaries, and documentation span six risk flags.
- Repository fit: **PASS** — every current and planned path is bounded by `rename-herdr-go-1`; required shell/runtime tools are present.
- Assumptions: **PASS WITH CONSTRAINTS** — the real published asset remains unavailable, so local proof uses a synthetic matching archive and network stubs.
- Smaller path: **PASS** — installer, release naming, runtime paths, service identity, and user promises must change together to avoid a broken first run.
- Proof surface: **PASS** — the cell runs formatting, Rust tests/lints, shell checks, a durable contract harness, unit-file assertions, and web bundle/tests.

Decision: **READY WITH CONSTRAINTS**.

## Feasibility matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| Legacy state migrates without content inspection or overwrite | Data loss | Old-only, new-only, both-exist, missing, and failed-move tests | D3; `src/config/mod.rs`; cell truths | Ready with constraints |
| Only the selected service mode can start | Port conflict / outage | Fake-systemctl ordering and idempotency matrix | D4; `install.sh`; `dev-deploy.sh`; contract-test requirement | Ready with constraints |
| Installer and release agree on archive identity | Install failure | Producer/consumer string and extraction-root assertions | D5; release workflow and installer cell key link | Ready |
| A fetched installer needs no checkout | Broken advertised quick start | Copy only `install.sh` into an empty root; fake the release download; assert outputs and unexpected local reads | D7; self-contained installer contract | Ready with constraints |
| User docs progressively disclose detail | Confusion / false promise | README structure, no-clone primary path, local-link, packaged-guide, and platform-claim assertions | D7; bounded README/task/advanced guides | Ready |
| Cell ordering is executable | Deadlock | One wave, zero cycles, zero unsatisfied dependencies | `bee cells schedule` output | Ready |

## Panel findings

The first review found five blockers: the user-doc requirement was uncovered, `install.sh` depended on checkout-local files, verification did not prove the new experience, the documentation scope was underspecified, and platform promises could exceed real installer support. Planning was repaired with D7, a Linux-only self-contained bootstrap, an explicit progressive documentation hierarchy, packaged-link checks, and an empty-directory deterministic installer harness. Recheck found no structural blocker.

Warnings retained:

- A real GitHub release download cannot be proven until a `herdr-go` asset is published; docs must not imply that smoke test already passed.
- The installer harness must copy only `install.sh` into the empty test root and reject unexpected local-template reads.
- The atomic slice is large, but splitting the migration/release/install/documentation contract would allow incompatible intermediate states.

## Cold-pickup cell review

- Cells reviewed: 1
- Critical flags open: 0
- Minor flags open: 0
- Dependency graph: one wave, no cycles
- Result: a worker can execute `rename-herdr-go-1` from the locked context, plan, and cell without guessing.

## Approval block

VALIDATION COMPLETE — READY FOR EXECUTION

- Mode: high-risk
- Work: atomic local rename, compatibility migration, self-contained Linux install, and value-first documentation
- Reality gate: PASS
- Feasibility: READY WITH CONSTRAINTS
- Structure: PASS after one repair iteration
- Spikes: none; deterministic empty-directory proof is specified in the contract test
- Cell review: PASS (1 cell, 0 critical open)
- Unresolved concern: real published-asset smoke test waits for a release

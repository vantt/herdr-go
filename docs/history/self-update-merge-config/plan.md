---
artifact_contract: bee-plan/v1
mode: high-risk
approved_gate2: 2026-07-22
---

# Plan: herdr-go update (self-update + config merge)

Mode: `high-risk` — 5 risk flags: audit/security (checksum verification, self-replacing running binary), external provider (GitHub Releases), cross-platform (systemd/launchd/Windows restart), data model (config.json merge), multi-domain (CLI + config + network + CI release pipeline)
Why this is the least workflow that protects the work: the feature self-replaces a running system binary and its config based on an unattended download from an external provider — a bad merge, a skipped checksum, or a broken rollback path all have real blast radius on a running service, which is exactly the ceremony a `standard` lane would under-protect.

## Requirements (from CONTEXT.md)

- D1: `update` fetches only via GitHub `releases/latest` for `vantt/herdr-go`, no version-pin flag in v1.
- D2: Compare the semver prefix of `herdr_go::VERSION` against the latest release tag; no-op if already current.
- D3: Auto-trigger the existing `service restart` mechanism after binary swap, no confirmation prompt.
- D4: Auto-call `/api/health` after restart, report result in `update`'s own output.
- D5: Merge source-of-truth is the new binary's own compiled `ensure_config` default JSON — no separate template file.
- D6: Field deletion/rename across versions out of scope for v1 — orphan fields left untouched, no warning.
- D7: Back up the existing config file before merging, reusing `backup_and_recreate`.
- D8: Verify release binary checksum before overwrite; requires a release-pipeline change to publish checksums.
- D9: On failed post-restart health check, auto-rollback to previous binary + config, restart again, report failure.
- D10: If the latest release has no published checksum yet, fail closed — refuse to update, no warn-and-proceed fallback.

## Discovery

Findings folded in here (L1 — quick verify, no separate discovery.md): confirmed `reqwest`+`tokio` already present (`Cargo.toml:18,25`, `main.rs:138`) so no new HTTP/async dependency is needed for D1/D2/D8; confirmed **no checksum/hash crate and no checksum-generation step exist anywhere in the repo today** (`.github/workflows/release.yml` has zero hash step for herdr-go's own release assets) — this is new scope, not a hookup, and is why release-checksum publishing is its own first epic below. Full rejected-alternatives and risk detail: `approach.md`.

## Approach

See `approach.md` (graduated to a standalone file per the high-risk lane).

## Shape

**Epic map** (capability/risk-shaped — high-risk default):

Feature outcome: a single `herdr-go update` command that safely takes a running installation from its current version to the latest published release, with no manual steps.

Repo-reality basis: `install.sh` already proves the download/OS-detection/binary-swap mechanics manually; `service restart`/`/api/health` already exist and work; nothing about checksum-verified downloads, config-merge-on-upgrade, or automated rollback exists anywhere in the repo today.

| Epic | Capability/Risk Area | Why It Exists | Slices | Proof Needed |
|---|---|---|---|---|
| EP1 | Release-checksum publishing | D8/D10 hard-depend on a checksum existing to verify — currently zero exist; lowest blast radius (CI-only, no running-service risk) and fully decoupled from the rest, so it can be proven first | S1: standalone checksum-generation script + local test · S2: wire into `release.yml` for every asset-producing job | Local script proof against dummy files (S1); YAML-valid + every asset job wired (S2) |
| EP2 | Version awareness | D1/D2 — must know "is there a newer release" before doing anything else | S3: GitHub Releases API client + semver-prefix compare + no-op-when-current | Unit test against a mocked HTTP response; validating confirms actual tag naming convention first |
| EP3 | Checksum-gated download | D8/D10 — the security-critical gate; must refuse, not warn, when unverifiable | S4: download release asset + `sha2`-based checksum verify + fail-closed path | Unit tests: checksum present+valid → proceeds; checksum present+invalid → refuses; checksum absent → refuses (D10) |
| EP4 | Config merge | D5/D6/D7 — the other half of the feature's value (per user: unlocks a future settings-form built on the merged file) | S5: backup existing config, merge new-version defaults for missing fields only, leave existing/orphaned fields untouched | Unit tests mirroring `write.rs:407-437`'s backup-then-write structure |
| EP5 | Restart, health, rollback | D3/D4/D9 — the HIGH-risk component per `approach.md`'s risk map; a bad rollback is the one failure mode that leaves the service worse off than before `update` ran | S6: binary swap + auto-restart + health poll (D3/D4) · S7: rollback-on-failed-health-check (D9), reusing the swap/restart primitives from S6 | Unit tests via the injectable-closure pattern (`checks.rs:793-817`); a real end-to-end smoke script before this is trusted in production |
| EP6 | Integration and end-to-end proof | Wire the `update` verb into `main.rs` dispatch; prove the whole chain against a real (or pinned-fixture) release, mirroring the existing `scripts/macos-install-smoke.sh` post-publish pattern | S8: `update` verb wiring + help text · S9: `scripts/update-smoke.sh` + CI job | Real smoke run against a tagged release in CI, matching the existing install-smoke jobs' rigor |

**Slice queue:**

1. **S1 → S2 (EP1, no deps)** — current slice, prepared below.
2. S3 (EP2) — deps: none (independently buildable/testable once discovery confirms tag format); does not require S1/S2 to be merged, but D8/D10 cannot be exercised for real until EP1 ships.
3. S4 (EP3) — deps: S3 (needs the download/fetch scaffolding) and, for production use, EP1 shipped (checksums must actually exist to verify against) — testable in isolation via mocked HTTP fixtures before EP1 ships.
4. S5 (EP4) — deps: none functionally (independent of S1-S4); ordered here because it's the other locked-decision half of the feature.
5. S6 → S7 (EP5) — deps: S4 (download+verify must complete before a binary is trusted enough to swap) and S5 (config merge must run before restart, per the feature boundary's stated order).
6. S8 → S9 (EP6) — deps: all of the above; this is the wiring + proof slice.

**Current slice to prepare: EP1 (S1, S2).** Rationale: zero dependency on any other slice, zero running-service risk (pure CI/script work), and it unblocks realistic testing of D8/D10's fail-closed behavior in every later slice.

## Test matrix

Probes scoped to what's material for this feature (high-risk lane — per-dimension probes):

1. **User types** — single-operator CLI tool, no auth/multi-user concept; not material beyond "the person running `update` is the same person who can already run `service restart`" (already true today).
2. **Input extremes** — malformed/truncated JSON from the GitHub API response; a config file that is valid JSON but missing all fields; a config file that is *not* valid JSON at all (existing `Config::load_file` error path — merge must not attempt to run against an unparseable file).
3. **Timing** — `update` interrupted mid-download (partial file) → must not swap a truncated binary (checksum check is exactly this guard, D8); interrupted mid-restart (process killed between stop and start) → covered by inheriting `service restart`'s own semantics (R4, not reinvented).
4. **Scale** — not material (single binary, single config file, no collections).
5. **State transitions** — already-latest-version re-run of `update` (D2 no-op path); `update` run twice in a row after a successful update (second run should again resolve to "already up to date", not attempt to "update" to the same version).
6. **Environment** — no network (GitHub API unreachable) → clear error, no partial state change; missing `sha2` crate build issue N/A (compile-time dependency); Windows path where `service restart` has the pre-existing "unproven against real Scheduled Task" gap (inherited risk, noted in `approach.md`, not newly introduced).
7. **Error cascades** — GitHub API returns non-200 (rate-limited, 404) → `update` reports the error and makes no filesystem changes; checksum mismatch → refuse (same as D10's "absent" case, just a different reason string); restart command itself fails (non-zero exit) → treated as a health-check failure, triggering D9's rollback path.
8. **Authorization** — not material (no multi-tenant/permission model in this CLI).
9. **Data integrity** — the central concern for D5/D6/D7: the merge must never lose or silently alter a user-owned field's value; the backup (D7) must exist and be verifiable before the merged file is written, mirroring `write.rs:407-437`'s exact assertions.
10. **Integration** — GitHub API response shape drift (new/renamed fields in the release JSON) → parse defensively (only read the fields `update` needs, per serde's default "ignore unknown fields" behavior already implied by existing config code); a release published with an asset but no matching checksum entry → D10's fail-closed path, not a crash.
11. **Compliance** — not material (no PII touched; config file is local, not transmitted anywhere by this feature).
12. **Business logic** — boundary: a release tag equal to (not greater than) the running version → D2's no-op, not an "update" (equality is not "newer"); a release tag that is *older* than the running version (e.g. GitHub `releases/latest` misconfigured) should also no-op, never downgrade, in v1.

## Out of scope

- Version pinning / rollback-to-arbitrary-version flag (D1) — always latest only.
- Rename-mapping for config fields renamed between versions (D6) — orphan fields are left as-is, no migration table.
- Reconciling the 3 pre-existing duplicated default-config templates (`ensure_config`, doctor's `default_config_json`, `install.sh`'s literal) — tracked separately as `docs/backlog.md` PBI-044.
- A settings-form UI built on the merged config file — user's stated future direction, no code here.
- Fixing the pre-existing "Windows service restart unproven against a real Scheduled Task" gap (`service-lifecycle.md`) — inherited risk, not introduced or fixed by this feature.

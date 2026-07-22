# Approach: herdr-go update (self-update + config merge)

## Recommended path

Build a new `update` CLI verb (`src/main.rs`, matching the existing manual-dispatch pattern used by `doctor`/`service`, `main.rs:70-87,160-164`) inside a new `src/update/` module, in this order: (1) publish release-asset checksums from CI (prerequisite for D8/D10, currently missing entirely), (2) fetch the latest release + compare versions (D1/D2) via the GitHub REST API (reqwest is already a dependency, no new HTTP crate needed), (3) download the matching asset and verify its checksum, failing closed when none is published (D8/D10), (4) merge the new version's `ensure_config` default JSON into the user's existing config, backing it up first (D5/D6/D7), (5) swap the binary, trigger the existing `service restart` (D3), poll `/api/health` (D4), and roll back binary+config on failure (D9).

## Rejected alternatives

- **Shell out to `sha256sum`/`Get-FileHash` for checksum verification in Rust** — rejected in favor of adding the `sha2` crate: avoids per-OS command differences (macOS ships `shasum`, not `sha256sum`) and an extra process spawn; the project already leans pure-Rust (`rustls-tls` over an OpenSSL binding).
- **Per-asset `.sha256` sidecar files** (one extra uploaded file per platform asset) — rejected in favor of one `checksums.txt` per release (industry-standard convention, e.g. GoReleaser): one upload step, one file for `update` to fetch and parse, instead of guessing a sidecar filename per asset.
- **A full `semver` crate for version comparison** — deferred, not rejected outright: existing release tags are simple `X.Y.Z` (no prerelease/build metadata seen in current tags); flagged as a **question for validating** (confirm actual tag format) rather than locked here, since it changes whether a manual parse is sufficient.
- **A separate fetched "default config template" file for the merge step** — rejected per CONTEXT.md D5; the new binary's own compiled `ensure_config` default is the only source, avoiding a 4th duplicate template.

## Risk map

| Component | Risk | Reason | Proof needed |
|---|---|---|---|
| Release-checksum publishing (CI) | MEDIUM | No checksum/hash step exists anywhere in `release.yml` today for herdr-go's own assets (only for a *third-party* downloaded artifact); the release job is a cross-OS matrix (Linux musl ×2, macOS, Windows as a separate job) — merging one `checksums.txt` across jobs needs artifact-passing (`upload-artifact`/`download-artifact`) that hasn't been proven for this workflow | Cell self-update-merge-config-1 (local script proof) + self-update-merge-config-2 (wiring); validating should confirm the matrix→merge job shape is feasible in GitHub Actions syntax before cells beyond slice 1 are cut |
| GitHub API version fetch (D1/D2) | LOW | reqwest+tokio already used elsewhere in the binary (`main.rs:299` for Cf Access); this is a straightforward JSON GET | Unit test with a mocked/injected HTTP response |
| Version string parsing | MEDIUM | `herdr_go::VERSION` is a composite fingerprint, not a bare semver (`src/lib.rs:19-22`); exact release-tag naming convention (`vX.Y.Z` vs `X.Y.Z`) not yet confirmed | Validating: inspect an actual published release tag name |
| Checksum verification + fail-closed (D8/D10) | MEDIUM | New Rust-side crypto path (`sha2` crate, not yet a dependency); must correctly refuse rather than silently proceed when no checksum asset exists | Unit test: mocked release with/without checksum asset, assert refusal path taken |
| Config merge (D5/D6/D7) | LOW | Directly mirrors existing, already-tested patterns: `repair_fields` (map-diffing shape) and `backup_and_recreate` (backup-then-write, already has two passing tests at `write.rs:407-437`) that a new merge function can pattern-match | Unit tests mirroring `write.rs:407-437`'s structure |
| Binary swap + restart + health + rollback (D3/D4/D9) | HIGH | Self-replacing a running binary, plus a rollback path with no existing precedent (only `offer_service_restart_with`'s injectable-closure pattern exists, and only for restart, not swap/rollback); a bad rollback implementation is the one failure mode that could leave the service in a worse state than before `update` ran | Cell-level unit tests using the injectable-closure pattern (`checks.rs:793-817`) for restart/health, PLUS a real end-to-end smoke script (mirroring `scripts/macos-install-smoke.sh`) before this is trusted against a real deployment |
| Windows service restart path | MEDIUM (pre-existing, inherited) | `service-lifecycle.md` already documents this branch is "proven only by local build/lint... not yet exercised against a real Windows Scheduled Task" — `update`'s D3 reuse inherits that same unproven gap, not a new one this feature introduces | Out of this feature's scope to fix; flag as inherited risk, not a blocker |

## Files and order

1. `scripts/generate-checksums.sh` (new) — standalone, testable in isolation (slice 1, cell 1)
2. `.github/workflows/release.yml` — wire the script in (slice 1, cell 2)
3. `Cargo.toml` — add `sha2` dependency (slice 2)
4. `src/update/mod.rs` (new) — version fetch/compare (D1/D2), download+checksum-verify+fail-closed (D8/D10) (slice 2/3)
5. `src/config/mod.rs` or a new `src/config/merge.rs` — merge function (D5/D6/D7) (slice 4)
6. `src/update/mod.rs` (extended) — binary swap, restart, health check, rollback (D3/D4/D9) (slice 5)
7. `src/main.rs` — wire the `update` verb into dispatch + help text (slice 5)
8. `scripts/update-smoke.sh` (new, mirroring `scripts/macos-install-smoke.sh`) + `.github/workflows/release.yml` smoke job — end-to-end proof (slice 6)

## Relevant learnings

- `docs/history/default-agent-presets/CONTEXT.md` D1/D2 — the "existing config never touched" precedent this feature deliberately extends with a merge step, and the pre-existing template-duplication problem (tracked separately as PBI-044, out of this feature's scope).
- `docs/specs/service-lifecycle.md` R4 — restart/status verbs must stay thin pass-throughs; `update` must call them, not reimplement restart logic.
- `checks.rs:793-817` (`offer_service_restart_with`) — the repo's only existing pattern for unit-testing restart-adjacent logic without a real service manager; reuse its injectable-closure shape for D3/D4/D9's implementation.

## EP3 discovery (2026-07-22, resolved during EP3 planning)

- EP1 shipped a **single merged `checksums.txt`** (confirmed: `release.yml`'s `checksums` job downloads every staged asset via `download-artifact --pattern release-asset-* --merge-multiple`, runs `generate-checksums.sh dist > checksums.txt`, uploads it as one release asset literally named `checksums.txt`). EP3 does **not** need to handle a per-job-group fallback — that path was never taken.
- Release asset naming, confirmed from `release.yml`/`install.sh`: `herdr-go-<target-triple>.<ext>` — `.tar.gz` for `x86_64-unknown-linux-musl` / `aarch64-unknown-linux-musl` / `aarch64-apple-darwin`, `.zip` for `x86_64-pc-windows-msvc`. No other OS/arch combination has a published asset (matches `install.sh`'s own unsupported-combo errors).
- `sha2 = "0.11.0"` is already present as a **transitive** dependency (`Cargo.lock`) — promoting it to a direct `Cargo.toml` dependency adds no new crate to the build.

## EP5 carry-forward invariant (from EP3 validating, 2026-07-22)

D8's "verify **before** overwrite" guarantee is only as strong as EP5's discipline: EP5 (binary swap + restart + rollback) **must** source the binary bytes it installs exclusively from EP3's `download_and_verify()` `Ok(bytes)` result — never add a second, separate raw-download path that bypasses checksum verification. `download_and_verify` carries a doc comment stating this; EP5's cell(s) and validating pass must explicitly confirm this invariant holds before Gate 3 for that slice.

## EP5 design correction (2026-07-22, found during EP5 planning, before any cells cut)

The original Technical Design (implement-plan.md) ordered "merge config -> swap binary -> restart". This is wrong: `update`'s own process is still running the OLD binary right up until restart, so merging at that point would read the OLD binary's compiled `default_config_json` (D5 requires the NEW version's).

**Corrected flow:**
```
stop service (run_service_command("stop"))
  -> backup + swap the binary (parameterized, testable independent of the real running exe)
  -> self-exec the NEWLY-SWAPPED binary with a hidden internal verb
     (`herdr-go --internal-merge-config <config-path>`), capturing its exit code
     and (on success) the config backup path it prints to stdout -- this runs
     merge_config_on_upgrade (EP4) under the NEW binary's own compiled defaults,
     satisfying D5. A merge failure here is non-fatal to the update: per
     backup_and_recreate's existing guarantee, a failed merge never touches the
     original config file, so it's always safe to proceed
  -> start service (run_service_command("start"))
  -> poll /api/health with a bounded retry budget (D4)
  -> on success: done. on failure: restore the backed-up binary AND the
     captured config backup (if any), start again, report the failure (D9)
```

This needs one small, hidden CLI addition in `src/main.rs` (`--internal-merge-config`, never shown in `--help`, never documented as a public command) — scoped to EP5 only, distinct from EP6's job of wiring the public `update` verb.

## Questions for validating

- ~~Confirm the actual naming convention of existing GitHub release tags...~~ **Resolved during EP2/S3 planning (2026-07-22):** `git ls-remote --tags origin` shows `v0.1.0`, `v0.1.1`, `v0.1.2` — plain `vX.Y.Z` semver, no prerelease/build metadata in use. A manual split-and-compare (strip leading `v`, split on `.`, compare three integers) is sufficient; no `semver` crate dependency is needed.
- ~~Confirm whether a cross-job `checksums.txt` merge... or two separate checksum files... is the pragmatic fallback.~~ **Resolved during validating (2026-07-22):** confirmed structurally sound either way — a merge job with `needs:` on every asset-producing job plus `upload-artifact`/`download-artifact` is valid against this repo's actual `release.yml` structure (the existing `macos-install-smoke` job already uses `needs: build`, precedent at release.yml:194). Cell `self-update-merge-config-2` prefers one merged `checksums.txt` but explicitly allows a per-job-group fallback if the artifact-passing wiring proves impractical during execution — EP3 (S4)'s future download+verify logic must be designed to tolerate either outcome (one file or several), not assume a single `checksums.txt` exists.

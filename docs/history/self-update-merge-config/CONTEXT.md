# herdr-go update (self-update + config merge) — Context

**Feature slug:** self-update-merge-config
**Date:** 2026-07-22
**Exploring session:** complete
**Scope:** Standard
**Domain types:** CALL (new CLI subcommand), RUN (release download + service restart), ORGANIZE (config merge)

## Feature Boundary

A new `herdr-go update` subcommand checks the latest GitHub release for `vantt/herdr-go`, and — if newer than the running binary — downloads and verifies it, swaps the binary, merges the user's existing `config.json` against the new version's default config (new fields seeded with defaults, existing user values untouched), backs up the old config first, restarts the service automatically via the existing `herdr-go service restart` mechanism, and confirms the result with a health/status check — rolling back to the previous binary+config and restarting again if that health check fails.

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | `update` fetches only via GitHub `releases/latest` for repo `vantt/herdr-go` — same channel as `install.sh`. No version-pin flag in v1. | Matches `install.sh:132-137` (version/URL resolution); user framed this as end-user convenience, not a power-user tool — YAGNI on version pinning. |
| D2 | `update` compares the **semver prefix** of the running `herdr_go::VERSION` against the latest release tag; if the running semver is already >= latest, it reports up-to-date and does nothing (no download, no restart). | `VERSION` is a composite fingerprint (semver + short git sha + build timestamp, `src/lib.rs:19-22`), not a bare semver, so the comparison must extract and compare only the semver portion — a direct string compare against a release tag would not work. No existing "compare to latest" helper exists; avoids needless restarts. |
| D3 | After the binary swap, `update` **always** automatically triggers the existing `herdr-go service restart` mechanism (PBI-033) with no confirmation prompt. | User explicit: *"việc update tự trigger restart luôn sẽ rất tốt vì thực sự không nhớ nổi lệnh restart"* — direct answer, not inferred. |
| D4 | After restart, `update` automatically calls the existing **`/api/health` endpoint** and reports the result in the `update` command's own output. | Matches PBI-042's original description verbatim. Distinct from the `service status` verb (`checks.rs:392-408`, service-manager state only) — `/api/health` is the process-level confirmation. `run_service_command` has no built-in health check today, so this is new glue code wiring an existing endpoint into a new flow, not new product scope. |
| D5 | The merge's source-of-truth for "new version's default config" is the **new binary's own compiled `ensure_config` default JSON** (`src/config/mod.rs:846-869`) — no separate fetched template file. New fields merge in with their default value; fields the user already has are never overwritten. | Avoids maintaining a 4th duplicate default-config template (3 already exist and drift today — see PBI-044); reuses the existing struct. Note for planning: `ensure_config`'s default is generated dynamically (e.g. `allowed_roots` is seeded from the running user's `home()/projects`, `mod.rs:848-860`), not a static constant — extracting a given field's default means reading that generated output, not a fixed map. |
| D6 | Field deletion/rename across versions is **out of scope for v1** — no rename-mapping table. An orphaned field (present in the user's file, absent from the new default) is left untouched, with no warning printed. | Rename-mapping needs a per-version migration table with no current prior art or demonstrated need — YAGNI, matches the user's "don't need to read repo instructions" simplicity framing. Revisit only if a real rename need surfaces. |
| D7 | The existing config file is backed up before the merged result is written, reusing the existing `backup_and_recreate` helper (`src/config/write.rs:86`). | Existing precedent already built for exactly this class of risk (currently wired only into doctor's repair flow); also supports the user's stated future intent of building a settings form off the merged config file safely. |
| D8 | `update` verifies release binary integrity (checksum) before overwriting the running binary. This requires extending the release pipeline to publish a checksum for each release asset. | User chose this over matching `install.sh`'s current no-verification model, since the `update` path runs fully automated/unattended, unlike a manually-run `install.sh`. |
| D9 | If the post-restart health check fails, `update` automatically rolls back to the previous binary + previous (backed-up) config, restarts again, and then reports the failure to the user. | User chose auto-rollback over "just report the error" — avoids bricking a running service on a bad release; matches the convenience framing (user doesn't want to manually debug/restart). |
| D10 | If the latest release has no published checksum yet (release pipeline not updated to D8 yet), `update` **fails closed**: refuses to download/install anything and reports the error. No warn-and-proceed fallback. | User chose fail-closed over a warning-only fallback — consistent with D8's own reasoning; `update` is effectively gated on the release pipeline shipping checksums before it can update anything at all. |

### Agent's Discretion

- Exact CLI flag/subcommand shape (e.g. `herdr-go update` with no args vs. flags like `--check-only`) is left to planning, as long as the default invocation performs the full D1-D9 flow with no required flags.
- Exact rollback storage mechanism (e.g. `.bak` suffix on the previous binary vs. a dated backup dir) is left to planning; the only fixed constraint is that both the previous binary and previous config must be recoverable at the moment of the health-check rollback (D9).

## Specific Ideas And References

- User's stated future intent: the merged config file becomes the reliable, always-current source config that a later settings-form feature can build on — not new scope now, but D5/D7 (single merge source-of-truth + backup) are chosen partly to keep that door open cleanly.

## Existing Code Context

From the quick scout only. Downstream agents read these before planning.

### Reusable Assets

- `install.sh:132-147` — existing `releases/latest` GitHub download + OS/arch asset resolution + `tar`/`install` binary-swap logic; the closest prior art for D1's download step.
- `src/config/mod.rs:846-869` (`ensure_config`) — compiled default-config JSON template (merge target for D5); currently only writes when the file doesn't exist (PBI-036 D1) — `update`'s merge is new logic layered next to this, not a change to `ensure_config` itself.
- `src/config/write.rs:160-181` (`repair_fields`) — nearest existing field-merge shape (keeps old values, applies explicit replacements) from doctor's repair flow; different semantics (no "seed default for new field" case) but useful reference for the merge implementation.
- `src/config/write.rs:86` (`backup_and_recreate`) — existing backup-before-overwrite helper (D7), currently used only by doctor's repair flow.
- `src/doctor/checks.rs:366-459` (`run_service_command`) — thin per-OS pass-through for `service restart`/`status` (PBI-033); reused as-is for D3/D9's restart step, per spec's own constraint (`docs/specs/service-lifecycle.md` R4) that it never gains retry/state-checking logic beyond the native command.
- `src/lib.rs:22` (`herdr_go::VERSION`) — compile-time build fingerprint const, used for D2's current-vs-latest compare.

### Established Patterns

- Config default-of-truth is duplicated in 3 places today (`ensure_config`, doctor's `default_config_json`, `install.sh`'s fresh-install literal) and already known to drift (PBI-041 was caused by exactly this) — D5 pins `ensure_config`'s copy as this feature's merge source; the broader dedup is deferred to PBI-044.
- Service-lifecycle commands are deliberately "thin pass-through, no added retry/state logic" (service-lifecycle.md R4) — `update`'s restart step must call the existing command, not reimplement restart logic.

### Integration Points

- `src/main.rs:162-163` — CLI verb dispatch point where a new `update` verb would be wired in, alongside the existing `--service <verb>` dispatch.
- `/api/health` (referenced by `VERSION`, `src/lib.rs:19-22`) — the health-check endpoint D4/D9's confirmation step calls.

## Canonical References

- `install.sh` — current manual install/update flow this feature automates.
- `docs/specs/service-lifecycle.md` — locked constraints (R4 and the Windows-branch gap) governing reuse of `service restart`/`status`.
- `docs/history/default-agent-presets/CONTEXT.md` (D1, D2) — the "existing config file is never touched by `ensure_config`" decision this feature is scoped to layer a merge on top of, and the pre-existing doctor-template duplication note.
- `docs/backlog.md` PBI-041, PBI-042, PBI-044 — related/originating backlog rows.

## Outstanding Questions

### Deferred To Planning

- [ ] Where does the release pipeline (GitHub Actions) currently build/publish release assets, and what's the minimal change to also publish a checksum per D8? — needs a look at the actual release workflow file, which is implementation research, not a product decision.
- [ ] Concrete rollback storage mechanism (previous binary/config location and naming) — per Agent's Discretion, left to planning to pick within the D9 constraint.

## Deferred Ideas

- Reconciling the 3 duplicated default-config templates (`ensure_config`, doctor's `default_config_json`, `install.sh`'s literal) into one source — captured as `docs/backlog.md` PBI-044 (proposed), not part of this feature's scope (D5 sidesteps it by only depending on `ensure_config`'s copy).
- A future settings-form UI built on top of the merged config file — user's stated long-term direction, not scoped here; no new decision needed until that feature is proposed.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked decisions, code context, canonical references, and deferred-to-planning questions. Validating and reviewing use locked decisions for coverage and UAT.

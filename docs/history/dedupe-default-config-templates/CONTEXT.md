# Dedupe Default Config Templates — Context

**Feature slug:** dedupe-default-config-templates
**Date:** 2026-07-22
**Exploring session:** complete
**Scope:** Standard
**Domain types:** ORGANIZE, RUN

## Feature Boundary

Collapse the default `config.json` document, currently hand-written in four
separate places that drift (Rust ×2, bash, PowerShell), down to one canonical
source (`config::default_config_json`) that every other emitter obtains
verbatim instead of re-deriving; ends at "every fresh-install path and the
doctor corrupt-recreate path produce byte-identical config.json content" —
does not touch already-installed machines (that's PBI-041).

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Scope is **4** duplicate default-config JSON emitters, not the 3 the original PBI-045 backlog text named: `config::default_config_json` (`src/config/mod.rs:851-863`, canonical), `doctor::default_config_json` (`src/doctor/checks.rs:902-920`, corrupt-recreate path only), `install.sh:151`, and `install.ps1:132-140` (found via scout, not in the original backlog row). | `install.ps1` builds the same incomplete (missing `agent_presets`) default-config hashtable as `install.sh` — leaving it out would ship a half-fixed dedupe. |
| D2 | Canonical source stays `config::default_config_json` (`src/config/mod.rs`), already established as the merge source-of-truth by `self-update-merge-config` D5. Every other emitter must produce byte-identical content — directly (in-crate Rust callers) or indirectly (installers, via the running binary) — never re-derive its own copy. | Reuses the existing reference point instead of inventing a 5th representation (e.g. a separate template file or a shared `Default` struct). |
| D3 | `doctor`'s local `default_config_json` (`src/doctor/checks.rs:902-920`, used only by the corrupt/unparseable-config recreate path at `checks.rs:562`) is deleted; that call site computes the same `home/projects`-or-`home` root and calls `config::default_config_json(&root)` directly. | In-crate call, no cross-process step needed. `checks.rs:535`'s missing-file path already reuses `config::ensure_config` directly — only the corrupt-recreate path had its own copy. |
| D4 | `install.sh` and `install.ps1` obtain the canonical JSON from the freshly-downloaded `herdr-go` binary itself: a new CLI branch (exact shape left to planning) prints `config::default_config_json`'s output to stdout given a root/home argument; each installer captures that stdout and writes it to `config.json` exactly as today — same idempotent only-if-missing guard, same timing (before the service's first start). | Both installers already download+extract the binary before writing config.json, so this is the only mechanism that fully removes the hand-written bash/PowerShell literals while preserving the existing guarantee that `config.json` exists immediately after the installer finishes. **Rejected:** deleting the installer's config-write block and relying on `ensure_config` at first service start — would silently break that guarantee on Linux, where `install.sh` enables but does not auto-start the systemd unit (macOS/Windows do auto-start, Linux doesn't). |
| D5 | This feature fixes only the write path for fresh installs and doctor's corrupt-recreate path. It does **not** retroactively backfill already-installed `config.json` files missing `agent_presets` — that stays PBI-041's separate concern (already mitigated for machines that run `herdr-go update`, per `self-update-merge-config` D5). | Re-affirms the scope boundary already drawn in `docs/history/self-update-merge-config/CONTEXT.md` — carried forward, not re-litigated. |
| D6 | This is also a real functional fix, not pure DRY cleanup: today, on every platform, the installer pre-writes `config.json` **before** the binary's first run, so `ensure_config`'s own `agent_presets`-seeding logic never fires on a fresh install via any installer — only via a later `herdr-go update`. After D4, fresh installs on Linux/macOS/Windows all get `agent_presets` seeded correctly at install time. | Confirmed by reading `install.sh` (CONFIG_FILE written before both the Darwin and systemd branches that install/start the service) and `install.ps1` (`ConfigFile` written before `Start-ScheduledTask`). Recorded so planning/review don't mistake this for a no-user-visible-effect refactor. |
| D7 | `config.example.json` (repo root, shipped in every release dist per `.github/workflows/release.yml:82`) is also missing `agent_presets` — a 5th artifact carrying the same drift, discovered during planning, not by the original exploring scout. Folded into scope: add `agent_presets` to it by hand, keeping its intentionally-different placeholder values (`herdr_session: gateway`, `allowed_roots: CHANGE_ME`) since those exist to prompt user customization, not to byte-match the canonical default. | It documents the current default shape for users editing their own config; leaving it stale after this fix would recreate exactly the drift this feature exists to close. Unlike D4's installers, it is a static documentation artifact, not code — fixed as a plain field addition, not wired through the new CLI branch. |

### Agent's Discretion

Exact shape of the new CLI branch (subcommand/flag name, whether it's
documented in `--help` or purely internal, exact root/home argument name) is
left to planning — implementation detail, not a product decision.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Canonical default config | The exact byte-for-byte JSON document produced by `config::default_config_json(root)` — the one reference every other emitter (doctor, install.sh, install.ps1) must match exactly, per D2. |

## Existing Code Context

### Reusable Assets

- `src/config/mod.rs:851-863` (`default_config_json`) — the canonical emitter (D2). Already consumed by `ensure_config` (`mod.rs:865`) and by `update`'s config-merge (`src/config/merge.rs`, `self-update-merge-config` D5).
- `src/main.rs:212` — `ensure_config` is called here at normal startup; it is the reason config.json gets created at all when missing.

### Established Patterns

- `install.ps1`'s web-secret handling (comment at `install.ps1:176-178`: "the running binary creates and ACL-protects its own token file on first start; we only read it here") — existing precedent for an installer deferring generation to the binary rather than duplicating logic. D4 follows the same shape for config.json, but via an explicit CLI print (not a deferred first-start write) so the pre-existing "config.json exists right after install" guarantee is kept.

### Integration Points

- `src/doctor/checks.rs:562` — `write::backup_and_recreate(config_path, &default_config_json(home))` call site that D3 repoints at `config::default_config_json`.
- `install.sh:151` — `printf` literal (missing `agent_presets`) that D4 replaces with a captured-from-binary write.
- `install.ps1:132-140` — `ConvertTo-Json` hashtable literal (missing `agent_presets`) that D4 replaces with a captured-from-binary write.
- `config.example.json` — static documentation sample (missing `agent_presets`) that D7 fixes with a plain field addition.

## Canonical References

- `docs/history/self-update-merge-config/CONTEXT.md` D5 — established `config::default_config_json` as the canonical source-of-truth this feature builds on.
- `docs/backlog.md` PBI-045 — this feature's originating backlog row (flipped to `in-flight`).
- `docs/backlog.md` PBI-041 — the separate, still-`proposed` backfill concern for already-installed machines (D5 boundary).

## Outstanding Questions

### Resolve Before Planning

None — all material questions resolved with confident recommendations (gate-bypass level `full`; see D1-D6 rationale for the rejected alternative considered at D4).

### Deferred To Planning

- [ ] Exact shape of the new CLI branch installers call to fetch canonical JSON (name, `--help` visibility, argument naming) — technical only, no product impact.
- [ ] Whether `checks.rs`'s root-selection (`home.join("projects")`-or-`home`) can be extracted as a small shared helper reused by `config::ensure_config` too, avoiding a near-duplicate of *that* logic as well — technical only, doesn't change JSON output.

## Deferred Ideas

- PBI-041 (backfill `agent_presets` into already-installed machines that haven't run `update`) — stays a separate, already-existing backlog item; not this feature's scope (D5).

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads
locked decisions, code context, canonical references, and the
deferred-to-planning questions above.

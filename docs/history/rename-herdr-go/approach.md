# Approach: Rename herdr-gateway to herdr-go

## Recommended path

Follow the user-requested order per D1–D7: rename the GitHub repository first, update and verify `origin` second, then apply one atomic compatibility-aware local rename. Locally, introduce canonical `herdr-go` paths with fail-closed legacy-directory migration before any canonical directory is created, rename systemd templates and installer wiring, make the Linux installer self-contained when fetched remotely, update the release producer/consumer contract, then replace the technical-first docs with a value-first funnel and advanced layer.

## Rejected alternatives

- Blind repository-wide replacement — corrupts historical evidence and silently abandons existing config/SQLite state.
- Keep old config/data paths forever — leaves the current product internally branded with the retired name and violates the requested complete rename.
- Rename local references first — contradicts the user's requested operation order.
- Rename the `herdctl` executable — not requested and would unnecessarily break scripts and operator muscle memory.
- Advertise the same one-command install on every release platform — macOS and Windows do not yet have the verified service integration used by the Linux installer.
- Keep a remote installer's source-build fallback — a fetched script has no checkout to compile; it should fail clearly and link to the advanced source-build path.

## Risk map

| Component | Risk | Reason | Proof needed |
|---|---|---|---|
| Config/data migration | HIGH | A wrong move can hide credentials or SQLite history | Unit tests for old-only, new-only, and both-exist cases |
| systemd migration | HIGH | Old/new production and development services could bind the same port | Fake-systemctl behavior test covering the full conflict matrix and command ordering |
| Release artifacts | HIGH | Producer/consumer string drift breaks installation | Contract search and real post-release asset smoke test |
| No-clone Linux bootstrap | HIGH | Current script reads checkout-local config and service templates | Run a network-stubbed installer from an empty directory and assert installed outputs |
| GitHub rename/remote | MEDIUM | External mutation can leave local origin stale | `gh repo view` and `git remote get-url origin` after rename |
| Branding/docs | MEDIUM | False platform promises or broken progressive links undermine the first run | Scoped stale-name scan, link check, and README structure assertions |

## Files and order

1. GitHub repository and local `origin` — external rename and verification.
2. `src/config/mod.rs`, `src/main.rs`, `src/doctor.rs` — canonical paths, fail-closed migration, legacy diagnosis.
3. `packaging/herdr-go.service`, `packaging/herdr-go-dev.service`, `install.sh`, `dev-deploy.sh` — service and filesystem migration.
4. `.github/workflows/release.yml` — new archive producer matching installer consumption.
5. `README.md`, `docs/installation.md`, `docs/usage.md`, `docs/advanced/`, specs, and a durable rename/documentation contract test.

## Relevant learnings

- `docs/history/learnings/20260718-release-v0-1-0.md` — release archive production and installer consumption are one compatibility contract.
- Decision `a1cd297d-8a00-4979-8a4d-c9b4e36f4238` — the current SQLite path must not change without explicit migration.
- Decision `c202a89a-01f7-4f10-a310-2ebb4632535e` — archive naming must change and be smoke-tested atomically.

## Questions for validating

- Migration uses a sibling-directory rename without reading contents and aborts on rename failure; it never falls through to fresh state. When old and new both exist, startup emits the warning to stderr/systemd logs.
- Production install stops/disables legacy production, legacy development, and new development units before starting new production. Development deploy stops/disables legacy production, legacy development, and new production before starting new development. Both remove legacy unit files and reload systemd.
- Install/deploy render the hardened unit with the resolved canonical config/data paths, including custom XDG locations, rather than leaving `%h` defaults inconsistent with runtime.
- A committed contract test owns the scoped stale-name allowlist, custom-XDG path agreement, producer/consumer/service assertions, harmless missing-unit behavior, and fail-before-start ordering.
- A self-contained installer emits its config and service contract without checkout-local files and fails clearly if no release asset is available.
- The release archive carries every relative guide linked by README; a contract check rejects broken local links and a clone-first primary path.
- Demo mode keeps its memorable credential safe by binding loopback unless the user explicitly opts into network exposure.
- The service keeps system protection and privilege hardening without applying a blanket read-only policy to user workspaces.
- The installer verifies a functional systemd user manager before moving state or installing files; docs name that boundary and explain token recovery for repeat installs.

# Windows Support — Context

**Feature slug:** windows-support
**Date:** 2026-07-19
**Exploring session:** complete
**Scope:** Deep
**Domain types:** CALL | RUN | READ | ORGANIZE

## Feature Boundary

Make herdr-gateway build, package, configure, start, and operate truthfully on supported Windows systems, with the support level determined by Windows runtime evidence and full completion requiring a live connection to real herdr.

## Locked Decisions

These are fixed. Planning must implement them exactly — cited, never reinterpreted. Changing one requires a new D-ID or explicit supersession.

| ID | Decision | Rationale |
|----|----------|-----------|
| D1 | Windows support is claimed only to the level proven on Windows. A demo/fake-backed binary is labeled preview; full completion requires live real-herdr start, discovery, observe, input/reply, and restart evidence on Windows. | A Windows ZIP already exists in the release matrix without a Windows compile or runtime contract; packaging alone is not support. |
| D2 | Production connectivity must use a Windows endpoint transport actually exposed by upstream herdr. Do not invent or ship an unproven gateway-specific relay merely to bridge the current Unix-socket adapter. | The application wire protocol is transport-independent, but the endpoint capability belongs to the upstream integration contract. |
| D3 | Default Windows configuration lives under per-user roaming application data; the database and disposable runtime data live under per-user local application data; the token lives in a user-only protected location. Unix/XDG locations are not migrated automatically unless the operator explicitly selects them. Validation must prove that another ordinary local user cannot read the token. | These semantics match Windows roaming/local expectations and make the security claim testable without prescribing a particular API. |
| D4 | The first supported distribution is a target-specific ZIP and foreground command. Windows Service, auto-start, PowerShell installer, and development deployment are out of scope until foreground real-herdr operation is proven. | This is the smallest honest lifecycle boundary and prevents Linux-only systemd/install material from appearing in Windows packages. |
| D5 | Linux behavior, paths, shell installer, systemd service, migration rules, and release artifacts remain unchanged except where packaging must become target-specific. | Windows support must not regress the existing supported platform. |
| D6 | A ZIP may be published as **Windows preview — demo only** after Windows CI proves its preview contract. General Windows support, normal installation guidance, and the public support matrix remain unavailable until D1's live real-herdr evidence passes. | Artifact availability and a production support claim are different promises. |
| D7 | If the selected upstream herdr release does not build and run on Windows with a usable local endpoint, implementation stops at an honestly labeled preview artifact. Full Windows support remains blocked and no substitute production transport is introduced. | This gives planning a deterministic stop branch without weakening D1 or D2. |
| D8 | The initial target matrix is `x86_64-pc-windows-msvc` on Windows 11 and Windows Server 2022. CI runner aliases may be used only when their resolved image is recorded; support docs name the pinned operating-system versions. | A rolling runner label is evidence transport, not a stable customer support contract. |
| D9 | For the first slice, clean-machine setup means download, extract, configure, and run the ZIP in the foreground. Service and installer acceptance move to a follow-up backlog item. | This supersedes the broader lifecycle wording originally captured in PBI-011 and aligns acceptance with D4. |
| D10 | Preview CI must prove build, portable tests, ZIP contents, extract/run, help, doctor, first-run native state, repeat-run preservation, token access protection, and HTTP/UI access on the pinned matrix. Full-support CI additionally proves real-herdr start, discovery, observe, input/reply, and restart. | Each published label maps to a concrete evidence tier. |

### Agent's Discretion

The agent may choose conditional module boundaries, Windows directory APIs, archive assembly, CI decomposition, and test seams. Choices must preserve all locked decisions above, minimize platform forks, and prefer behavior tests over compile-only assertions.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Windows preview | A Windows-native artifact whose demo/fake-backed foreground path is verified, while real-herdr connectivity remains explicitly unsupported or blocked. |
| Full Windows support | Foreground gateway operation against real herdr on Windows with the complete control flow proven end-to-end. |
| Native per-user state | Configuration in roaming app data, database/runtime data in local app data, and token material protected for the owning user rather than stored under Unix/XDG defaults. |

## Specific Ideas And References

- Keep the embedded web interface and portable wire protocol; replace only platform-bound endpoint and lifecycle assumptions.
- Treat the existing Windows release matrix entry as an unverified packaging skeleton, not evidence of support.

## Existing Code Context

### Reusable Assets

- `src/herdr/mod.rs` and `src/herdr/fake.rs` — transport boundary and fake backend for portable preview/runtime tests.
- `src/herdr/wire.rs` — transport-independent newline-delimited request and response model.
- `src/supervisor.rs` — injectable process restart seam suitable for platform-specific command behavior.
- `.github/workflows/release.yml` — existing MSVC target, executable suffix, and ZIP skeleton to make target-specific and verify.

### Established Patterns

- Embedded web assets and TCP HTTP serving travel with one executable.
- Release claims are withheld until the matching artifact is downloaded, extracted, run, and smoke-tested.
- Preflight checks happen before migration, build, filesystem, or service mutation.

### Integration Points

- `src/herdr/socket.rs` — unconditional Unix stream and Unix default endpoint; primary production compile/runtime blocker.
- `src/config/mod.rs` — Unix/XDG state discovery and Unix-only token mode handling.
- `src/main.rs` — endpoint composition, help text, doctor behavior, and foreground startup.
- `.github/workflows/ci.yml` — currently tests only on Ubuntu.
- `.github/workflows/release.yml` — currently puts Linux installer/service material into the Windows archive.
- `docs/specs/installation.md` — current Linux-only lifecycle contract that Windows docs must extend without weakening.

## Canonical References

- `docs/specs/system-overview.md` — current gateway boundaries and integration model.
- `docs/specs/installation.md` — current install, migration, preflight, and release-truthfulness rules.
- Upstream herdr Windows documentation/source and release artifacts — must establish the real Windows endpoint and process contract during planning research.
- Microsoft Windows application-data and file-security documentation — must anchor native state and token protection choices during planning research.

## Outstanding Questions

### Resolve Before Planning

None. Full gate bypass delegates evidence-grounded defaults to the agent.

### Deferred To Planning

- Does the selected upstream herdr release build and run on the D8 matrix, and which endpoint transport does it expose? A negative answer takes the D7 preview branch.
- Which native directory API and ACL mechanism give deterministic per-user state and token protection?
- Which portable tests can run unchanged on Windows, and which Linux installer contracts remain Linux-only?
- Can the current process supervisor invoke the upstream Windows executable without shell-specific behavior?

## Deferred Ideas

- Windows Service, auto-start, PowerShell installer, and dev-deploy support — revisit only after foreground full support is proven.
- Additional Windows architectures — add only after x86_64 MSVC is runtime-proven.

## Handoff Note

CONTEXT.md is the source of truth. Planning must research upstream Windows capability and official platform behavior before choosing transport, state, ACL, packaging, and verification details. Validation must reject any support claim stronger than its Windows evidence.

# Validation: Windows foreground runtime foundation

## Verdict

**NOT READY — RUN ON A WINDOWS-CAPABLE, GIT-WRITABLE EXECUTION SURFACE.**

Gate 3 remains unapproved. No production source edit for `windows-support` is authorized.

## Reality gate

- **MODE FIT: PASS** — six risk flags and credential security require the high-risk lane.
- **REPO FIT: PASS** — the portable protocol boundary, endpoint adapter, config/state, supervisor, CI, and release paths exist. Upstream herdr source proves a reusable cross-platform local-socket mechanism.
- **ASSUMPTIONS: PASS after repair** — the upstream source contract is pinned at `.bee/spikes/windows-support/upstream-contract.md`; runtime and ACL assumptions are owned by `windows-support-3`.
- **SMALLER PATH: PASS** — adapting upstream's `interprocess::local_socket` mechanism is smaller and safer than CLI subprocesses or a gateway relay.
- **PROOF SURFACE: FAIL in this environment** — only `x86_64-unknown-linux-gnu` is installed; `pwsh` is absent; listener E2Es are sandbox-denied; `.git/index.lock` cannot be created, so a Windows workflow cannot be committed/pushed from this runner.

## Feasibility matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| Upstream exposes a compatible Windows endpoint | High | Current source + docs | herdr `src/ipc.rs` uses `GenericFilePath`/`GenericNamespaced`; public docs specify named pipes | PASS (source-level) |
| Exact logical endpoint mapping can be reused | High | Same conversion and resolution order | `.bee/spikes/windows-support/upstream-contract.md` | PASS (source-level) |
| Windows branches compile | High | x86_64 MSVC compile/test | Windows target absent locally; no Windows job can be launched from uncommitted state | NOT PROVEN |
| Real request/subscription/restart works | High | v0.7.4+ Windows live smoke | No Windows host/PowerShell in this runner | NOT PROVEN |
| Token is inaccessible to another ordinary user | High | Production creation + second-user read denial | Linux cannot establish NTFS effective permissions | NOT PROVEN |
| Schedule is valid | Medium | zero cycles and expected dependency order | `windows-support-1 -> windows-support-2 -> windows-support-3` after repair | PASS |

## Adversarial review and repairs

The coherence, feasibility/cold-pickup, and security/scope lenses found blockers: Windows-only code could cap without compiling; ACL denial had no owner; proof-first evidence was absent from the DAG; existing-token validation and race-free creation were unspecified; Windows allowed-root and doctor disclosure were underdefined.

Repairs applied:

- Pinned upstream's exact `interprocess::local_socket` compatibility pattern in `.bee/spikes/windows-support/upstream-contract.md`.
- Revised `windows-support-1` to use that mechanism and include supervisor restart wiring.
- Revised `windows-support-2` to require protected-before-visible token creation, existing-token effective validation, native absolute allowed roots, fail-closed startup, and redacted diagnostics.
- Added blocking `windows-support-3` for pinned Server 2022 compile, real herdr request/subscription/restart, native state, and second-user token denial.
- Clarified that the current slice includes its Windows proof and cannot close on Linux-hosted unit tests alone.

## Cell review

- `windows-support-1`: structurally cold-pickup-ready after the upstream evidence anchor and scope repair.
- `windows-support-2`: structurally cold-pickup-ready after security semantics were made explicit; real ACL proof remains delegated to cell 3.
- `windows-support-3`: correctly owns the missing proof, but its verify command is intentionally Windows-only and cannot run in this environment.

## Approval block

Reality gate: **FAIL (proof surface unavailable)**
Feasibility: **NOT READY**
Structure: repaired after one review iteration
Unresolved blocker: Windows Server 2022/11 execution plus Git-writable workflow transport
Execution gate: **not approved**

## Resume condition

Resume on a Git-writable checkout with either a Windows Server 2022 host or permission to commit and run the repository's Windows CI. Run the baseline first, then validate `windows-support-3`; only a green result allows Gate 3 and production edits.

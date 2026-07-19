# Validation: rename-herdr-go current slice

## Reality gate

Mode: `high-risk`

Current work: rename current product surfaces, preserve existing state/services, and keep installation/release contracts coherent after the already-completed GitHub rename.

- MODE FIT: PASS — data migration, external repository identity, public install/release contracts, cross-platform claims, and multiple domains require the high-risk lane.
- REPO FIT: PASS — every bounded runtime, installer, service, workflow, package, documentation, and test path exists or is an explicitly declared rename/new artifact.
- ASSUMPTIONS: PASS — blocking migration, service-conflict, archive, stale-name, and documentation assumptions have executable proof surfaces.
- SMALLER PATH: PASS — blind replacement would corrupt history and abandon existing state; retaining the old canonical identity would not fulfill the rename.
- PROOF SURFACE: PASS — the cell runs focused contract checks plus formatting, Rust tests/lints, shell syntax, web bundle, and web tests.

Decision: proceed.

## Feasibility matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| GitHub repository and origin can be renamed first | External identity drift | GitHub and local remote query | `gh repo view` reports `vantt/herdr-go`; fetch/push origin are `https://github.com/vantt/herdr-go.git` | READY |
| Existing state migrates without reading secrets | Hidden credentials/SQLite history | Old-only, new-only, both, missing, unsafe/failure tests | Sibling-directory rename is content-opaque; cell requires fail-closed tests before canonical creation | READY WITH CONSTRAINTS |
| Prod/dev services cannot overlap | Port conflict | Fake-systemctl matrix and order/idempotency assertions | Cell and approach specify every conflicting legacy/new unit for both modes | READY WITH CONSTRAINTS |
| Hardened units match runtime paths | Write denial under custom XDG | Rendered-path contract test | Approach requires resolved canonical/legacy paths, including custom XDG, in the installed unit | READY WITH CONSTRAINTS |
| Release producer and installer agree | Broken downloads | Deterministic producer/consumer assertions | One archive-root contract is owned by the workflow, installer, and contract test | READY |
| Current branding is complete without rewriting history | Stale or corrupted references | Allowlisted scoped scan | Contract test owns the allowlist; history/audit/opaque fixtures are explicitly excluded | READY |
| Work can be scheduled | Dependency/collision | Cell scheduler | One-cell wave, zero cycles, zero unsatisfied dependencies, zero empty scopes | READY |

## Review findings

Iteration 1 found four structural blockers: the external step was missing/reordered, stale-name proof was absent, service migration semantics were ambiguous, and service behavior had only syntax checks. It also found migration-before-mkdir and fail-closed requirements missing.

Iteration 2 confirmed all blockers closed after the GitHub/origin rename, explicit prod/dev service conflict matrices, fail-closed migration ordering, and a committed contract test were added. Remaining constraints:

- both-exist warnings must appear on startup stderr/systemd logs;
- installed hardened units must use resolved default or custom-XDG config/data paths;
- missing legacy units are harmless, while real migration failures must prevent the new unit from enabling/starting;
- the true renamed-release download smoke test remains pending until a new release asset exists.

## Cell review

Cells reviewed: 1 (`rename-herdr-go-1`).

- CRITICAL flags: 0 open after iteration 2.
- MINOR flags: the observable warning channel and custom-XDG systemd behavior are resolved as explicit approach/report constraints.
- Cold pickup: PASS — truth artifacts, bounded files, executable acceptance, key links, and prohibitions are sufficient without chat history.

## Approval block

VALIDATION COMPLETE

- Mode: high-risk
- Reality gate: PASS
- Feasibility: READY WITH CONSTRAINTS
- Structure: PASS after 2 iterations
- Cell review: PASS (1 cell, 0 CRITICAL open)
- External prerequisite: GitHub repository and origin already renamed and verified
- Deferred proof: real installer download against the first `herdr-go` release asset

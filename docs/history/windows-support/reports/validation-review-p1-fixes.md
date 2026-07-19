# Validation: review P1 fixes

Verdict: **READY WITH CONSTRAINTS**

## Reality gate

- MODE FIT: PASS — security, release and cross-platform P1 repairs remain high-risk.
- REPO FIT: PASS — both cells use existing config/socket and workflow seams.
- ASSUMPTIONS: PASS after repair — the pinned preview asset and SHA-256 are verified from the upstream GitHub release API.
- SMALLER PATH: PASS — two non-overlapping cells cover runtime and CI/release concerns.
- PROOF SURFACE: PASS after repair — CI retains the MSVC target; release-only negatives are scoped to release.yml; checksum verification is bound to the exact binary path used by the smoke.

## Schedule

One parallel wave: `windows-support-4`, `windows-support-5`; no cycles, unsatisfied dependencies or empty file scopes.

## Validator repairs

1. Restricted the no-Windows-target assertion to the release workflow so Windows CI can retain `x86_64-pc-windows-msvc`.
2. Required both immutable preview URL and exact digest, verification before execution, and explicit binary-path injection into every smoke invocation.
3. Required a portable injected-root seam for host testing of Windows profile selection while retaining the real Windows proof obligation.

## Constraints

- No Windows artifact is published in v0.1.1.
- No Windows support claim is made.
- The blocked Windows Server 2022 proof remains required for a later Windows preview release.

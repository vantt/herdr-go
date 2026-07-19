# Validation: final rename acceptance correction

## Reality gate

- MODE FIT: PASS — state migration side effects, installer/deploy mutation ordering, and public install claims remain high-risk.
- REPO FIT: PASS — the unconditional migration call, dev-deploy ordering defect, stale workflow comment, old release asset names, and advertised curl path were all directly observed.
- ASSUMPTIONS: PASS — the mode matrix, mutation classes, and asset availability are explicit and testable.
- SMALLER PATH: PASS — one bounded correction touches only the runtime gate, sibling dev path, truthful docs/specs, workflow comment, and contract test.
- PROOF SURFACE: PASS — full Rust/shell/contract/web verification is runnable; focused tests must use the same predicate wired by main.

Decision: READY WITH CONSTRAINTS.

## Feasibility matrix

| Assumption | Evidence | Result |
|---|---|---|
| Mode isolation is implementable | Args already exposes doctor/demo/config/bind; migration primitive is isolated | READY |
| Bind-only remains normal startup | D9 and the cell matrix state it explicitly | READY |
| Dev deploy can preflight before mutation | All checks and mutation commands are visible and orderable in one shell script | READY |
| Quick install is currently unavailable | Existing release assets are `herdr-gateway-*`; installer requests `herdr-go-*` | READY |
| No release needs publishing in this scope | Docs can gate availability and name the evidence-based removal trigger | READY |

## Panel findings

The coherence review found D9 missing from CONTEXT and stale current-slice/success wording; both were repaired. Feasibility confirmed three real defects and constrained migration to `!doctor && !demo && config_path.is_none()` without redesigning explicit-config storage. Cold pickup found no critical flags; its mode-matrix, dev mutation-class/failure-path, and future caveat-removal warnings were added to the cell action.

## Cell review

- Cell: `rename-herdr-go-3`
- Dependencies: capped `rename-herdr-go-2`
- Critical flags open: 0
- Constraints: main-wired table matrix; fake/failure dev preflight where practical; explicit caveat removal trigger; no release/push/external mutation.

## Approval block

VALIDATION COMPLETE — READY WITH CONSTRAINTS

- Reality gate: PASS
- Structure: PASS after one repair iteration
- Cell review: PASS
- Deferred evidence: real download/extract/run/service smoke waits for the first renamed asset

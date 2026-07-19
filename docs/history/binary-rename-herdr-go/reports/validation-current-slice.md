# Validation: binary identity rename to herdr-go

## Reality Gate

- MODE FIT: PASS. Four risk flags make high-risk appropriate: public contracts,
  cross-platform, existing covered behavior, and multi-domain.
- REPO FIT: PASS. The existing rename/spec/testing surface already owns product
  identity and has contract tests that can be updated.
- ASSUMPTIONS: PASS. No external compatibility is required because the user
  explicitly confirmed the project is pre-release and requested no fallback.
- SMALLER PATH: PASS. A partial rename would break contracts; one atomic slice
  is the smallest honest path.
- PROOF SURFACE: PASS. Baseline full verify is green and focused grep/runtime
  checks can prove the new name.

## Feasibility Matrix

| Assumption | Risk | Proof | Result |
|---|---|---|---|
| Cargo accepts `herdr-go` bin with `herdr_go` crate imports | HIGH | Cargo package/bin/lib rename plus tests | READY |
| Installer/release/CI can agree on one executable name | HIGH | Script/workflow grep contracts | READY |
| Runtime env/state names can change without migration | HIGH | User decision D2/D4 and config tests | READY |
| Historical evidence can retain old names | MEDIUM | Scoped scan excludes `docs/history/` | READY |

## Verdict

READY WITH CONSTRAINTS: execute one atomic rename slice, no compatibility alias,
and verify with both focused rename contracts and the full project verify.

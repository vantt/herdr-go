# Validation: promoted-path safety correction

## Reality gate

- Mode fit: **PASS** — the change closes credential exposure and service-permission risks.
- Repository fit: **PASS** — the demo bind, generated unit, packaged unit, installer ordering, contract test, and user guides all exist in the bounded cell.
- Assumptions: **PASS WITH CONSTRAINTS** — the preflight uses `command -v systemctl` plus `systemctl --user show-environment`, not the ambiguous degraded-state status code.
- Smaller path: **PASS** — runtime defaults, installer behavior, packaged policy, tests, docs, and specs must agree.
- Proof surface: **PASS** — focused Rust, lint, shell contract, bundle, and web tests are runnable in this environment.

Decision: **READY WITH CONSTRAINTS**.

## Feasibility matrix

| Assumption | Evidence | Result |
|---|---|---|
| Demo can default to loopback without removing an explicit override | Existing argument parsing preserves `--bind`; demo config owns the default | Ready |
| Home workspace writes can coexist with system hardening | `ProtectHome=read-only` is independent of `ProtectSystem`; remove the former while retaining `NoNewPrivileges` and `ProtectSystem=strict` | Ready with constraints |
| User-manager support can be proven before mutation | `command -v systemctl` and `systemctl --user show-environment` are side-effect-free and ordered before the first migration call | Ready with constraints |
| Repeat-install token guidance can avoid disclosure | Installer stores the token in the canonical mode-600 environment file and prints only inside its creation branch | Ready |

## Panel and cold-pickup review

The panel found no structural blocker. The first cold-pickup review required more precision around the systemd directives and the exact preflight command/order. The cell was repaired to name both directives to retain, the directive to remove, the successful reachability probe, forbidden earlier mutations, canonical token retrieval/rotation, and source-order regression assertions.

Warnings retained:

- Keep the real renamed-release smoke test explicitly deferred until a release asset exists.
- Replace the stale spec claim that the installer automatically falls back to source compilation; it now stops and links the advanced guide.

## Approval block

VALIDATION COMPLETE — READY FOR EXECUTION

- Mode: high-risk
- Work: safe demo default, usable service permissions, pre-mutation installer preflight, and accurate login guidance
- Reality gate: PASS
- Feasibility: READY WITH CONSTRAINTS
- Cell review: repaired; no ambiguous implementation choice remains
- Unresolved concern: real published-asset smoke test waits for a release

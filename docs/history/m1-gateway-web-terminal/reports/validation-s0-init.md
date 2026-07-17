# Validation — S0 init cell (m1-0)

Lane: high-risk (greenfield). Slice under validation: init cell only. Bypass level: full (Gate 3 auto).

## Reality gate

| Check | Verdict | Evidence |
|---|---|---|
| MODE FIT | PASS | Greenfield high-risk; init cell is the mandated first slice (bee-hive greenfield lane). Skeleton-only, no feature logic — right size. |
| REPO FIT | PASS | No existing Cargo.toml/package.json (greenfield confirmed via `ls`/`find`). rustc+cargo 1.96.1, node 24.18.0, npm 11.16.0 all present (`cargo --version`, `node --version`). |
| ASSUMPTIONS | PASS | Crate stack (tokio/axum/serde/serde_json/rusqlite-bundled/reqwest/anyhow/thiserror/tracing/base64/tower-http) is standard, specified by major-version floor so cargo resolves latest-compatible. npm stack (vite/typescript/@xterm/xterm/vitest) standard on node 24. |
| SMALLER PATH | PASS | Init cell is already the smallest honest first slice; cannot shrink without skipping the compile-baseline. |
| PROOF SURFACE | PASS | The init cell's own `verify` (`cargo test && web build && web test`) is the empirical resolution proof — a wrong version fails loudly and is bumped in seconds. Pre-exec crate probe attempted but not required (and blocked by the write-guard during validating, correctly). |

## Feasibility matrix

| Assumption | Risk | Proof required | Evidence | Result |
|---|---|---|---|---|
| Rust crates resolve on 1.96.1 | LOW | init cell verify | standard stack, semver floors; empirical at execution | ACCEPTED (proven at exec) |
| npm stack resolves on node 24 | LOW | init cell verify | standard vite/xterm stack | ACCEPTED (proven at exec) |
| Fake-herdr frame shape is real | — | DISCOVERY PBI-002 | frame schema 7 fields verified against herdr source | ALREADY PROVEN |

Risky feature parts (auth silent-404, path-allowlist 7-step, Tier 2 relay, subscribe de-dup) are **not** in the init slice — each is proven by its own adversarial/integration test in its slice. That is the real proof surface, per plan.md risk map.

## Cold-pickup review (m1-0)

Cell carries files, read_first (CONTEXT+plan), directive action citing D4e3ef1a1, must_haves (compile + one green test each side + recorded verify), runnable verify, prohibitions (stubs only, no node_modules/target commit, no secret reads). A stranger could execute it cold. No CRITICAL flags.

## Verdict

**READY** — Gate 3 auto-approved under full bypass. Execution proves crate/npm resolution empirically; a failure returns to fix versions, not to re-plan.

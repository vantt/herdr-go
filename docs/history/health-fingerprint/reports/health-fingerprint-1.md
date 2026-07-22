# health-fingerprint-1 — Report

**Status:** [DONE]

**Outcome:** `build.rs` now computes the build fingerprint (git short-sha with `-dirty` suffix per D3, local-offset build timestamp per D1/D4, `nogit` fallback per D5) and emits it as a single `cargo:rustc-env=HERDR_GO_FINGERPRINT` value; removed the restrictive `rerun-if-changed=static` directive so it always reruns. `src/lib.rs`'s `VERSION` now reads that one env var (D6 single source), staying a compile-time `&'static str` with zero edits at its 3 existing call sites (`src/main.rs:113`, `src/main.rs:294`, `src/web/mod.rs:56`).

**Files touched:** `Cargo.toml`, `Cargo.lock`, `build.rs`, `src/lib.rs`

**Commit:** `269d73a` — `feat(health-fingerprint-1): replace semver-only VERSION with build fingerprint`

**Full trace/evidence:** `.bee/cells/health-fingerprint-1.json`

## Outstanding Questions

None.

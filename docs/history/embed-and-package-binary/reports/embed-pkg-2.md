# embed-pkg-2 — Embed the web UI into the binary with a disk-override fallback

**Status:** [DONE]

**Outcome:** `router()` prefers an on-disk build under `static_dir` when
`<static_dir>/index.html` exists (config override / local dev, D b300856d) and
otherwise serves the UI embedded into the binary via `rust_embed` + `axum_embed`
(SPA fallback → `index.html`, 200). `herdctl doctor` now reports the embedded UI
as a working fallback instead of a failure.

**Files touched:** `src/web/mod.rs`, `src/doctor.rs`

**Verify:** `(cd web && npm run bundle) && cargo test --quiet && cargo clippy --quiet -- -D warnings`
— bundle built; lib 87 passed / 0 failed (incl. 3 new router tests); clippy exit 0.

Full trace and behavior-change evidence: [`.bee/cells/embed-pkg-2.json`](../../../../.bee/cells/embed-pkg-2.json)

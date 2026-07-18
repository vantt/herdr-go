# embed-pkg-1 — worker report

Status: [DONE]

Outcome: Added `rust-embed`+`axum-embed` deps, a `build.rs` guard so a fresh checkout compiles without `npm run bundle`, `config::data_dir()` (byte-identical to install.sh's default `SHARE_DIR`), and rewired `main.rs`'s sqlite path off `config.static_dir` onto `data_dir()`.

Files touched: `Cargo.toml`, `Cargo.lock`, `build.rs` (new), `src/config/mod.rs`, `src/main.rs`.

Commit: `ff9661e` — feat(embed-pkg-1): add embed deps, build.rs guard, decouple sqlite data_dir from static_dir

Full trace/evidence: `.bee/cells/embed-pkg-1.json`

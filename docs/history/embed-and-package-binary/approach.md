# Approach: embed-and-package-binary

## Recommended path

Embed the built web UI into `herdctl` with `rust-embed` + `axum-embed` (per D b300856d), guarded by a `build.rs` that guarantees the `static/` folder exists at compile time (`create_dir_all`, gitignored dir stays gitignored — no repo changes needed) so `cargo build`/`test`/`clippy` never hard-fail when nobody has run `npm run bundle` yet. The router keeps `ServeDir`-from-disk when `<static_dir>/index.html` exists (config override / local dev with a real build present) and falls back to the embedded `ServeEmbed<Assets>` (SPA fallback → `index.html`, 200) otherwise — this is what makes `config.static_dir` an optional override rather than a hard requirement, honoring D b300856d without adding a second config flag. `rust-embed` defaults to reading from fs in debug builds and only embedding at compile time in `--release` builds; this repo enables the `debug-embed` feature so embedding always happens at compile time in every profile, deterministically, and is testable under plain `cargo test` — the router's own disk-override-if-present logic above is what covers the dev-override need instead, independent of rust-embed's own dev/release toggle. Consequence for verify commands: `static/` must be populated (`npm run bundle`) *before* any `cargo build`/`test` invocation that needs the real embedded UI, every time, in every profile — not just for release builds.

`install.sh` and `dev-deploy.sh` still assume a git checkout (`REPO_DIR`) — that invocation model is unchanged and matches every existing doc (`docs/installation.md` step 1 is `git clone` for all three options). What changes is only *how the binary is obtained*: `install.sh` first tries to download+extract the matching `herdr-gateway-<target>.tar.gz` from the latest GitHub release (per D 3168932d, same target strings as `release.yml`'s matrix), and only falls back to `cargo build --release` (preceded by `npm run bundle`, since embedding now needs `static/` populated *before* `cargo build` runs) when no prebuilt asset matches. `dev-deploy.sh` needs the same bundle-before-build reordering — it doesn't need the download path (it's explicitly "build this checkout and run it").

A real coupling had to be untangled to make this safe: `main.rs` currently derives the sqlite state-file path from `config.static_dir.parent()` — today that resolves to `~/.local/share/herdr-gateway` only because `install.sh` happens to write `static_dir` as `$SHARE_DIR/static`. Once `static_dir` becomes optional/no-longer-installed-to-disk, that derivation breaks. Fix: an explicit `herdctl::config::data_dir()` (`${XDG_DATA_HOME:-$HOME/.local/share}/herdr-gateway`, mirroring the existing `config_dir()`), independent of `static_dir`. This resolves to the exact same path existing installs already use, so no data migration is needed for anyone who installed via `install.sh`'s defaults.

`release.yml` keeps producing the same `herdr-gateway-<target>.tar.gz`/`.zip` bundle (binary + `install.sh` + `config.example.json` + `packaging/` + docs) — that bundle is what a standalone `install.sh` run (outside a checkout, if ever needed) or the checkout-based one needs for the config/systemd-unit templates. The only change: it stops copying `static/` into the bundle, since the binary now carries the UI itself.

## Rejected alternatives

- **Standalone `curl | sh` installer with no checkout assumption** (full mdview parity) — rejected: every existing doc path (`README.md`, `docs/installation.md`) assumes `git clone` first, and `install.sh` already depends on `REPO_DIR`-relative files (`packaging/herdr-gateway.service`, `config.example.json`) that mdview's simpler single-binary tool doesn't have an equivalent of. Rewriting the whole invocation model is out of scope for "packaging parity" and was not what the user asked to change (README/usage docs update, not the install UX contract).
- **`include_dir!` instead of `rust-embed`** — rejected: no dev-mode fs fallback, no SPA/mime/etag/gzip handling built in (would need hand-written axum handler + `mime_guess`), and `axum-embed` gives a drop-in `tower::Service` matching the existing `ServeDir`/`ServeFile` shape almost line for line.
- **Checksum/signature verification for downloaded binaries** — rejected per D 3168932d: HTTPS-from-GitHub is the trust boundary, matching mdview's own install.sh precedent; adding it now would be scope creep beyond "match mdview's pattern."
- **Move sqlite state file into `config_dir()` instead of introducing `data_dir()`** — rejected: would move existing installs' data (`~/.local/share/herdr-gateway/herdctl-state.sqlite`) to a different directory (`~/.config/herdr-gateway/`) on upgrade, silently orphaning history. `data_dir()` reproduces today's real path exactly.

## Risk map

| Component | Risk | Reason | Proof needed |
|---|---|---|---|
| `rust-embed`/`axum-embed` compile-time embedding | MEDIUM | New crates, SPA-fallback field name unconfirmed in docs fetch | `cargo doc` / compile the router change; confirm `ServeEmbed` fallback API against installed crate version, not just docs.rs prose |
| `build.rs` guard | LOW | Standard `create_dir_all` pattern, well-understood | `cargo test`/`clippy` on a checkout with no `static/` present (fresh clone simulation) |
| `data_dir()` / sqlite path decoupling | MEDIUM | Silent data-loss risk if the new path ever diverges from the old implicit one | Compare `data_dir()` output against today's `install.sh`-derived `SHARE_DIR` for the default `PREFIX` — must be byte-identical |
| `install.sh` download+fallback | MEDIUM | No GitHub release exists yet for this repo — the "successful download" branch is untestable end-to-end this session | Exercise the real fallback branch (download genuinely 404s today — no tags exist), confirm source-build path still succeeds; document the download-success branch as unverified pending a real tagged release |
| CI (`ci.yml` `rust` job) | LOW→MEDIUM without build.rs, LOW with it | `rust` job never runs `npm run bundle`; without the build.rs guard this job would break on every PR | Run the `verify` command locally end to end |
| `release.yml` simplification | LOW | Deleting one `cp -r` line | Read the diff; tarball still contains everything `install.sh`/systemd need |
| Docs accuracy | LOW | Text-only | Read-through against the actual new `install.sh`/config behavior |

## Files and order

1. `Cargo.toml`, `build.rs` (new), `src/config/mod.rs`, `src/main.rs` — deps, the compile-time-safety guard, `data_dir()`, sqlite path decoupling.
2. `src/web/mod.rs` (embedding + disk-override-or-embedded router logic), `src/doctor.rs` (web UI check message).
3. `install.sh`, `dev-deploy.sh`, `packaging/herdr-gateway.service` (stale `thanhsmind` → `vantt` org fix, incidental).
4. `.github/workflows/release.yml` (drop the `static/` copy).
5. `README.md`, `docs/installation.md`, `docs/deployment.md` (only if it needs a touch) — docs last, once behavior is settled.

## Relevant learnings

- `docs/history/learnings/critical-patterns.md`: the scout-block hook denies `build`/`target`/`dist`/`node_modules` in Bash commands — `cargo check`/`cargo test` not `cargo build` when running commands in-session; web build script is `bundle` not `build`. Directly relevant since this feature's own verify commands touch both.
- No prior decision or learning about `static_dir`'s dual role (UI assets + sqlite anchor) existed before this session — this approach is the first time it's made explicit.

## Questions for validating

- Does the installed `axum-embed` version's `ServeEmbed` actually expose a `fallback_behavior`/index-file field with the name assumed here? (docs.rs prose was inconclusive on the exact field name — confirm against the real crate source once vendored.)
- Is `${XDG_DATA_HOME:-$HOME/.local/share}/herdr-gateway` really byte-identical to what `install.sh`'s current default `PREFIX=$HOME/.local` produces for `SHARE_DIR`? (Yes by inspection — `PREFIX="${PREFIX:-$HOME/.local}"`, `SHARE_DIR="$PREFIX/share/herdr-gateway"` → `$HOME/.local/share/herdr-gateway`, and `XDG_DATA_HOME` defaults to `$HOME/.local/share` — but a custom `PREFIX` env override would diverge; that already exists today as an edge case, not a regression.)

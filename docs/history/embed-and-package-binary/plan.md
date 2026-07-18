---
artifact_contract: bee-plan/v1
artifact_readiness: implementation-ready
mode: high-risk
---

# Plan: embed-and-package-binary

Mode: `high-risk` — 5 risk flags: external systems (install.sh talks to GitHub Releases), public contracts (install.sh behavior, `config.static_dir` semantics, doctor output, README/docs), cross-platform (target-matching matrix in install.sh), existing covered behavior (router, main.rs, doctor.rs, install.sh, dev-deploy.sh, ci.yml are all live and tested), multi-domain (Rust build-time embedding + build.rs + 2 shell scripts + CI workflow + 3 docs files).

Why this is the least workflow that honestly protects the work: this is not a story-sized `standard` change — it changes what `cargo build` depends on (a `static/` dir that today only exists after a separate `npm` step, touched by nothing before this), relocates a data file's on-disk path (sqlite state, currently anchored to `static_dir` implicitly), and touches two shell scripts with no automated tests. A `standard` lane's plan-checker + cell reviewer would not catch a wrong sqlite-path assumption the way a persona pass forces enumerating "what breaks if this diverges from today's real path" up front.

## Requirements

Scoping synthesis (bee-hive routed straight to planning; user approved skipping full `bee-exploring`) plus two locked decisions:

- **D b300856d**: Embed the built web UI into the `herdctl` binary (`rust-embed`/`include_dir` — planning picks `rust-embed`+`axum-embed`, see approach.md). `config.static_dir` stays an optional dev/override path; embedded assets are the runtime default.
- **D 3168932d**: `install.sh` downloads a prebuilt binary from the latest GitHub release matching `release.yml`'s target matrix, falls back to `cargo`+`npm` build-from-source when no prebuilt asset matches. No checksum/signature verification (HTTPS-from-GitHub is the trust boundary, matches mdview's precedent).
- User: update `README.md` and `docs/usage.md` (scope widened during discovery to also include `docs/installation.md`, the actual page documenting install steps — `docs/usage.md` itself has no install-path content to update) to reflect the new install/packaging story.

## Discovery

L1 quick verify, findings cited (no separate discovery.md — small enough to keep inline):

- `axum-embed` 0.1.0 exists on crates.io, depends on `axum-core ^0.4` (matches this repo's `axum = "0.7"`, which itself depends on `axum-core 0.4`) and `rust-embed ^8` — compatible, drop-in for the current `ServeDir`/`ServeFile` pattern in `src/web/mod.rs:56-60`.
- `rust-embed` 8.12.0 embeds at compile time in release builds, reads from fs in debug by default (a `debug-embed` feature forces embedding always) — not used here; the repo's own `static_dir`-override-if-present logic covers the "serve something other than the compiled-in UI" need instead, so behavior stays identical between `cargo build` and `cargo build --release`.
- **Real coupling found, not anticipated in the original ask**: `src/main.rs:177-181` derives the sqlite state-file path from `config.static_dir.parent()`. This only resolves correctly today because `install.sh` happens to write `static_dir` as `$SHARE_DIR/static` (`~/.local/share/herdr-gateway/static`), so its parent is the app's actual data directory — which is also exactly what `packaging/herdr-gateway.service`'s `ReadWritePaths=%h/.local/share/herdr-gateway %h/.config/herdr-gateway` hardening expects writable. Once `static_dir` is optional and no longer installed to disk, this derivation breaks. See approach.md for the fix (`data_dir()`).
- **Build-order bug found**: `install.sh` and `dev-deploy.sh` both currently run `cargo build --release` *before* `npm run bundle`. Harmless today (static serving is runtime, from-disk). Becomes a real bug once embedding is compile-time: the binary would embed whatever `static/` contained *before* the bundle step (nothing, on a fresh checkout) rather than the freshly built UI. Both scripts need the two steps swapped.
- **CI dependency found**: `.github/workflows/release.yml` already bundles the web UI before building (`ci.yml`'s `rust` job does not — it never touches `web/`). `static/` is gitignored (`.gitignore:26`). Without a guard, adding `#[derive(RustEmbed)] #[folder = "static/"]` breaks every `cargo test`/`clippy` run in CI's `rust` job and on a fresh clone, because the folder won't exist. Fix: a `build.rs` that `create_dir_all("static")` before the derive macro runs (`cargo:rerun-if-changed=static` so real content is picked up when it later appears) — decouples the Rust build from Node entirely; `ci.yml`'s `rust` job needs no change.
- No GitHub release/tag exists yet for this repo (`git tag -l` empty, `gh release list` empty) — the "successful prebuilt download" branch of the new `install.sh` is real code but cannot be exercised end-to-end this session. The fallback branch (download fails → build from source) *can* be fully exercised for real, since it's genuinely today's live condition.

## Approach

See `docs/history/embed-and-package-binary/approach.md` (high-risk lane — recommended path, rejected alternatives, risk map, file order, open questions for validating).

## Shape — epic map

Feature outcome: `herdctl` runs as a single self-contained binary (no external `static/` needed to serve the UI); `install.sh` gets a working install without a Rust/Node toolchain when a prebuilt release exists; docs describe the real flow.

Repo-reality basis: no release has ever been cut from this repo, so the "download succeeds" path is new territory verified only by code inspection + the crates' own docs, not a live run.

| Epic | Capability/Risk Area | Why It Exists | Slices | Proof Needed |
|---|---|---|---|---|
| E1 | Compile-time asset embedding + data-dir decoupling | Core of D b300856d; unlocks everything else | current slice (cells 1-2) | `cargo test`, `cargo clippy -D warnings`, fresh-clone compile check |
| E2 | Install/dev script rework + CI packaging | Core of D 3168932d | current slice (cells 3-4) | `bash -n`, real fallback-path run of `install.sh` |
| E3 | Docs | User's explicit ask | current slice (cell 5) | read-through against actual new behavior |

Single current slice — all three epics are small enough, and tightly enough coupled (docs describe E1+E2's actual behavior; E2's release.yml change only makes sense once E1's embedding lands), that splitting into multiple slices would just add handoff overhead without a real go/no-go gate between them.

## Test matrix

High-risk lane — probes per dimension (12 edge dimensions, only the ones that apply; the rest are genuinely N/A for a packaging change and are named as such, not silently skipped):

| Dimension | Probe |
|---|---|
| Empty/missing input | Fresh clone with no `static/` dir at all — `cargo test`/`clippy` must still pass (build.rs guard) |
| Boundary values | N/A — no numeric boundaries in this change |
| Malformed input | `install.sh` run against a platform/arch combo not in the release matrix (e.g. force an unrecognized `uname -m`) — must fall through to source build, not crash |
| Concurrency | N/A — no new concurrent paths |
| Failure/partial failure | `curl` download genuinely fails (no release exists) — must fall back cleanly, not leave a partial/corrupt binary installed |
| Idempotency | Re-run `install.sh` twice — second run must not clobber existing config/secrets (existing behavior, must stay true) |
| Backward compatibility | `data_dir()` must resolve to the exact path today's `install.sh` defaults already use, so existing sqlite state is found, not orphaned |
| Config/flag interaction | `config.static_dir` pointing at a real dir with `index.html` must still win over embedded assets (dev override) |
| Observability | `herdctl doctor`'s "web UI" check must report something accurate for both the embedded-only and disk-override cases, not just "missing" |
| Security boundary | No checksum verification is intentional (D 3168932d) — confirm this isn't silently escalated (e.g. no accidental `curl | sh`-style unchecked execution beyond what's decided) |
| Resource exhaustion | N/A — no new unbounded loops/allocations |
| Cross-platform | Target-detection logic (`uname -s`/`uname -m` → musl/darwin target strings) must match `release.yml`'s exact matrix strings |

## Out of scope

- Cutting an actual GitHub release/tag to exercise the download-success path end-to-end — that's a real, visible, hard-to-reverse action (`git push --tags`) and stays a decision for the user to make explicitly after this lands, not something this feature does autonomously.
- Windows support in `install.sh` (it's already bash/systemd-only, matching mdview's own Linux+Darwin-only install.sh — Windows users are out of scope for a systemd-user-service installer regardless of this feature).
- Checksum/signature verification of downloaded binaries (explicitly rejected, D 3168932d).
- Rewriting the install UX to a standalone `curl | sh` model that doesn't assume a git checkout (rejected in approach.md — out of scope for "packaging parity" as asked).

<!-- implementation-ready additions (after Gate 2): -->

## Current slice

Slice: embed-and-package-binary (single slice, all 3 epics — see Shape rationale above).

Entry state: web UI served from an external `static/` dir at runtime; `install.sh`/`dev-deploy.sh` always build from source; sqlite path implicitly anchored to `static_dir`.

Exit state: `herdctl` binary carries the built UI at compile time with a disk-override fallback; sqlite path explicitly anchored to a new `data_dir()` (byte-identical to today's real path for default installs); `install.sh` tries a prebuilt-release download first, falls back to (correctly-ordered) source build; `dev-deploy.sh` reordered; `release.yml` no longer ships `static/` in the tarball; README/installation docs describe the new flow.

Files bounded: `Cargo.toml`, `build.rs` (new), `src/config/mod.rs`, `src/main.rs`, `src/web/mod.rs`, `src/doctor.rs`, `install.sh`, `dev-deploy.sh`, `packaging/herdr-gateway.service`, `.github/workflows/release.yml`, `README.md`, `docs/installation.md`, `docs/deployment.md`.

Verify commands: `cargo test --quiet && cargo clippy --quiet -- -D warnings` (Rust cells); `bash -n install.sh && bash -n dev-deploy.sh` plus a real `install.sh` run exercising the fallback branch (shell cells); full recorded `commands.verify` before the slice caps.

## Cells

Created via `bee.mjs cells add` — see `.bee/cells/embed-pkg-*.json`. Index:

- `embed-pkg-1` — Cargo deps, `build.rs` guard, `data_dir()`, sqlite path decoupling.
- `embed-pkg-2` — Compile-time UI embedding in the router, disk-override-or-embedded logic, doctor check update. Depends on `embed-pkg-1`.
- `embed-pkg-3` — `install.sh` rework, `dev-deploy.sh` reorder, stale org-name fix in the systemd unit.
- `embed-pkg-4` — `release.yml` tarball simplification. Depends on `embed-pkg-2`.
- `embed-pkg-5` — Docs (`README.md`, `docs/installation.md`, `docs/deployment.md` if it needs a touch). Depends on `embed-pkg-2`, `embed-pkg-3`, `embed-pkg-4`.

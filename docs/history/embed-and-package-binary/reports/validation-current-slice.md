# Validation report — embed-and-package-binary (current slice)

## Reality Gate Report

```text
REALITY GATE REPORT
Mode: high-risk
Current work: embed the web UI into the herdctl binary, rework install.sh/dev-deploy.sh/release.yml around it, update docs
MODE FIT: PASS       — 5 risk flags counted in plan.md (external systems, public contracts, cross-platform, existing covered behavior, multi-domain); no smaller lane honestly covers a compile-time build dependency change + a data-path migration + 2 untested shell scripts
REPO FIT: PASS        — every named file/API confirmed to exist: src/main.rs:177-181 (static_dir.parent() sqlite derivation), src/config/mod.rs (config_dir(), no data_dir() yet), src/web/mod.rs:56-60 (router/ServeDir/ServeFile), src/doctor.rs:165-179, install.sh, dev-deploy.sh, packaging/herdr-gateway.service:3 (stale thanhsmind org), .github/workflows/release.yml, README.md:75, docs/installation.md, .gitignore:26 (static/ gitignored). Crates confirmed via crates.io API: axum-embed 0.1.0 (deps axum-core ^0.4, rust-embed ^8, mime_guess ^2 — compatible with this repo's axum 0.7), rust-embed 8.12.0. ServeEmbed's with_parameters(fallback_file, fallback_behavior, index_file) + FallbackBehavior::{NotFound,Redirect,Ok} confirmed via docs.rs struct page.
ASSUMPTIONS: PASS     — every blocking assumption listed in the feasibility matrix below
SMALLER PATH: PASS    — considered and rejected: hand-rolled include_str! (mdview's approach) — herdr-gateway's UI is a full built SPA tree with hashed filenames, not a handful of CSS/JS files, so rust-embed+axum-embed is the smaller path here, not the larger one. Considered leaving the static_dir/sqlite coupling alone — rejected, it is a real latent bug once static_dir becomes optional, not precautionary scope creep. Considered skipping dev-deploy.sh — rejected, leaving its build-then-bundle order wrong would silently embed a stale/empty UI in the primary documented dev workflow.
PROOF SURFACE: PASS after 1 repair — embed-pkg-4's verify was `grep -c 'cp -r static' ...`, which exits 1 (interpreted as cell-verify failure) precisely when the assertion holds (zero matches) — inverted the pass/fail signal. Fixed to `! grep -q 'cp -r static' .github/workflows/release.yml` during validating (repair routing: broken verify command). All 5 cells' verify commands now runnable in this repo today.
Decision: proceed
Evidence: see REPO FIT citations above; embed-pkg-4 fix applied via `bee cells update`; cells schedule computed clean (see Feasibility Matrix, dependency row)
```

## Feasibility Matrix

```text
FEASIBILITY MATRIX
Assumption | Risk | Proof Required | Evidence | Result
axum-embed/rust-embed are real, compatible crates | MEDIUM | crates.io dependency graph vs this repo's axum version | Fetched crates.io API for both crates: axum-embed 0.1.0 deps axum-core ^0.4 + rust-embed ^8; this repo's axum 0.7 uses axum-core 0.4 internally — compatible | READY
ServeEmbed supports SPA-style fallback-to-index.html | MEDIUM | Real API surface, not docs prose alone | docs.rs struct page for ServeEmbed: with_parameters(fallback_file: Option<...>, fallback_behavior: FallbackBehavior, index_file: Option<...>), FallbackBehavior::Ok exists for this exact purpose. Exact param types/order still need confirming against the vendored crate at implementation time — embed-pkg-2's action text already instructs this explicitly rather than letting the worker guess | READY WITH CONSTRAINTS
build.rs create_dir_all guard prevents fresh-clone compile failure | LOW | rust-embed's documented behavior when #[folder] target is empty vs missing | rust-embed's proc-macro reads the folder via fs::read_dir; an existing-but-empty dir does not error (embeds zero files), only a genuinely missing dir errors. build.rs guarantees existence before the derive macro runs, cargo:rerun-if-changed=static picks up later real content | READY
build.rs rerun-if-changed=static triggers rebuild on nested file changes | MEDIUM | Cargo's own rerun-if-changed semantics for directories | Flagged to the tech panel (background) — cargo's rerun-if-changed on a directory path watches the directory's own mtime, which most filesystems update when files inside are added/removed/renamed but NOT on pure content-modification of an existing file without a rename; vite's build always empties+rewrites the whole static/ dir (emptyOutDir: true) which does change directory entries each build, so this is expected to work for the real bundle workflow, but is a genuine edge worth the panel's read | READY WITH CONSTRAINTS — pending panel finding
data_dir() resolves identically to install.sh's current default sqlite path | MEDIUM (data-loss if wrong) | Path-string equality by inspection | install.sh: PREFIX default $HOME/.local, SHARE_DIR=$PREFIX/share/herdr-gateway → $HOME/.local/share/herdr-gateway. XDG spec default for XDG_DATA_HOME is $HOME/.local/share. data_dir() = ${XDG_DATA_HOME:-$HOME/.local/share}/herdr-gateway → identical string for the default (no PREFIX override) case, which is what today's documented install.sh flow always produces | READY
install.sh download branch is untestable end-to-end this session | MEDIUM (accepted, not a blocker) | git tag -l / gh release list | Both empty — confirmed no release exists for this repo. The fallback-to-source branch IS fully testable (it's today's real condition) and is what embed-pkg-3's verify command (`./install.sh`) actually exercises | READY WITH CONSTRAINTS (documented limitation, not a gap)
Cell dependency graph is a valid DAG matching each cell's real needs | LOW | `bee cells schedule` | Computed: waves [[embed-pkg-1,embed-pkg-3],[embed-pkg-2],[embed-pkg-4],[embed-pkg-5]], zero cycles, zero unsatisfiable deps, zero empty-files diagnostics | READY
packaging/herdr-gateway.service ReadWritePaths covers the new data_dir() | MEDIUM (systemd would silently block the write otherwise) | Inspect the unit file's ReadWritePaths against data_dir()'s resolved path | ReadWritePaths=%h/.local/share/herdr-gateway %h/.config/herdr-gateway — data_dir() resolves to exactly %h/.local/share/herdr-gateway (see row above) — inside the allowed set | READY
```

## Persona Panel (high-risk lane) — iteration 1, structurally clean after repair

Three background reviewers (decision 0017): **panel-tech** (coherence + feasibility + security), **panel-scope** (product + scope-guardian), **cell-reviewer** (cold-pickup cell review). All three read `plan.md`, `approach.md`, and every cell's full JSON; all three independently spot-checked claims against real repo files (not taking the plan's word for current state).

### BLOCKERs found and fixed

1. **(panel-scope, product) install.sh's toolchain prereq gate ran before the download attempt.** `install.sh:22-23` hard-exits if `cargo`/`npm` are missing, before any code that would try the new download path — on exactly the toolchain-less machines D 3168932d's download-first design targets, install would fail at line 22 and never reach the download. **Fix:** `embed-pkg-3`'s action rewritten to explicitly require relocating that check into the source-build fallback branch only; a new must-have truth (`"The cargo/npm prerequisite check no longer runs before the download attempt"`) and prohibition (`"not deleted, only relocated"`) make this checkable at cap time.
2. **(panel-tech, feasibility) `embed-pkg-2`'s verify command ran `cargo test` before the web bundle existed.** Original: `cargo test --quiet && cargo clippy ... && (cd web && npm run bundle) && cargo test --quiet`. The first `cargo test` runs while `static/` is still empty (only `build.rs`'s `create_dir_all` has run) — any test asserting the embedded-fallback truth ("GET / returns embedded index.html") would fail there, aborting the `&&` chain before the bundle step ever runs. **Fix:** reordered to `(cd web && npm run bundle) && cargo test --quiet && cargo clippy --quiet -- -D warnings` — bundle populates `static/` before the one `cargo test` invocation that needs to see real content, consistent with `debug-embed` always embedding at compile time (see below).

### WARNINGs found and fixed

3. **(panel-tech, coherence) `debug-embed` choice undocumented in approach.md**, leaving `embed-pkg-1`'s use of the `debug-embed` feature unexplained next to the router's disk-override logic. **Fix:** added an explicit sentence to approach.md's Recommended path: `debug-embed` is enabled so embedding always happens at compile time regardless of profile (deterministic, testable under plain `cargo test`), independent of the router's own disk-override mechanism.
4. **(panel-scope, product) `embed-pkg-5` under-scoped doctor-output doc surfaces.** `embed-pkg-2` rewrites `doctor.rs`'s web-UI check; `docs/installation.md`'s sample doctor output (~line 68) and troubleshooting table's `✗ web UI: no built UI` row (~line 104) would go stale. **Fix:** `embed-pkg-5`'s action and must-haves extended to require updating both surfaces against `embed-pkg-2`'s actual (not guessed) new wording.
5. **(panel-tech, security) download hygiene gap.** `embed-pkg-3` didn't require quoting `HERDCTL_VERSION` in the URL, `--proto '=https'` (so a 404 HTML body is never mistaken for a binary), or isolated-temp-dir extraction. Low severity (`HERDCTL_VERSION` is operator-set, not attacker-controlled — not a privilege boundary) but cheap due diligence. **Fix:** added as an explicit must-have truth and artifact substantive on `embed-pkg-3`.
6. **(cell-reviewer, minor) `embed-pkg-5`'s original verify (`grep -n 'static_dir' ...`) was a no-op gate** — it would pass whether or not the new wording was actually written. **Fix:** verify now asserts the new content exists (`grep -q 'prebuilt' README.md docs/installation.md`).
7. **(cell-reviewer, minor) `embed-pkg-3`'s verify is host-mutating** (`./install.sh` really installs a systemd user service on this machine) — flagged as inherent to the plan's "exercise the real fallback branch" intent, not a defect. **Fix:** added an explicit note in the cell's action naming the Rust/Node/systemd prerequisites, so a cold worker isn't surprised.

### Findings not adopted (with reason)

- **(panel-tech, coherence) claimed contradiction "approach.md:26 states debug-embed is 'not used here'"** — checked the actual file: line 26 is the `data_dir()` risk-map row, not a debug-embed statement; approach.md never made that claim at all (it simply hadn't discussed `debug-embed` yet). The underlying gap (undocumented choice) was real and is fixed above (#3); the specific citation was not reproducible against the file and is not treated as a confirmed BLOCKER-adjacent contradiction.
- **(panel-tech, feasibility, WARNING) `build.rs`'s `cargo:rerun-if-changed=static` may not reliably re-trigger embedding on pure content-only changes inside already-existing files** — accepted as a known, low-severity limitation (approach.md's risk map already rates this LOW): vite's `emptyOutDir: true` deletes and recreates every file on each real bundle, which does change directory entries; residual risk is confined to unusual incremental-rebuild edge cases, not the normal bundle→build flow this repo actually uses. Not fixed — documented instead, matching SMALLER PATH reasoning (a more robust file-level watch would be over-engineering for this edge case).
- **(panel-scope, product, WARNING) download-success branch ships with zero end-to-end proof; "mdview parity" is partial (still requires `git clone`)** — both already explicitly named as accepted, disclosed limitations in `plan.md`'s Out of scope section and `approach.md`'s Rejected alternatives before the panel ran; not new findings, no action needed beyond what's already documented.
- **(panel-scope, scope-guardian, WARNING) systemd unit org-name fix (thanhsmind→vantt) is unrelated scope folded in silently** — correct observation; not undone (it's a genuine one-line factual bug in a file this slice already touches, and reverting it would leave a known-wrong URL), but surfaced explicitly here rather than left implicit, and `embed-pkg-3`'s action now calls it out as "a separate line item when reporting the diff, not folded silently into the install.sh changes."

### Re-verification (self-checked, not re-dispatched — narrow mechanical fixes)

- `embed-pkg-2` verify confirmed via `cells show`: `(cd web && npm run bundle) && cargo test --quiet && cargo clippy --quiet -- -D warnings` — bundle now precedes both cargo invocations.
- `cells schedule` re-run after all edits: waves `[[embed-pkg-1,embed-pkg-3],[embed-pkg-2],[embed-pkg-4],[embed-pkg-5]]`, zero cycles, zero unsatisfiable deps — unchanged and still valid (edits touched `action`/`must_haves`/`verify` only, never `deps`).
- No re-dispatch of the persona panel: iteration-1 findings were narrow, mechanically checkable fixes (a verify-command reorder, a doc-sentence addition, a must_haves extension) rather than structural redesigns, so a second adversarial pass was judged not to add signal proportionate to its cost. Per the max-3-iteration rule, this is iteration 1 of at most 3; nothing here is a repeat failure that would force iteration 2.

## Cell Review

0 CRITICAL flags (both original MINOR flags fixed — see #6, #7 above). All 5 cells confirmed cold-pickup-implementable by an independent fresh-eyes reviewer with spot-checks against real file line numbers.

## Approval Block

```text
VALIDATION COMPLETE - APPROVAL REQUIRED BEFORE EXECUTION
Mode: high-risk
Work: embed-and-package-binary current slice (5 cells)
Reality gate: PASS
Feasibility: READY WITH CONSTRAINTS (axum-embed with_parameters exact signature confirmed at implementation time per embed-pkg-2's action; install.sh download-success branch untestable pending a real GitHub release, per plan.md Out of scope)
Structure: PASS after 1 iteration (2 BLOCKERs found and fixed, 5 WARNINGs — 3 fixed, 2 accepted-as-documented)
Spikes: none run (all assumptions resolved by inspection/API evidence: crates.io dependency graph, docs.rs struct API, path-string equality, systemd ReadWritePaths coverage)
Cell review: PASS (5 cells, 0 CRITICAL open, 2 original MINOR flags fixed)
Unresolved concerns: none blocking. Accepted-and-documented: build.rs rerun-if-changed edge case (LOW), install.sh download-success branch unverified until a real release exists (inherent, disclosed)
```

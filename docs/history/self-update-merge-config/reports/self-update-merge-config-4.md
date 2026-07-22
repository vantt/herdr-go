# self-update-merge-config-4 — Fetch latest GitHub release metadata

**Status:** [DONE]

**Outcome:** Added `src/update/github.rs` — a pure `parse_tag_name` over the release response body, a thin (intentionally untested) async `fetch_latest_tag` wrapper hitting `GET https://api.github.com/repos/vantt/herdr-go/releases/latest` (D1), and `check_for_update` composing the fetched tag with the existing version-compare logic (D2). Declared `pub mod github;` in `src/update/mod.rs`.

**Files touched:**
- `src/update/github.rs` (new)
- `src/update/mod.rs` (added `pub mod github;`)

**Verify:** `test -f … && grep … && cargo test --quiet update` → 10 passed / 0 failed (6 existing compare tests + 4 new pure-parser tests, no network). `cargo clippy --lib` clean.

Full trace and evidence: `.bee/cells/self-update-merge-config-4.json`.

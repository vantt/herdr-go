# web-create-endpoints-2

**Status:** [DONE]

**Outcome:** Made `cwd` optional (`Option<&str>`) on `tab_create` and `agent_start` across the trait, `SocketHerdr`, and `FakeHerdr` — `None` omits the wire `cwd` key entirely; documented the tab.create-vs-agent.start omit-cwd asymmetry on the trait; grew the fake seed with a shell-only workspace whose anchor is cwd-only, and made the fake return `agent_placement_not_found` for an unknown workspace on `agent.start` (matching the live client).

**Files touched:**
- `src/herdr/mod.rs` — trait signatures + asymmetry doc comment on `agent_start`
- `src/herdr/socket.rs` — `tab_create_params`/`agent_start_params` omit key on `None`; signatures; 2 new `createcwd_` param tests
- `src/herdr/fake.rs` — seed w3 (shell-only, cwd-only anchor); `agent_start` unknown-workspace → `agent_placement_not_found`; `tab_create(None)` resolves anchor; 4 new/updated `createcwd_` tests

**Verify:** `VERIFY_PASS` — `cargo test --quiet` 231 passed; `cargo fmt --all --check` clean; `cargo clippy --all-targets -- -D warnings` 0 warnings; `cargo test --lib -- createcwd_` 6 passed.

**Full trace/evidence:** `.bee/cells/web-create-endpoints-2.json`

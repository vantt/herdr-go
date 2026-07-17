---
artifact_contract: bee-plan/v1
artifact_readiness: implementation-ready
mode: high-risk
---

# Plan — m1-gateway-web-terminal

## Mode gate

Risk flags counted: **auth** (web session), **audit/security** (security boundary, the single gate to herdr's no-auth socket), **external systems** (herdr CLI/socket), **data model** (sqlite store), **public contracts** (web API, WS protocol), **multi-domain** (Rust backend + TS frontend + installer). 6 flags, multiple hard-gate → **high-risk**. Greenfield → **init cell first**.

Smaller modes insufficient: this is a multi-domain greenfield app crossing a security boundary; tiny/small/standard cannot honestly protect auth + external-socket + data-model work simultaneously.

## Discovery

L0/L1 — no new research needed. Architecture, stack, herdr wire behavior all locked (CONTEXT.md + DISCOVERY.md + decision log). Toolchain verified: rustc/cargo 1.96.1, node 24.18.0, npm 11.16.0. Reference implementation `upstreams/airemote` (Go) provides dogfooded patterns already inventoried in `docs/distillery/porting-log.md`.

## Approach

Single binary crate `herdctl` (module-per-concern, `internal/`-style visibility, no premature workspace split — YAGNI, matches airemote's single-binary shape). Frontend a separate `web/` npm package (vite + xterm.js) built to static assets embedded/served by axum. Hexagonal seams as ports (`HerdrControl`, `HerdrStream`, `EventSource`, `Notifier`, `Store`); everything else concrete. `Fake` herdr adapter makes the whole app runnable and end-to-end testable with no live herdr — the primary testing substrate.

Dependency direction (one-way): `web`/`telegram` → `core` → traits ← `herdr`/`store` impls; `security` called everywhere, calls nothing; `main.rs` wires all.

Rejected: teloxide (per `afbc6161`); multi-crate workspace (no real boundary need yet); routing Tier 2 relay through core (breaks transparent-pipe design `4e3ef1a1`).

Risk map:
- Tier 2 WS relay ↔ HerdrStream framing — MEDIUM — proven at protocol layer (PBI-002); Fake adapter emits real-shaped frames for e2e.
- Auth fail-closed silent-404 — MEDIUM — adversarial tests (unauth → 404 no-leak).
- Path-allowlist / slug — MEDIUM — adversarial + mutation tests (airemote proof shapes).
- sqlite concurrency — LOW — WAL + busy-timeout, single-writer.

## Slices

- **S0 — init** (init cell): `herdctl` Cargo project + `web/` npm skeleton, module stubs compile, one passing test each side, `.bee/config.json` `commands.verify` recorded, clean first commit. Verify = the recorded test command.
- **S1 — core pure**: `config` (serde deny_unknown_fields, allowed-roots fail-closed, token-env-only) + `security` (path-allowlist 7-step, slug byte-level, redactor) with adversarial tests.
- **S2 — herdr port**: wire types (frame schema 7 fields, event envelopes), `HerdrControl`+`HerdrStream` traits, `Fake` adapter (emits frames/snapshots), `Cli` adapter (subprocess, `--session`, protocol pin). Integration tests via Fake.
- **S3 — services**: `store` (sqlite offset+delivered, WAL, migrations) + `supervisor` (health-check loop) + `watcher` (poll EventSource + cursor de-dup).
- **S4 — web backend**: axum auth (static token + cookie, silent-404), switcher API (snapshot→agent list+status), Tier 2 WS relay (xterm.js↔HerdrStream, hybrid rotate).
- **S5 — frontend**: TS+xterm.js mobile-first — login → switcher (portrait, status badges) → landscape terminal. Beautiful, responsive, theme-aware.
- **S6 — integration + ship**: `main.rs` composition root, `install.sh` + systemd user unit, full usage docs (README + docs/usage), e2e test (Fake herdr → relay → frame round-trip).
- **S7 — stretch**: Telegram notify (poll + de-dup + at-least-once) if time remains.

## Test matrix (against 12 edge dimensions, high-risk depth)

- Input validation: path traversal, sibling-prefix, symlink-planted, empty slug, foreign bytes, unknown config keys, empty allowlist → all fail-closed (adversarial suite).
- Auth: unauth request → 404 no-leak; wrong token → 404; valid → cookie; button/payload revalidated.
- Concurrency: sqlite multi-writer under load; single-writer control `--takeover`.
- Protocol: version mismatch → typed error; EOF without terminal.closed → reconnect; seq order applied; subscribe replay de-duped.
- Failure: herdr down → supervisor restart; relay disconnect → full-frame resync.
- e2e: Fake herdr drives idle→working→blocked→done; frame round-trips to a headless xterm parse.

## Acceptance criteria

`cargo test` + `cargo clippy -- -D warnings` + `npm run build` + `npm test` all green; app boots against Fake herdr and serves the UI; installer produces a working systemd unit; docs let a stranger install + use it.

## Open questions for validating

None blocking — all MEDIUM risks have a named proof above. Validating confirms the toolchain crate versions resolve and the Fake-adapter e2e shape is real.

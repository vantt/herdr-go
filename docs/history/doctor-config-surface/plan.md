---
artifact_contract: bee-plan/v1
mode: high-risk
feature: doctor-config-surface
context: docs/history/doctor-config-surface/CONTEXT.md
approved_gate2: 2026-07-20
---

# Doctor as Config Surface — Plan

## Mode Gate Record

**Risk flags counted: 6 — including a hard-gate flag → `high-risk`.**

| Flag | Present | Evidence |
|---|---|---|
| audit/security | **yes (hard gate)** | Writes secret tokens and `allowed_roots`, the app's filesystem access-control boundary (`src/config/mod.rs:766-814`, `:895-901`). |
| auth | **yes** | The web session secret is auth material; D8 adds a new startup path that injects it. |
| public contracts | **yes** | `doctor`'s documented read-only guarantee (`docs/specs/installation.md:254-270`) and its exit code (`src/main.rs:124-127`) both change. |
| cross-platform | **yes** | Owner-only file writes differ per OS (unix mode 600 vs Windows ACL); TTY detection and D8's env-file load must work on both. |
| changes behavior an existing test asserts | **yes** | `main_migration_seam_obeys_the_cli_mode_matrix` constrains doctor's write paths (D12). |
| multi-domain | **yes** | CLI surface + config layer + secret layer + docs/spec. |
| data model | no | The 8-field config schema is unchanged; only who writes it changes. |
| external systems | no | herdr socket interaction stays diagnostic-only. |
| validation removal | no | D6 explicitly preserves fail-closed validation. |

**Product files in scope: ~8** (`src/doctor*`, `src/config/mod.rs`, `src/main.rs`, `README.md` + spec are docs but the Rust surface alone exceeds every small-lane cap).

**Why smaller modes are insufficient:** `tiny`/`small` cap at 0–1 flags and ≤3 product files; this has 6 flags. Any hard-gate flag alone forces `high-risk` regardless of size — and this one writes secrets and widens a security boundary. `spike` does not apply: no single yes/no proof decides whether the plan is real; feasibility is established (see Discovery), the risk is in the writes themselves.

## Discovery (L2 — prompt mechanism)

Greenfield: repo has **no** prompt crate, no TTY crate, and no existing interactive pattern anywhere in `src/` (the only `stdin` hit is subprocess suppression at `src/supervisor.rs:47`). Cargo.toml is edition 2021, no MSRV pin, and every dependency maps to one concrete need — a lean, purpose-built philosophy.

| Candidate | Verdict |
|---|---|
| **stdlib: `std::io::IsTerminal` + `stdin().read_line`** | **Chosen.** Zero new dependencies. `IsTerminal` is stable since Rust 1.70 and in std, so D5's TTY detection costs nothing. The prompt shapes actually needed are few and simple: yes/no confirm, line input with default, typed confirmation, choose-from-numbered-list. |
| `dialoguer` | Rejected. Pulls `console` + a transitive tree for ~4 prompt shapes; disproportionate for the need and against the repo's one-crate-per-concrete-need norm. |
| `inquire` | Rejected. Same reasoning, heavier still. |
| `rpassword` (secret entry only) | **Open for validating.** Stdlib cannot suppress terminal echo, so typing a github/telegram token would display it on screen. This is narrower than D6 (which governs never echoing values *back*), but it is a real shoulder-surfing exposure. Options: add `rpassword` (tiny, single-purpose, cross-platform) — recommended; or accept visible echo with an explicit warning; or accept a file path instead of a pasted value. Validating picks one with evidence. |

## Approach

Turn `doctor` from a linear print-only report into **diagnose → offer fix → re-check**, plus a settings editor, while keeping the non-interactive path byte-identical to today.

**Chosen path.** Build the three write/IO foundations first, each independently testable, *before* wiring any interactive flow into the checks. This keeps the security-sensitive layers (config repair, secret writes, env-file loading) under unit test on their own, rather than only reachable through a TTY session that tests cannot easily drive.

**Module structure.** `src/doctor.rs` (~305 lines today) grows well past a single file. Split to `src/doctor/`: `mod.rs` (orchestration + `run()`), `checks.rs` (the 10 checks), `prompt.rs` (TTY detection + prompt primitives), `fix.rs` (guided fixes), `edit.rs` (settings editor). **`tests/rename_contract.sh:29` tracks `src/doctor.rs` in its source-surface inventory and must be updated in the same cell as the split**, or the contract test flags drift.

**Rejected alternatives.**
- *Separate `config` subcommand* — rejected by D1 (user chose a single entry point).
- *Rewrite the check engine into a trait-object registry* — rejected as YAGNI; the imperative `Vec<Check>` builder works, and re-running checks is achievable without an abstraction rewrite.
- *Full re-run of `run()` after every fix* — rejected as the default: it re-pings herdr and re-reads everything per fix. Applied once at the end instead (D11). See the socket-chain caveat below.

**Risk map.**

| Component | Risk | Proof required before execution closes |
|---|---|---|
| D8 startup load of `herdr-go.env` | **HIGH** | New startup contract + security surface. Tests: process env always wins; file ignored when `validate_token_protection` fails; absent file is not an error; no secret value ever logged. |
| Config field-by-field repair (D7) | **MEDIUM** | Data-loss risk. Tests: valid fields preserved; unparseable JSON takes the backup path; backup file actually written before any overwrite; invalid result never persisted (D6). |
| `allowed_roots` guard (D9) | **MEDIUM** | Security boundary. Tests: `/`, `$HOME`, and a symlink each demand typed confirmation; plain narrow path does not. |
| Non-TTY parity (D5) | **MEDIUM** | Greenfield detection. Proof: piped invocation produces the same report and same exit code as today, with zero writes. |
| Secret replace-not-append (D10) | **MEDIUM** | Today's `ensure_web_secret` only appends (`src/config/mod.rs:756-762`). Test: rewriting an existing key yields one line, not two. |
| doctor module split | LOW-MED | `bash tests/rename_contract.sh` green. |
| Re-check + exit code (D11) | LOW | Test: exit reflects post-fix state. |

**Nothing in this repo invokes `doctor` non-interactively** — zero matches across CI, release, `install.sh`, `dev-deploy.sh`. D5's blast radius today is therefore low, but the requirement stays locked: it protects future scripted use and is the property that keeps D12/`4827aae8` honest.

## Slices

Cells exist for **Slice 1 only**. Later slices are mapped, not created.

### Slice 1 — Write foundations (current)

| Cell | Scope |
|---|---|
| `doctor-config-surface-1` | Module split + `prompt.rs`: TTY detection, prompt primitives, and non-TTY parity guaranteed. No behavior change yet. |
| `doctor-config-surface-2` | Config write/repair layer: atomic validated write, field-by-field repair, backup fallback, `allowed_roots` guard. |
| `doctor-config-surface-3` | Secret layer: replace-not-append writer + D8 startup env-file loading with precedence and permission gating. |

### Slice 2 — Interactive surface (not yet created)

Wire guided fixes into each check (D3), add the all-8-field settings editor, apply re-check + exit-code semantics (D11). Depends on all of Slice 1.

*Known structural caveat for Slice 2:* the socket/endpoint chain (checks 3–5, `src/doctor.rs:89-141`) has an early return that prints and exits on socket-resolution failure, and check 5 depends on the `socket` value derived in check 3/4. Fix-then-recheck is straightforward for checks 1, 2/3-config, 6, 7, 9, 10; the socket chain needs the early return restructured or an explicit full re-run. Validating should confirm the approach before Slice 2 cells are written.

### Slice 3 — Docs & spec sync (not yet created)

Rewrite `docs/specs/installation.md:254-270` (the read-only guarantee D1 supersedes, plus D11 exit semantics); update `README.md:100-127` config table — **adding the missing 8th field `telegram_chat_id`** — and `:159-177` troubleshooting/token guidance for D8; update `print_help()` (`src/main.rs:84-106`). This is the "update the guide docs" the request asked for.

## Test Matrix (edge dimensions)

| Dimension | Cases |
|---|---|
| Input validity | valid value; invalid value re-prompts; empty input takes the default |
| Boundary | `allowed_roots` = `/`, `$HOME`, symlink, narrow real dir; unparseable vs merely-invalid JSON |
| State | no config; valid config; invalid config; no env file; env file with existing key; env file with bad permissions |
| Environment | TTY vs piped/redirected stdin; unix vs Windows permission paths |
| Precedence | process env vs `herdr-go.env` (env must win) |
| Idempotence | writing the same key twice yields one line, not two |
| Failure | write fails mid-way; backup fails; permission validation fails → file distrusted, not silently used |
| Security | no secret value in any output, log, or error; migration never triggered (D12) |
| Contract | exit code pre/post fix; non-TTY report identical to today |

## Open Questions For Validating

- [ ] Secret entry echo: add `rpassword`, warn-and-echo, or accept a file path? (Discovery table above.)
- [ ] Socket-chain restructuring for fix-then-recheck — confirm before Slice 2.
- [ ] Whether an explicit `--non-interactive` / `--yes` flag is warranted on top of D5's automatic TTY detection (Agent's Discretion in CONTEXT.md).

## Baseline

Recorded green at plan time: `cargo test --quiet` all pass, `cargo clippy --quiet -- -D warnings` silent, exit 0.

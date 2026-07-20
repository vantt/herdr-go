# Doctor as Config Surface — Context

**Feature slug:** doctor-config-surface
**Date:** 2026-07-19
**Exploring session:** complete
**Scope:** Deep
**Domain types:** CALL (CLI interface), ORGANIZE (config)

## Feature Boundary

Evolve the `herdr-go doctor` command from a read-only diagnostic report into the
single friendly, standard surface through which a user inspects **and** configures
every configuration capability of the app: each check that finds a problem offers
an inline guided fix, and the user can edit any of the 8 `config.json` settings and
set up the env-only secrets — all from one interactive command. The feature ends at
the CLI surface plus its guide docs; it does not add a web settings UI, and it does
not remove hand-editing.

## Locked Decisions

These are fixed. Planning must implement them exactly — cited, never reinterpreted.
Changing one requires the user, a new D-ID or an explicit supersession note, never a
silent edit.

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | `doctor` itself becomes the interactive surface: it diagnoses, then for each fixable problem offers a guided fix, and lets the user edit any setting — one command, not a separate `config`/`setup` sibling and not a web UI. This intentionally supersedes doctor's documented "read-only, never mutates" contract (`docs/specs/installation.md:254-270`). | User chose "through doctor" as a single entry point; most faithful to the request. |
| D2 | Config scope covers all 8 `config.json` fields **and** helping the user set up the env-only secrets (web/github/telegram tokens), which are written to the `herdr-go.env` file (mode 600), never into `config.json`. The "tokens never live in `config.json`" invariant (enforced by `RawConfig` `deny_unknown_fields` + the token-rejection test at `src/config/mod.rs:903-914`) is preserved. | User chose complete zero-to-running setup; security invariant must not regress. |
| D3 | A failed check offers its fix inline (e.g. "config missing → create it now?", "allowed_roots empty → add a root"), replacing the old print-only advice text with an actionable prompt. | Core of what "friendly" means here. |
| D4 | The interactive surface is **additive**: hand-editing `config.json` and `herdr-go.env`, `--config`, `--bind`, and auto-created defaults all keep working unchanged. | YAGNI / backward compatibility; no existing path removed. |
| D5 | Interactivity requires a TTY. When stdin/stdout is not a TTY (piped, CI, scripts), `doctor` degrades to today's behavior exactly: run all checks, print the report with fix hints, exit 0/1 — no prompts, no writes. | Automation and CI must never hang on a prompt or mutate state unattended. |
| D6 | **Config** writes re-run full config validation (`Config::load_str`/`load_file` path) before persisting, and refuse to save an invalid result. Secret values are never echoed back or printed; token diagnostics keep revealing only presence/absence, never the value. | Preserve fail-closed validation and the existing no-secret-leak guarantee. |
| D7 | An existing-but-invalid `config.json` is repaired **field-by-field**: doctor walks only the invalid fields and keeps every valid field the user already set. When the file cannot be parsed as JSON at all (no fields are recoverable), it falls back to timestamped-backup-then-recreate (`config.json.bak-<ts>`), never a silent overwrite. | User choice. Respects hand-edits (D4) while still fixing the most common failure; the unparseable case has no field-level repair path, so it needs the backup fallback. |
| D8 | The binary loads `herdr-go.env` at startup as a **fallback** source for all three secrets. Process environment always wins; the file is only consulted for keys absent from the environment, and `validate_token_protection()` must pass before the file is trusted. This is a deliberate new startup contract so secrets set through doctor work on direct runs and on Windows, not only under systemd. | Without it, D2's github/telegram token setup is silently inert outside systemd (`Secrets::from_env` reads the process env only, `src/config/mod.rs:53-58`; the env file reaches the process solely via `EnvironmentFile=` in `install.sh:95` / `dev-deploy.sh:30`). |
| D9 | A guided `allowed_roots` addition warns and requires an explicit typed confirmation when the path is over-broad — `/`, the user's home directory, or a symlink. Never silently accepted, never hard-refused. | `allowed_roots` is the app's primary security boundary, but config validation checks **only absoluteness** (`non_absolute_root_rejected`, `src/config/mod.rs:895-901`) — `Config::load_str` does not check existence at all; existence is a separate doctor-side check (`src/doctor.rs:155-170`). So validation alone would let one keypress widen the boundary to everything. Refusing outright would block legitimate use. |
| D10 | **Secret** writes do not use config validation (none applies to `herdr-go.env`). They reuse `prepare_token_directory` / `write_new_token` / `validate_token_protection` (`src/config/mod.rs:766-814`) for mode 600 on unix and owner-only ACL on Windows, and they **replace an existing key in place rather than appending**. | `ensure_web_secret` today only appends (`src/config/mod.rs:756-762`), so re-writing an existing key would produce duplicate lines. Permission handling must not be reinvented. |
| D11 | In interactive mode, after fixes are applied doctor **re-runs the checks** and its exit code reflects the **final** state, not the pre-fix state. Non-interactive exit semantics are unchanged (D5). | Exit code is doctor's only external contract (`src/main.rs:124-127`); a user-visible public contract, so it is locked here rather than guessed at by planning. |
| D13 | Secret entry is **masked** (terminal echo suppressed) using the `rpassword` crate — the one new dependency this feature adds. After entry, doctor confirms with length **and the last 3 characters only**, and only when the value is at least 12 characters long; shorter values show length alone. The leading characters are never shown (for Telegram the prefix is the bot id). Empty input means "skip, set it later" and never blocks progress. The full value never appears in output, logs, or error text, and a secret is **never** accepted as a command-line argument. | Masking is what users expect from any secret prompt; showing a short tail restores the ability to catch a bad paste, which full masking removes. Hand-rolling echo suppression would mean reimplementing a security-sensitive primitive across termios and the Windows Console API. Argv is readable from `/proc/<pid>/cmdline` and lands in shell history. |
| D14 | Doctor runs in **three phases**: diagnose every check read-only and collect all failures; offer fixes; then re-run **all** checks once and report. The early return in the socket-resolution check (`src/doctor.rs:97-105`) is removed — checks that cannot be computed because a prerequisite failed report as *skipped* rather than aborting the whole run. Re-running everything once, rather than re-running individual checks, is deliberate. | Today one socket error hides the seven checks after it, forcing a fix-one-thing-at-a-time loop; removing the early return is a UX fix independent of this feature. A full second pass cannot leave stale derived state (`socket` is derived from config, so a config fix invalidates it), and costs only one extra herdr ping on a human-invoked command. Supports D11's final-state exit code. |
| D15 | Add a `--check` flag meaning "diagnose only, change nothing" — it works even on a TTY and takes precedence over everything. Do **not** add `--yes` or any blanket auto-apply flag. Interactive mode requires **both stdin and stderr** to be a TTY; prompts are written to **stderr** while the report goes to stdout. Absent a TTY, behavior is read-only regardless of flags. | TTY auto-detection alone cannot serve a user at a terminal who only wants the diagnosis. `--yes` would auto-widen `allowed_roots` and write tokens unattended — exactly the unattended-mutation property decision `4827aae8` was created to prevent; any future non-interactive need should take explicit values instead. Checking stdin alone would let `doctor \| tee log` write prompts into the log file. |
| D16 | The settings editor shows the existing non-loopback warning **at the moment `bind_addr` is changed**, not only at startup. | The default bind is `0.0.0.0` (decision `e5d2c9f0`) and the guardrail is a startup warning; a warning delivered at the moment of the decision is the one that actually changes behavior. |
| D17 | The CLI keeps a **flat parser — no subcommand dispatch is added** — and exposes exactly **two** modes: bare `doctor` (diagnose → offer a fix per failing item → re-run) and `doctor --check` (diagnose only, never writes). There is **no `--edit`** shortcut; the settings editor is reached through a single end-of-run prompt ("edit a setting? [y/N]", default no). | The two flags were not equally justified. `--check` serves an intent auto-detection cannot express: a user at a TTY who does not want to be prompted. `--edit` had no unique justification — the end-of-run prompt reaches the same editor one keypress away. Its stated rationale (avoiding a slow 10-check pass) was measured and found false: unix socket connect fails immediately with no retry or timeout (`src/herdr/socket.rs:100-102`), the only retry is a bounded Windows named-pipe-busy loop (`:107-118`), and the remaining checks are filesystem stats. Removing a shipped flag is hard; adding one later is easy. |
| D12 | Interactive mode does **not** change doctor's exclusion from default-state migration: `migrate_default_state_if` must still never run in doctor mode, in any TTY/flag combination. | Test-enforced today (`main_migration_seam_obeys_the_cli_mode_matrix`, `src/main.rs` tests) and locked by decision `4827aae8` after a failure analysis found real state-changing side effects in read-only/demo modes. D1 permits user-consented config/secret writes only — never this migration. |

### Agent's Discretion

Delegated to planning (implementation choices, not product decisions):
- Whether to introduce real subcommand dispatch or drive interactivity from a
  flag/auto-TTY-detection on the existing `doctor` path (no CLI framework exists
  today — custom parser in `src/main.rs:53-82`).
- The exact prompt/TUI mechanism (inline stdin prompts vs a small prompt helper);
  no new heavy TUI dependency unless planning justifies it.
- Ordering of the fix-then-edit flow and how "edit any setting" is presented
  (menu, per-field walkthrough, etc.).
- Whether a `--yes`/`--non-interactive` style flag is added on top of D5's TTY
  auto-detection.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| Guided fix | An interactive prompt attached to a failing check that performs the corrective write (create config, add a root, write a token) instead of only printing advice. |
| Config surface | The full set of settings the user can inspect and change through doctor: the 8 `config.json` fields plus the three secrets in `herdr-go.env`. |

## Existing Code Context

From the quick scout only. Downstream agents read these before planning.

### Reusable Assets

- `src/doctor.rs` — the 10-check diagnostic engine (`pub async fn run() -> bool` at
  `:49`, `Check` struct with `ok`/`critical`/`label`/`detail`/`fix` at `:9-15`, and
  the module-private `print_report` at `:215`). The fix strings already name the
  corrective action per check — the raw material for D3's prompts.
- `src/config/mod.rs:766-814` — `prepare_token_directory` / `write_new_token` /
  `validate_token_protection`: the existing cross-platform owner-only write path
  (unix mode 600 via `OpenOptionsExt`, Windows ACL/SID via the internal `windows`
  module). D10 reuses these rather than reinventing permission handling.
- `src/config/mod.rs` — `Config`/`RawConfig` (8 fields), `load_str`/`load_file`
  validation collecting all errors (D6), `ensure_config()` writing starter JSON
  (`:705-723`), `ensure_web_secret` / env-file writer (`:729-764`, mode 600),
  `config_dir()`/`data_dir()`/`default_config_path()` (`:673-687`).
- `Secrets` struct (`src/config/mod.rs:42-58`) — the three env-only tokens D2 helps set.

### Established Patterns

- Env-secret file generation already exists (web token, mode 600) — D2's secret
  setup extends this same pattern to github/telegram tokens.
- `ensure_config()` already writes a valid default JSON — D3's "create config now"
  reuses it rather than inventing a writer.

### Integration Points

- `src/main.rs:53-82` (arg parse) and `:61`,`:124-127` (doctor dispatch + exit code)
  — where an interactive/non-interactive branch and any new flags attach.
- `src/main.rs:84-106` `print_help()` — help text must describe the new capability.

## Canonical References

- `docs/specs/installation.md:254-270` — the authoritative "Diagnose the setup" spec
  section that currently guarantees read-only; D1 supersedes it, so scribing must
  rewrite this section after execution.
- `README.md:100-127` (Configuration table), `:159-177` (Troubleshooting, including
  the `HERDR_GO_WEB_SECRET` / `herdr-go.env` login-token guidance at `:168-177` that
  D8 changes) — the guide docs the user asked to update after the work. Note the
  table at `:121-127` documents only **7** fields; `telegram_chat_id` is missing and
  must be added, since D2 exposes all 8 through doctor.
- `src/main.rs` tests (`main_migration_seam_obeys_the_cli_mode_matrix`) — the
  test that enforces D12; it must stay green in every TTY/flag combination.
- `tests/rename_contract.sh:29` — tracks `src/doctor.rs` in its source-surface
  inventory; splitting doctor into a module directory requires updating this array
  or the contract test flags drift.

## Outstanding Questions

### Resolved After Planning

- [x] Interactive prompt mechanism — resolved by D13: stdlib primitives plus
      `rpassword` for masked secret entry only. `dialoguer`/`inquire` rejected as
      disproportionate for four prompt shapes.
- [x] Fix-then-recheck structure for the socket chain — resolved by D14: three
      phases with a single full re-run, early return removed.
- [x] Non-interactive detection and override flags — resolved by D15: `--check`
      added, `--yes` refused, interactive requires stdin **and** stderr to be a TTY.

- [x] Subcommand-dispatch vs flag-based entry — resolved by D17: flat parser, no
      subcommand dispatch, two modes only.

No open questions remain for this feature.

## Deferred Ideas

- Web settings UI as an alternative friendly surface — explicitly not chosen now
  (D1); a candidate for later if phone/browser configuration becomes a need.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads locked
decisions, code context, canonical references, and deferred-to-planning questions.
This is a **high-risk** change (security: token + `allowed_roots` writes; reverses a
documented public contract) — planning and validating must treat it as such.

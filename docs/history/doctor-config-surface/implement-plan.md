# Doctor as Config Surface — Implementation Brief

`herdr-go doctor` today only diagnoses and prints advice; after this feature it
becomes the one friendly place to both check *and* fix the setup. When a check
fails, doctor will offer to fix it right there instead of just printing a hint,
and the user will be able to walk through and edit every setting — the 8
`config.json` fields plus the three env-only secrets — from that same command.
Nothing about running `herdr-go doctor` non-interactively (piped, CI, scripts)
changes at all. This brief covers only Slice 1: the write/IO foundations. No
interactive wiring ships yet — that is Slice 2.

## What changes / what does not

**Changes (once all slices land):**
- A failing check offers its fix inline instead of only printing advice text (D3).
- The user can edit any of the 8 `config.json` fields and set up the three
  env-only secrets (web/github/telegram tokens) through `doctor` (D2).
- An invalid-but-present `config.json` is repaired field-by-field, keeping every
  field the user already had right (D7).
- Secrets set through doctor now also work on a direct `herdr-go` run, not only
  under systemd, because the binary loads `herdr-go.env` as an env fallback at
  startup (D8).
- After interactive fixes are applied, doctor re-runs its checks and the exit
  code reflects the final, post-fix state (D11).

**Does not change:**
- Non-interactive invocation (no TTY — piped, CI, scripts) stays byte-identical
  to today: same checks, same report, same exit code, zero prompts, zero writes
  (D5).
- Hand-editing `config.json` and `herdr-go.env` directly keeps working exactly
  as it does now; the interactive surface is additive, not a replacement (D4).
- `--config`, `--bind`, and auto-created defaults are untouched.
- Tokens still never live in `config.json` — the strict-decoding refusal is
  unchanged.
- `migrate_default_state_if` still never runs in doctor mode, in any
  TTY/flag combination (D12).

## Locked decisions

Compressed from `docs/history/doctor-config-surface/CONTEXT.md`. IDs are stable
and must not be reinterpreted.

| ID | Decision |
|----|----------|
| D1 | `doctor` itself becomes the interactive surface — diagnose, offer a guided fix per problem, and let the user edit any setting. One command, not a separate `config`/`setup` sibling, not a web UI. Supersedes doctor's documented read-only contract. |
| D2 | Config scope covers all 8 `config.json` fields plus setting up the env-only secrets, written to `herdr-go.env` (mode 600), never into `config.json`. |
| D3 | A failed check offers its fix inline, replacing print-only advice with an actionable prompt. |
| D4 | The interactive surface is additive: hand-editing, `--config`, `--bind`, and auto-created defaults all keep working unchanged. |
| D5 | Interactivity requires a TTY; non-TTY (piped/CI) degrades to today's exact behavior — no prompts, no writes. |
| D6 | Config writes re-run full validation before persisting and refuse to save an invalid result; secret values are never echoed back or printed. |
| D7 | An invalid-but-parseable `config.json` is repaired field-by-field, keeping every already-valid field; an unparseable file falls back to timestamped-backup-then-recreate, never a silent overwrite. |
| D8 | The binary loads `herdr-go.env` at startup as a fallback secrets source. Process environment always wins; the file is consulted only for keys absent from the environment, and must pass `validate_token_protection()` first. |
| D9 | A guided `allowed_roots` addition warns and requires typed confirmation when the path is over-broad (`/`, home directory, or a symlink) — never silently accepted, never hard-refused. |
| D10 | Secret writes reuse `prepare_token_directory` / `write_new_token` / `validate_token_protection` for mode 600 / owner-only ACL, and replace an existing key in place rather than appending. |
| D11 | After interactive fixes, doctor re-runs the checks and the exit code reflects the final state, not the pre-fix state. Non-interactive exit semantics are unchanged. |
| D12 | Interactive mode does not change doctor's exclusion from default-state migration — `migrate_default_state_if` must never run in doctor mode, in any TTY/flag combination. |
| D13 | Secret entry is masked using the `rpassword` crate (the one new dependency this feature adds). After entry, doctor confirms with length plus the last 3 characters only when the value is at least 12 characters long; shorter values show length alone; leading characters and the full value are never shown. Empty input means "skip, set it later". A secret is never accepted as a command-line argument. |
| D14 | Doctor runs in three phases: diagnose every check read-only, offer fixes, then re-run all checks once and report. The early return in the socket-resolution check is removed; checks that cannot be computed because a prerequisite failed report as *skipped* rather than aborting the run. |
| D15 | Add a `--check` flag ("diagnose only, change nothing") that takes precedence over everything, even on a TTY. Do not add `--yes` or any blanket auto-apply flag. Interactive mode requires both stdin and stderr to be a TTY; prompts are written to stderr, the report goes to stdout. |
| D16 | The settings editor shows the existing non-loopback bind warning at the moment `bind_addr` is edited, not only at startup. |
| D17 | The CLI keeps a flat parser — no subcommand dispatch — and exposes exactly two modes: bare `doctor` (diagnose, offer a fix per failing item, re-run) and `doctor --check` (diagnose only, never writes). There is no `--edit` shortcut; the settings editor is reached through a single end-of-run prompt ("edit a setting? [y/N]", default no). |

## Affected files

Projected from the three Slice 1 cells' `files` arrays. Nothing outside Slice 1
is touched yet.

**Cell 1 — `doctor-config-surface-1`** (module split + prompt primitives)
- `src/doctor/mod.rs` — new; carries `run()` orchestration moved out of today's
  single-file `src/doctor.rs`.
- `src/doctor/checks.rs` — new; the 10 existing checks moved verbatim, no logic
  changes.
- `src/doctor/prompt.rs` — new; `std::io::IsTerminal`-based dual-TTY detection
  (stdin **and** stderr, per D15) plus four prompt primitives (yes/no confirm
  with default, line input with default, typed confirmation,
  choose-from-numbered-list) that write their prompt text to stderr, never
  stdout; plus `rpassword`-backed masked secret entry (D13) returning the
  value and a non-revealing display form (length, plus the last 3 characters
  only when the value is at least 12 characters). All of this is dead code
  until Slice 2 wires it in.
- `src/lib.rs` — updated; module declaration points at the new `doctor/`
  directory instead of `doctor.rs`.
- `tests/rename_contract.sh` — updated; the source-surface inventory array
  (`tests/rename_contract.sh:29`) is edited to list the new module paths
  instead of `src/doctor.rs`.
- `Cargo.toml` — updated; adds `rpassword` as the single new dependency, for
  D13's masked entry.

**Cell 2 — `doctor-config-surface-2`** (config write/repair layer)
- `src/config/write.rs` — new; validated persist (D6), field-by-field repair +
  timestamped backup fallback (D7), and the `allowed_roots` breadth classifier
  (D9) — all pure functions, no prompting, unit-tested. Nothing calls this yet.
- `src/config/mod.rs` — read/reused, not restructured; `write.rs` calls
  `Config::load_str` for D6 validation instead of reimplementing it.

**Cell 3 — `doctor-config-surface-3`** (secret layer, depends on Cell 2)
- `src/config/secrets.rs` — new; a replace-not-append `herdr-go.env` writer
  (D10) built on the existing `prepare_token_directory` / `write_new_token` /
  `validate_token_protection` helpers, plus the D8 env-then-process-env
  fallback resolution with precedence and permission gating, unit-tested
  including the precedence and refusal cases.
- `src/config/mod.rs` — read/reused for the existing permission-handling
  helpers; no reimplementation of ACL or mode-600 logic.

## Implementation steps

Cell 1 and Cell 2 have no dependency on each other and can run in parallel.
Cell 3 depends on Cell 2 (it reuses the token-directory/permission helpers
that Cell 2's neighborhood in `src/config/mod.rs` documents and exercises).

### Cell 1 — `doctor-config-surface-1`: Split doctor into a module and add TTY-aware prompt primitives

1. Split `src/doctor.rs` into `src/doctor/mod.rs` (`run()` orchestration) and
   `src/doctor/checks.rs` (the 10 checks, moved without logic changes).
2. Add `src/doctor/prompt.rs`: dual-TTY detection via `std::io::IsTerminal`
   (stdlib, no new crate) that requires **both** stdin and stderr to be a
   terminal (D15), plus prompt primitives — yes/no confirm with default, line
   input with default, typed-confirmation, and choose-from-numbered-list —
   that write their prompt text to stderr, never stdout (D15). None of these
   are wired into any check yet.
3. Add masked secret entry to `src/doctor/prompt.rs` using the `rpassword`
   crate (D13): suppress terminal echo on input, then return the value plus a
   non-revealing display form — length, plus the last 3 characters only when
   the value is at least 12 characters long, length alone otherwise, never
   the leading characters. Empty input is representable as an explicit skip.
4. Add `rpassword` to `Cargo.toml` as the single new dependency this cell
   introduces.
5. Update `src/lib.rs`'s module declaration for the new `doctor/` layout.
6. Update `tests/rename_contract.sh`'s source-surface inventory (line 29) so
   the new module paths replace `src/doctor.rs`.
7. Unit-test the prompt primitives' parsing/default logic and the
   masked-entry display-form logic, all without needing a real terminal.

Verify (verbatim): `cargo test --quiet && cargo clippy --quiet -- -D warnings && bash tests/rename_contract.sh`

Key prohibitions: no interactive-prompt or TTY framework (`dialoguer`,
`inquire`, `requestty`, `crossterm`, `atty`, `is-terminal`) added to
`Cargo.toml` — `rpassword` for masked entry (D13) is the only permitted new
dependency; no hand-rolled terminal echo suppression via termios or the
Windows Console API; no prompt wired into any check in this cell; no check's
logic, order, or output text changes, and the socket-check early return stays
in place (its removal is D14 work for Slice 2); no `--check` flag in this
cell; doctor must not write, create, or mutate any file in this cell.

### Cell 2 — `doctor-config-surface-2`: Config write, field-by-field repair, and allowed_roots breadth guard

1. Add `src/config/write.rs` as a layer of pure, unit-testable functions —
   no prompting, no terminal use.
2. Validated persist (D6): re-run `Config::load_str` on the candidate JSON and
   refuse to write when invalid.
3. Field-by-field repair (D7): given the existing raw file plus a map of
   replacement values, preserve every currently-valid field and return which
   fields are invalid.
4. Backup-then-recreate fallback (D7): for JSON that cannot be parsed at all,
   write `config.json.bak-<timestamp>` **before** any overwrite.
5. Breadth classifier (D9): flag a candidate `allowed_roots` path as
   over-broad when it is the filesystem root, the user's home directory, or a
   symlink; do not flag an ordinary narrow directory.
6. Nothing in doctor calls any of this yet.

Verify (verbatim): `cargo test --quiet && cargo clippy --quiet -- -D warnings`

Key prohibitions: no prompting, stdin, or terminal use in this cell; no
weakening or bypassing existing config validation, including the
empty-`allowed_roots` fail-closed rule; no secret or token field may ever be
written into `config.json`; no overwrite of an existing `config.json` on any
path that hasn't first written a verified backup.

### Cell 3 — `doctor-config-surface-3`: Secret writer with replace-not-append and env-file startup fallback

Depends on: `doctor-config-surface-2`.

1. Work test-first: for each behavior below, write the test and record its
   failing output before implementing, so the cap carries red-failure
   evidence.
2. Secret writer (D10): write or update a key in `herdr-go.env`, reusing
   `prepare_token_directory` / `write_new_token` / `validate_token_protection`
   so unix mode 600 and the Windows owner-only ACL path are not reinvented;
   replace an existing key in place rather than appending.
3. Startup fallback (D8): `Secrets` loading gains a fallback that reads
   `herdr-go.env` for keys absent from the process environment. The process
   environment always wins. `validate_token_protection` must pass before the
   file is trusted; a file that fails protection is ignored with a non-fatal
   diagnostic, never silently used. A missing env file is not an error.

Verify (verbatim): `cargo test --quiet && cargo clippy --quiet -- -D warnings`

Key prohibitions: do not reimplement file permission or ACL logic — reuse the
existing helpers; the env file must never override a value already set in the
process environment; never trust an env file that fails permission
validation; never print, log, or include any secret value in output or error
text; never cause `migrate_default_state_if` to run in doctor mode (D12); no
prompting or stdin reads in this cell.

## Security posture

- **Secrets never enter `config.json`.** `RawConfig` uses
  `#[serde(deny_unknown_fields)]`, and a token field in the JSON document is a
  named parse error, exercised by the existing test at
  `src/config/mod.rs:903-914`. Slice 1 adds no new path into `config.json` for
  secret values.
- **The env file is trusted only after `validate_token_protection` passes**
  (D8). That check lives at `src/config/mod.rs:789-814` and already gates the
  web-secret persistence path; Cell 3 extends the same gate to the new
  startup fallback read.
- **Process environment always wins over the file** (D8) — a key set in the
  process environment is never overridden by the same key in `herdr-go.env`.
- **Over-broad `allowed_roots` paths require typed confirmation, not silent
  acceptance** (D9). Today's `Config::load_str` only checks that a root is
  absolute (`src/config/mod.rs:162-170`); nothing in config loading checks
  existence or breadth — that's a runtime concern doctor's check 7 already
  handles for existence (`src/doctor.rs:155-170`). Cell 2's breadth classifier
  adds the missing over-broad detection (root / home / symlink) that neither
  of those does today; the typed-confirmation *prompt* itself is Slice 2 work.
- **Doctor still never triggers `migrate_default_state_if`** (D12). The gate
  is `src/main.rs:27-35`, called at `src/main.rs:121`, and is proven by
  `main_migration_seam_obeys_the_cli_mode_matrix` in `src/main.rs`'s test
  module — the doctor/demo/config-path cases in that matrix must stay green
  through every cell in this feature.
- **No secret value is ever printed.** D6 and D10 both hold this; the
  existing read-only pattern doctor already uses for the web token check
  (`ensure_web_secret_readonly_impl`, `src/doctor.rs:262-281`) reports only
  presence/absence today, and Slice 1's new writers must not regress that —
  they persist values but never echo or log them.
- **Secrets are masked on entry and never displayed in full** (D13). Terminal
  echo is suppressed during input via `rpassword`; the confirmation shown
  afterward is length alone, or length plus the last 3 characters only when
  the value is at least 12 characters — never the leading characters, never
  the full value.
- **Secrets are never accepted as a command-line argument** (D13). Argv is
  world-readable via `/proc/<pid>/cmdline` and lands in shell history, so a
  secret passed as a flag or positional argument would leak outside the
  process.
- **`--yes` / blanket auto-apply is deliberately refused** (D15), to preserve
  the no-unattended-mutation property established by decision `4827aae8`: a
  flag that could auto-widen `allowed_roots` or write tokens without a human
  present would reopen exactly the risk that decision closed.

## Risks and what proves them

Condensed from `plan.md`'s risk map.

| Component | Risk | Proof required before execution closes |
|---|---|---|
| D8 startup load of `herdr-go.env` | HIGH | Process env always wins; file ignored when `validate_token_protection` fails; absent file is not an error; no secret value ever logged. |
| Config field-by-field repair (D7) | MEDIUM | Valid fields preserved; unparseable JSON takes the backup path; backup file exists before overwrite; invalid result never persisted. |
| `allowed_roots` guard (D9) | MEDIUM | `/`, `$HOME`, and a symlink each demand typed confirmation; a plain narrow path does not. |
| Non-TTY parity (D5) | MEDIUM | Piped invocation produces the same report and exit code as today, with zero writes. |
| Secret replace-not-append (D10) | MEDIUM | Rewriting an existing key yields one line, not two. |
| Doctor module split | LOW-MED | `bash tests/rename_contract.sh` green. |
| Re-check + exit code (D11) | LOW | Exit code reflects post-fix state. |
| New dependency `rpassword` (D13) | LOW | It is the only added dependency; masked-entry display logic is unit-tested without a terminal. |

## Open questions for validating

No open questions remain. All prior open questions are resolved:

- Secret entry echo — resolved by D13: masked entry via `rpassword`, with
  a length-plus-last-3-characters confirmation display shown only for
  values at least 12 characters long.
- Socket-chain restructuring for fix-then-recheck — resolved by D14:
  three phases (diagnose all, offer fixes, re-run all once), early return
  removed, uncomputable checks report as skipped.
- Whether an explicit `--non-interactive` / `--yes` flag is warranted —
  resolved by D15: `--check` is added; `--yes`/blanket auto-apply is
  explicitly refused.
- Subcommand-dispatch vs flag-based entry into interactive mode — resolved
  by D17: flat parser, no subcommand dispatch, exactly two modes (bare
  `doctor` and `doctor --check`), no `--edit` shortcut.

## Not in this slice

**Slice 2 — Interactive surface (not yet cellularized).** Wires the guided
fixes into each check (D3), adds the all-8-field settings editor reached
through a single end-of-run prompt ("edit a setting? [y/N]", default no) — no
`--edit` shortcut, no subcommand dispatch (D17) — applies the re-check +
exit-code semantics (D11), adds the `--check` flag (D15; the flag has no
meaning until interactive mode exists, so it is out of scope for Cell 1), and
removes the socket-check early return per D14: doctor moves to three phases
(diagnose all read-only, offer fixes, re-run all once), and checks that
cannot be computed because a prerequisite failed report as skipped instead of
aborting the run. Depends on all of Slice 1. Both the early-return removal
and the `--check` flag are explicitly excluded from Cell 1 (see Cell 1's key
prohibitions above) so Cell 1 stays behavior-neutral.

**Slice 3 — Docs & spec sync (not yet cellularized).** Rewrites the read-only
guarantee at `docs/specs/installation.md:254-270` that D1 supersedes (plus
D11's exit semantics), updates the README configuration table
(`README.md:121-127`) to add the missing 8th field `telegram_chat_id`, updates
the troubleshooting/token guidance (`README.md:168-177`) for D8, and updates
`print_help()` (`src/main.rs:84-106`).

## Anchor corrections

- CONTEXT.md's D9 rationale cites `src/config/mod.rs:895-901` for the claim
  that "validation only requires absolute+exists." That line range is the
  `non_absolute_root_rejected` test, which only exercises the absolute-path
  check in `Config::load_str` (`src/config/mod.rs:159-170`) — there is no
  *existence* check anywhere in config loading. The existence check the
  rationale is referring to lives in doctor's own check 7
  (`src/doctor.rs:155-170`, `!p.is_dir()`), a separate runtime check, not part
  of `Config::load_str`'s validation. This brief's Security posture section
  above cites both locations separately rather than repeating the merged
  claim. This does not change D9 itself, only the accuracy of one supporting
  citation.

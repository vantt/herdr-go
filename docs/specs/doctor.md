---
area: doctor
updated: 2026-07-20
sources: [doctor-config-surface]
decisions: [7e7d2990-7eff-4e7d-b2a0-aa957b11e56b, a1f78fa9-faff-4684-8b3c-3d0b9e0752c5]
coverage: partial
---

# Spec: Doctor — Diagnose and Configure

The single command an operator runs to check whether the application's setup
is healthy, fix what's wrong inline, and change any of its settings. This
area covers the diagnostic checks, the guided fixes attached to them, and the
settings editor. It does not cover the settings' own meaning or storage
location (see the `installation` area for `config.json`/the protected
secrets file) or the diagnosed subsystems' own behavior (herdr connectivity,
the web interface).

## Entry Points & Triggers

- Running the diagnose command with nothing else → diagnoses, and on a real
  interactive terminal offers a guided fix for anything fixable, then offers
  to edit any setting.
- Running the diagnose command with the diagnose-only option → always stays
  read-only: diagnoses and reports, never prompts, never writes, regardless
  of whether the terminal is interactive. This option takes precedence over
  everything else.
- Running the diagnose command from a script or pipe (not an interactive
  terminal), without the diagnose-only option → also stays fully read-only,
  identical to the diagnose-only report — no prompts, no writes.

## Data Dictionary

| # | Element | Meaning | Values | Required | Default |
|---|---------|---------|--------|----------|---------|
| 1 | Check result | The outcome of one diagnostic check | "healthy" (passes); "informational" (not a problem, worth noting); "problem" (fails, may or may not be fixable inline); "skipped" (could not run because an earlier check it depends on failed) | yes, per check | — |
| 2 | Check severity | Whether a "problem" result blocks a healthy overall verdict | "blocking" or "non-blocking" | yes, per problem check | — |
| 3 | Fixable check | Whether a check's problem has a guided fix attached | the setup file missing/invalid, or the allowed-workspace-roots setting missing a directory/being empty, or the login-secret file missing/unprotected are fixable; every other check is diagnostic-only, same as before this feature | yes | not fixable |
| 4 | Settings editor field | One of the 8 named settings in the setup file, or one of the 3 secrets in the protected secrets file | see the `installation` area's Data Dictionary for each field's own meaning | — | — |
| 5 | Root breadth | How broad an allowed-workspace-root path is, checked whenever one is added | "narrow" (an ordinary project directory — accepted without extra confirmation); "filesystem root", "the operator's home directory", or "a symlink" (each demands an explicit typed confirmation before being accepted) | yes, when adding a root | — |

## Behaviors & Operations

### Diagnose

- **Triggers:** every invocation, in every mode.
- **What changes:** nothing — read-only. Every check runs and reports a
  result; a check whose prerequisite check failed reports skipped rather
  than running against bad input or aborting the whole diagnose step. No
  check's presence or absence in the report changes because of this
  feature — a check that was already conditionally omitted (e.g. one that
  only makes sense when the setup file loaded) stays conditionally omitted.
- **Side effects:** none.
- **Afterwards:** every check that could run shows its result; skipped
  checks name which earlier failure caused the skip. A skipped check never
  counts as a pass by itself.

### Offer a guided fix

- **Triggers:** immediately after diagnose, only on a real interactive
  terminal and only when the diagnose-only option was not given.
- **Blocked when:** the diagnose-only option is present, or the terminal
  is not interactive — this step is skipped entirely in both cases, with
  zero prompts and zero writes.
- **What changes:** for each fixable problem found (Data Dictionary #3),
  the operator is offered its specific guided fix and can accept or
  decline each one independently. A missing setup file offers to create
  one with defaults. An invalid setup file offers to repair only the
  fields that are actually wrong, one at a time, preserving every field
  that was already valid; if the file cannot be read as a settings
  document at all, the operator is offered a backup-then-recreate instead.
  A missing or empty allowed-workspace-roots list, or one whose configured
  directory doesn't exist, offers to add a root or create the missing
  directory. Adding any root checks its breadth (Data Dictionary #5) — a
  narrow path is accepted immediately, but a filesystem root, the
  operator's home directory, or a symlink demands an explicit typed
  confirmation naming exactly which kind of broad path it is, never a
  plain yes/no. A missing or unprotected login-secret file offers to
  (re)create it through the same path startup itself uses.
- **Side effects:** every accepted fix is validated before being saved —
  an invalid result is never written, and the previous setting is left
  untouched on failure.
- **Afterwards:** the operator has applied zero or more fixes; declining a
  fix leaves that problem exactly as diagnosed.

### Recheck

- **Triggers:** immediately after the guided-fix step, whether or not any
  fix was applied — only reachable in the same interactive, non-diagnose-only
  mode as the guided-fix step above.
- **What changes:** nothing new — every check runs one more time, exactly
  once, so anything derived from a just-fixed setting is recomputed.
- **Side effects:** none beyond the second diagnose pass.
- **Afterwards:** the command's success/failure result reflects this final,
  post-fix state, not the state before any fixes were offered. In
  diagnose-only mode or a non-interactive invocation, there is no recheck —
  the result reflects the single diagnose pass, identical to before this
  feature.

### Edit a setting

- **Triggers:** a single yes/no prompt shown once, after the recheck step
  finishes — defaults to no. Only reachable in the same interactive,
  non-diagnose-only mode as the two steps above.
- **Blocked when:** the diagnose-only option is present, or the terminal is
  not interactive — the prompt is never shown in either case.
- **What changes:** on yes, the operator picks any one of the 8 setup-file
  settings or the 3 secrets from a menu, changes it, and returns to the
  menu to change another or stop. Every setup-file change is validated
  before being saved, exactly like a guided fix — an invalid value is
  rejected and the file is left as it was. Adding an allowed-workspace-root
  through this editor uses the exact same breadth check and typed
  confirmation as the guided fix above — there is only one such check in
  the system, not two independently-behaving ones. Changing the network
  address setting to anything other than local-only prints the existing
  non-loopback security warning immediately, before the change is saved —
  not only the next time the application starts. A secret is entered
  masked (never echoed to the screen) and never printed back.
- **Side effects:** none beyond the settings actually changed.
- **Afterwards:** the operator can keep editing settings until they choose
  to stop; the command then exits.

## Actors & Access

Single-operator system, same as `installation` — whoever runs the command
has full access to every check result and every setting.

## Business Rules

- **R1.** The command is one entry point with exactly two modes: run it
  bare, or with the diagnose-only option. There is no separate shortcut to
  jump straight to the settings editor — it is reached only through the
  end-of-run prompt (per D 7e7d2990-7eff-4e7d-b2a0-aa957b11e56b).
- **R2.** The diagnose-only option always wins over terminal detection: even
  on a real interactive terminal, giving it produces the same read-only
  report as a non-interactive invocation (per D 7e7d2990-7eff-4e7d-b2a0-aa957b11e56b).
- **R3.** A non-interactive invocation without the diagnose-only option
  behaves identically to giving it — read-only, no prompts, no writes — so
  scripted and piped use is never at risk of hanging on a prompt or
  mutating state unattended (per D 7e7d2990-7eff-4e7d-b2a0-aa957b11e56b).
- **R4.** Every accepted setting change is validated before being saved; an
  invalid result is never persisted and the previous value is left
  untouched (per D a1f78fa9-faff-4684-8b3c-3d0b9e0752c5).
- **R5.** Adding an allowed-workspace-root that is a filesystem root, the
  operator's home directory, or a symlink always demands an explicit typed
  confirmation naming which kind it is; an ordinary narrow path never does
  (per D a1f78fa9-faff-4684-8b3c-3d0b9e0752c5).
- **R6.** A secret is always entered masked and never echoed back in full —
  only its length, and its last few characters once it's long enough to
  make that safe (per D a1f78fa9-faff-4684-8b3c-3d0b9e0752c5).
- **R7.** The command's final success/failure result always reflects the
  last check pass that actually ran — the post-fix recheck when fixes were
  offered, or the single diagnose pass otherwise — never a stale pre-fix
  state (per D 7e7d2990-7eff-4e7d-b2a0-aa957b11e56b).

## Edge Cases Settled

- A check depends on a value only produced by an earlier, now-failed check
  → it reports skipped, naming the earlier failure, rather than running
  against bad input or aborting every check after it.
- The setup file cannot be parsed as a settings document at all (not just
  one field being wrong) → the guided fix offers backup-then-recreate
  instead of field-by-field repair, since no field is recoverable.
- The operator declines a guided fix → that problem is left exactly as
  diagnosed; nothing is silently applied.

## Open Gaps

- No new diagnostic check exists for the two secrets beyond the login
  token (the ones used for optional integrations) — they are settable only
  through the settings editor, never diagnosed as present/absent/protected
  the way the login token is. Whether that asymmetry is intentional long
  term or a candidate for a future check is undecided.
- The exact wording/format of the skipped-check indicator in the printed
  report, and of the settings-editor menu itself, is implementation detail
  not yet worth locking here — it may change without being a spec-breaking
  behavior change as long as the skipped/ok/fail/info distinction and the
  editor's field coverage stay intact.

## Pointers (implementation)

- `src/doctor/mod.rs` — the three-phase orchestration (diagnose, offer-fix,
  recheck) and the end-of-run edit prompt.
- `src/doctor/checks.rs` — the diagnostic checks, the `Check` result type
  (including the skipped state), and the guided-fix implementations for the
  setup-file and allowed-roots problems, including the shared root-breadth
  confirmation function.
- `src/doctor/prompt.rs` — the TTY-detection and prompt primitives (masked
  secret entry included) everything above is built on.
- `src/doctor/edit.rs` — the settings editor.
- `src/config/write.rs` — the pure validation/repair/breadth-classification
  functions the guided fixes and editor call, never reimplemented locally.
- `src/config/secrets.rs` — the secret-writer the editor calls for the 3
  secrets.
- `src/main.rs` — the diagnose-only flag and the command's exit-code wiring.

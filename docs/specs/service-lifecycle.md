---
area: service-lifecycle
updated: 2026-07-22
sources: [pbi-033-service-lifecycle-cli, self-update-merge-config]
decisions: [0689dfb8-b575-488b-92ce-41638bc42c74, 559ca1e7-d73d-4420-b5a9-5516c4e79540, ac6cb288-0cbd-4858-8d09-1f86e2b770b5, 1cf50ada-8545-4ddf-a360-a413c43a2f70]
coverage: partial
---

# Spec: Service Lifecycle Command

How an operator starts, stops, restarts, or checks the status of the
already-installed background service directly from the command line, without
having to remember three different operating-system commands. This area
covers only the `service` command's four verbs and how they pick the right
platform mechanism. It does not cover installing or registering the service
in the first place (see `installation`), nor the diagnostic command's own
guided-fix flow, which happens to reuse the same platform-detection idea for
its own restart-after-token-rotation offer (see `doctor`). The self-update
command also reuses this area's `start`/`stop` mechanism internally (never
its own separate implementation) as part of safely swapping to a new
version — see `docs/specs/self-update.md`.

## Entry Points & Triggers

- `herdr-go service start` → starts the background service via the operator's
  OS-registered service manager, if one is detected.
- `herdr-go service stop` → stops it the same way.
- `herdr-go service restart` → restarts it — a single native restart command
  where the platform has one, otherwise a stop followed immediately by a
  start.
- `herdr-go service status` → reports whether the OS considers the service
  running, without starting, stopping, or otherwise changing anything.
- `herdr-go service` with no verb, or any verb other than the four above →
  rejected the same way any other unrecognized input is (see Edge Cases
  Settled).

## Data Dictionary

| # | Element | Meaning | Values | Required | Default |
|---|---------|---------|--------|----------|---------|
| 1 | Verb | Which lifecycle action to perform | `start` · `stop` · `restart` · `status` | yes | — |
| 2 | Detected service manager (output, not input) | Which platform mechanism actually handled the request, tried in a fixed order | Linux systemd (per-user unit) · macOS launchd (per-user agent) · Windows Scheduled Task · none detected | — | — |

## Behaviors & Operations

### Start

- **Blocked when:** no supported service manager is detected on the machine
  (see Edge Cases Settled).
- **What changes:** the OS is asked to start the already-registered
  background service. Nothing is registered or created by this command
  itself — the service must already exist from installation.
- **Side effects:** whatever the native start command itself does; this
  command adds no logic on top (per D2, D7 — `0689dfb8`).
- **Afterwards:** the operator's terminal shows the native command's own exit
  status and output; the command's own exit code is that same status.

### Stop

- **Blocked when:** no supported service manager is detected.
- **What changes:** the OS is asked to stop the background service.
- **Side effects:** the native stop command's own behavior only — no extra
  idempotency handling is layered on top (per D7, `0689dfb8`).
- **Afterwards:** the operator's terminal shows the native command's own exit
  status and output.

### Restart

- **Blocked when:** no supported service manager is detected.
- **What changes:** the service is restarted — on Linux and macOS via a
  single native restart command; on Windows, which has no single-command
  restart for a Scheduled Task, by stopping and then starting it in
  sequence, still using only the same two native commands `stop`/`start`
  already use (per D7, `0689dfb8`).
- **Side effects:** the native command(s)' own behavior only.
- **Afterwards:** the operator's terminal shows the outcome; this is the same
  action the guided fix inside `herdr-go doctor` already offers after a
  token rotation (see `doctor`), now also reachable directly.

### Check status

- **Blocked when:** no supported service manager is detected.
- **What changes:** nothing — this is read-only.
- **Side effects:** none.
- **Afterwards:** the operator sees the service's current state as reported
  by the OS (e.g. running / stopped / not registered at all). A stopped or
  unregistered service is a normal, successful answer, not an error — the
  command's own exit code reflects whether the *query itself* succeeded, not
  whether the service happens to be running.

## Actors & Access

Single-operator system — whoever can run commands on the machine can run
`herdr-go service`, with whatever OS-level permission that already implies
for the underlying service manager (no separate authentication or
authorization layer inside the command itself).

## Business Rules

- **R1.** Platform detection is tried in a fixed order — Linux systemd
  (per-user), then macOS launchd (per-user), then Windows Scheduled Task —
  and the first one actually present on the machine handles the request; the
  platform is never assumed from how the binary was compiled (per D5,
  `0689dfb8`).
- **R2.** All four verbs ship together; there is no partial rollout of just
  `restart` or just `status` (per D3, `0689dfb8`).
- **R3.** All three platforms are supported from the first release of this
  command — none is deferred, even though no prior code in this project
  controlled a Windows service before this command existed (per D4,
  `0689dfb8`).
- **R4.** Every verb is a thin pass-through to the platform's own native
  command: it reports that command's own exit status and output as-is, and
  never adds retry, idempotency-guard, or state-checking logic beyond what
  the native command already does on its own (per D7, `0689dfb8`).

## Edge Cases Settled

- No supported service manager is detected on the machine (e.g. an
  unrecognized Linux init system, or the required platform binary is simply
  missing) → the command prints a clear error naming the failure and
  pointing at the manual per-platform commands, and exits with a non-zero
  status — the same shape used for any other unrecognized command-line input
  (per D6, `0689dfb8`).
- The service is not currently running when `stop` or `restart` is
  requested, or is already running when `start` is requested → whatever the
  native OS command itself does in that case; this command does not detect
  or special-case it (per D7 — see Business Rules R4).
- An unrecognized verb (or no verb) follows `service` → rejected with a
  usage message, the same way any other bad command-line input is rejected.

## Open Gaps

- The Windows branch has been proven only by local build/lint on a
  non-Windows development machine (no `pwsh`/Scheduled Task available here);
  its actual behavior against a real Windows Scheduled Task has not yet been
  exercised, the same open gap already recorded for this project's Windows
  install-lifecycle proof (see `installation`).
- Whether `status`'s exact reported wording matches `doctor`'s own existing
  service-state check, or reads slightly differently as its own standalone
  command, has not been decided — both currently describe the same
  underlying state independently.

## Pointers (implementation)

- `src/main.rs` — `service` token and its four-verb match arm in
  `parse_args()`; dispatches to `herdr_go::doctor::run_service_command`
  before any config/herdr wiring runs; `print_help()`'s `COMMANDS` section
  documents it.
- `src/doctor/checks.rs` — `run_service_command` and its three per-platform
  helpers (`run_systemd_service`, `run_launchd_service`,
  `run_windows_service`), reusing the same graceful-degrade probe idiom
  `active_service_restart` already established for `doctor`'s own restart
  offer.
- `src/doctor/mod.rs` — re-exports `run_service_command` across the module
  boundary so `main.rs` can call it.
- `src/update/rollout.rs` — the self-update command's `perform_update` calls
  `run_service_command("stop")`/`("start")` directly, the same function this
  area's own `service` command uses — see `docs/specs/self-update.md`.
- `README.md` (Service management section), `docs/installation.md` — operator
  documentation for all four verbs on all three platforms, alongside the
  raw per-platform commands kept as a fallback reference table.

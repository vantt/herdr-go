# PBI-033 — herdr-go service lifecycle CLI — Context

**Feature slug:** pbi-033-service-lifecycle-cli
**Date:** 2026-07-22
**Exploring session:** complete
**Scope:** Standard
**Domain types:** CALL, READ

## Feature Boundary

Add `herdr-go service {start|stop|restart|status}` — a thin, hand-parsed
subcommand in `src/main.rs` that auto-detects the OS's real service manager
(systemd user unit / launchd LaunchAgent / Windows Scheduled Task) and shells
out to it — plus update `README.md` and `docs/installation.md` so all three
verbs are documented for all three platforms. It ends at the CLI + docs; it
does not touch the supervision model itself (PRD.md §9's "systemd canh
gateway; gateway canh herdr" stays exactly as-is — this only gives the human
a single memorable front door to the OS commands that already exist).

## Locked Decisions

| ID | Decision | Rationale (only if it changes implementation) |
|----|----------|-----------------------------------------------|
| D1 | Ship the "add a subcommand" direction (backlog Option B), not docs-only (Option A). | User choice — asked directly (genuine product-scope/preference call, not derivable from evidence alone). |
| D2 | CLI shape is exactly `herdr-go service <start\|stop\|restart\|status>` — a new top-level `service` token with a nested verb, hand-parsed in `parse_args()`/`print_help()` (`src/main.rs`). No new dependency (no `clap`). | Matches the exact shape the user approved in the question preview; keeps `main.rs`'s existing "deliberately tiny, no arg-parsing dependency" property (`src/main.rs:17` comment). |
| D3 | All three verbs (`start`, `stop`, `restart`) plus a fourth, `status`, ship together in this feature — not restart-only, not a later add-on. | `status` was part of the approved shape; symmetric with the other three and cheap since the read side (service-state probing) already exists for `doctor`. |
| D4 | All three platforms (Linux systemd user unit, macOS launchd, Windows Scheduled Task) are in scope for v1 — none deferred, even though no Rust service-control code exists yet for Windows (only in `install.ps1`). | This is exactly the 3-platform gap the PBI names; deferring one platform would leave the original complaint half-fixed. |
| D5 | Platform/service detection is a **runtime probe** (try each service manager's presence/state when the command runs), the same idiom `src/doctor/checks.rs`'s `active_service_restart()` already uses for restart — not a compile-time `cfg(target_os)` branch. | Reuses an established, working pattern in this codebase instead of inventing a second one; `checks.rs:290-295`'s own comment explains why probing is sufficient (a missing binary just makes the probe fail through). |
| D6 | When no service manager is detected/registered (probe finds nothing), the command prints a clear, human-readable error to stderr naming the failure and pointing at the manual docs, and exits non-zero. | Matches `main.rs`'s existing `unknown argument` error convention (message + `print_help()` + `exit(2)` shape) instead of silently no-op'ing. |
| D7 | Each verb is a thin pass-through to the underlying native command — `systemctl --user <verb>`, `launchctl kickstart -k` / `bootstrap`/`bootout` as appropriate, `Start-ScheduledTask`/`Stop-ScheduledTask` — and surfaces that command's own exit status/output as-is. No extra idempotency/state-guard logic is layered on top (e.g. calling `start` on an already-running unit is whatever `systemctl --user start` itself does, not specially handled). | Keeps the subcommand thin per D1's own premise; the native tools already have sane idempotent semantics, so re-implementing that logic would be pure duplication (YAGNI). |
| D8 | Docs are updated in both `README.md` and `docs/installation.md`: `herdr-go service {start\|stop\|restart\|status}` is documented as the new, recommended unified command; the existing raw per-platform OS commands stay documented too (not deleted) as a fallback for users who prefer or need them directly. `print_help()`'s `COMMANDS` section in `src/main.rs` also gains a `service` line. | Keeps the docs gap actually closed (the PBI's own complaint) without breaking anyone currently following the raw-command instructions; matches how `doctor` is already documented alongside manual alternatives. |

### Agent's Discretion

Exact wording/section layout of the README.md and docs/installation.md
updates (e.g. whether the existing `### Restart` heading is renamed to
`### Service management` or a new heading is added alongside it) is left to
planning/execution, as long as D8's two requirements (new command documented
as primary, old commands kept as reference) both hold.

Whether `service restart` internally calls the same helper `doctor`'s guided
fix already uses (`src/doctor/checks.rs`'s `active_service_restart()`) or a
new sibling function is an implementation/module-boundary choice for
planning — not locked here.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| service | The herdr-go **gateway** process as registered with and managed by the OS-level service manager (systemd user unit / launchd LaunchAgent / Windows Scheduled Task). Not "herdr" — the separate subprocess the gateway itself supervises (PRD.md §9) — and not to be confused with it in code, docs, or CLI help text. |

## Existing Code Context

From the quick scout only. Downstream agents read these before planning.

### Reusable Assets

- `src/doctor/checks.rs:296-343` (`active_service_restart()`) — existing runtime-probe pattern for detecting and restarting the service: tries `systemctl --user is-active herdr-go.service` first, falls through to `launchctl print <target>`, returns `None` on Windows/unrecognized platforms today. The `service` subcommand's detection logic should follow this same shape, extended with a Windows branch (`schtasks`/`Get-ScheduledTask` equivalent) and generalized to all four verbs.
- `install.sh:195-213,237-239` and `install.ps1:60-62,146-171` — already contain full, working shell-out logic for start/stop/restart/status-equivalent on all three platforms (bash and PowerShell respectively). These are the canonical OS-command references to port into `src/main.rs`'s Rust implementation, not to be re-derived from scratch.
- `src/main.rs:17-115` — `Args` struct, `parse_args()`, `print_help()`: the exact insertion points for the new `service` token and its `COMMANDS` help line.

### Established Patterns

- Compile-time platform branching (`#[cfg(target_os = "macos")]` / `#[cfg(not(any(windows, target_os = "macos")))]`) exists in `src/config/mod.rs` for path resolution — a *different* pattern from the runtime-probe one used for service detection. Not the one to follow here (see D5).
- `main.rs`'s existing error convention for bad input: print a message + `print_help()` + `exit(2)` (`src/main.rs:75-79`) — the model for D6.

### Integration Points

- `src/main.rs` — new `service` branch in `parse_args()`/`main()`, new `COMMANDS` line in `print_help()`.
- `src/doctor/checks.rs` — likely shares or is called by the new subcommand's detection/action logic (left to planning, see Agent's Discretion).
- `README.md` (`### Restart` section, `README.md:61-71`) and `docs/installation.md` (restart block, `docs/installation.md:69-76`, plus the scattered start mentions at `docs/installation.md:17-19`).

## Canonical References

- `docs/PRD.md:156-167` (§9, "Vòng đời & supervision") — locks the "systemd canh gateway; gateway canh herdr" model this feature must not disturb.
- `docs/backlog.md` PBI-033 row — original problem statement and the (a)/(b) framing this exploring session resolved as D1.

## Outstanding Questions

### Deferred To Planning

- [ ] Exact Windows detection/verb implementation (which PowerShell/`schtasks` invocations map to probe/start/stop/restart/status) — `install.ps1` has the pieces but planning should confirm the concrete Rust `Command` invocations.
- [ ] Whether `service status` output is exactly what `doctor`'s existing probe already prints, or needs its own slightly different phrasing for a standalone command context.

## Deferred Ideas

- Exposing the same start/stop/restart/status control through the web UI/API (not just the CLI) — no user demand for this yet; the PBI's own complaint is entirely about the operator's local terminal experience, and PRD.md §9's supervision model doesn't call for a remote control surface.

## Handoff Note

CONTEXT.md is the source of truth. Decision IDs are stable. Planning reads
locked decisions, code context, canonical references, and deferred-to-planning
questions. Validating and reviewing use locked decisions for coverage and UAT.

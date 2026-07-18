---
area: installation
updated: 2026-07-18
sources: [embed-and-package-binary]
decisions: [b300856d, 3168932d]
coverage: partial
---

# Spec: Installation & Web Interface Serving

How an operator gets the application running on a machine, and how the
application serves its own web interface once running. This area covers the
install flow, where the application's persistent data lives, and how the
built-in web interface is chosen at startup — it does not cover the web
interface's own screens (see the `switcher` area and future screen specs) or
the day-to-day operation of an already-running instance (see `docs/usage.md`
until that becomes its own spec).

## Entry Points & Triggers

- Running the install script → sets the application up as a self-healing
  background service that starts automatically and survives a reboot.
- Running the application directly (no install) → runs once in the
  foreground; useful for a quick look or for iterating on a local build.
- Running the "dev-as-live" deploy helper → rebuilds the current checkout and
  makes that exact build the running background-service instance in one step
  (distinct from the install script, which is meant for a one-time production
  setup rather than a rebuild-on-every-change loop).
- Running the built-in diagnostic command → checks the current setup and
  reports what's missing or broken, with a plain-language fix for each.

## Data Dictionary

| # | Element | Meaning | Values | Required | Default |
|---|---------|---------|--------|----------|---------|
| 1 | Install prefix | Where the installed program and its data are placed on disk | any writable directory the operator chooses | no | the operator's personal local install area |
| 2 | Version to install | Which published version the install step downloads, when it downloads rather than builds | a specific published version, or "the latest one" | no | "the latest one" |
| 3 | Web interface source | Where the running application's web interface comes from | "built into the program itself" (always available, no setup needed) or "a separately built copy on disk at a configured location" (only used when actually present) | yes | built into the program itself |
| 4 | Application data location | Where the application stores its own persistent data (history, pending notifications) | a directory under the operator's personal data area | yes | derived automatically, not configured by the operator |

## Behaviors & Operations

### Install as a background service

- **Triggers:** operator runs the install script.
- **Blocked when:** never — the script always completes with either an
  installed program or a clear, named reason it could not (e.g. no working
  toolchain and no matching published version to download instead).
- **What changes:** the program is placed into the operator's personal
  install area; a starter configuration file and a login-token secrets file
  are created if none exist yet (an existing one is always left untouched); a
  background-service definition is installed so the program starts
  automatically and restarts itself if it ever exits, surviving a reboot.
- **Side effects:** first tries to obtain an already-published, ready-to-run
  copy of the program matching the operator's machine. Only when no such
  published copy exists for that machine does it build the program from
  source instead — which needs a working development toolchain present on
  the machine, and this requirement only applies on that fallback path (per D
  3168932d: on a machine with no development toolchain at all, a published
  copy existing for that machine is what makes install possible in the first
  place).
- **Afterwards:** the operator sets a login token in the generated secrets
  file, reviews the generated configuration, and starts the background
  service. Re-running the whole install step later (e.g. to upgrade) is safe
  and repeats cleanly — it never overwrites an existing configuration or
  secrets file, and never duplicates the background-service registration.

### Rebuild and run the current checkout as the live instance

- **Triggers:** operator runs the dev-as-live deploy helper.
- **Blocked when:** the machine has no working development toolchain — this
  path always builds from source, it never downloads a published copy (it
  exists specifically to run *this* uncommitted/local checkout, not a
  published release).
- **What changes:** the program and its web interface are rebuilt from the
  current checkout, and the background-service instance is restarted to run
  that fresh build in place.
- **Side effects:** none beyond the rebuild and restart.
- **Afterwards:** the freshly built code is what's live; re-running this
  helper after any further local change makes that new build live in turn.

### Serve the web interface

- **Runs when:** every time the application starts and a browser requests
  the web interface.
- **What changes:** nothing persists — this is a per-request choice, decided
  once at startup and applied consistently while the process keeps running.
- **Side effects:** none.
- **Afterwards:** the operator's browser gets the web interface either way,
  indistinguishably — from the operator's point of view there is no visible
  difference between the two sources (Data Dictionary #3). A page address
  that doesn't match a known screen still returns the interface's starting
  page rather than an error, so that in-app navigation (which is handled
  inside the web interface itself) keeps working after a full page reload or
  a bookmarked/shared link (per D b300856d).

### Store application data

- **Runs when:** the application starts (not in the throwaway/demo mode,
  which keeps everything in memory only) and needs to read or write its
  persistent history/notification records.
- **What changes:** a storage location under the operator's personal data
  area is used (creating it first if it doesn't exist yet); this location no
  longer has anything to do with where the web interface's on-disk copy (Data
  Dictionary #3) happens to live — the two are fully independent (per D
  b300856d). An operator who installed via the install script keeps seeing
  their existing history after upgrading, because this location is the exact
  same one the install script has always used.
- **Side effects:** none beyond the read/write itself.
- **Afterwards:** history and pending notifications persist across restarts.

### Diagnose the setup

- **Triggers:** operator runs the diagnostic command.
- **Blocked when:** never.
- **What changes:** nothing — read-only.
- **Side effects:** none.
- **Afterwards:** the operator sees a pass/fail line for each thing checked
  (the underlying agent-runner, the configuration file, connectivity, the
  login token, the allowed-locations setting, the web interface, the network
  address being listened on, service status), each failure paired with a
  one-line fix. The web-interface check can no longer fail (per D b300856d —
  a built-in copy is always available); it instead reports which source
  (Data Dictionary #3) is actually in effect right now, so the operator can
  tell whether an on-disk copy is unexpectedly overriding the built-in one.

## Actors & Access

Single-operator system — the person running these commands has full local
access to the machine; there is no separate access-control layer for
install/diagnostic operations (they run with whatever permissions the
operator already has on their own machine).

## Business Rules

- **R1.** Obtaining a published copy of the program never verifies its
  integrity beyond the secure-transport connection itself — no separate
  checksum or signature check (per D 3168932d). The transport being secure
  is the entire trust boundary.
- **R2.** The web interface is always available with zero extra setup — a
  fresh install with nothing else configured still serves a fully working
  interface (per D b300856d).
- **R3.** An on-disk copy of the web interface, when present at the
  configured location, always takes priority over the built-in one — this is
  how an operator overrides it for local iteration (per D b300856d).
- **R4.** The application's persistent data location is never affected by
  where the web interface is being served from (per D b300856d) — these were
  accidentally coupled before this decision and are now fully independent.
- **R5.** Re-running the install step is always safe: it never overwrites an
  existing configuration file or secrets file, and never duplicates the
  background-service registration (pre-existing rule, unchanged by this
  round of work).

## Edge Cases Settled

- No published copy exists for the operator's machine (unsupported
  hardware/OS combination, or nothing has been published yet at all) → falls
  back to building from source; this requires a working development
  toolchain only on this fallback path.
- A development toolchain is missing but a matching published copy exists →
  install still succeeds; the toolchain is never required in this case.
- Neither a matching published copy nor a working toolchain is available →
  install fails with a clear, named reason (not a silent partial install).
- An on-disk web-interface copy exists at the configured location → it is
  used instead of the built-in one, with no difference in behavior visible
  to the operator's browser.

## Open Gaps

- The "download a published copy successfully" path has been verified only
  by code inspection (correct address construction, secure-transport-only
  request, isolated extraction location, presence check on the extracted
  program before using it) — no version of this program has been published
  yet for this repository, so the path has never actually run against a real
  published copy in practice. The very next time a version is published,
  this should be exercised for real and this gap closed.
- No documented behavior yet for what happens if a published copy's internal
  layout ever changes shape (e.g. the program moves to a different location
  inside the downloaded package) — today a missing program after extraction
  is treated the same as "no published copy available" and falls back to
  building from source, but this exact scenario has not been deliberately
  tested.

## Pointers (implementation)

- `install.sh` — the install script; `detect_target`/`download_prebuilt`
  implement Data Dictionary #2's version resolution and the download
  attempt; falls back to `cargo build --release` + `npm run bundle` when no
  published copy matches.
- `dev-deploy.sh` — the dev-as-live deploy helper.
- `.github/workflows/release.yml` — publishes the copies `install.sh`
  downloads, one per supported target.
- `src/web/mod.rs` — `router()` implements the on-disk-override-or-built-in
  choice (`Assets` embedded via `rust-embed`, served via `axum-embed`'s
  `ServeEmbed`); `build.rs` guarantees the crate always has something to
  embed even before the web interface has ever been built locally.
- `src/config/mod.rs` — `data_dir()` (Data Dictionary #4's location) and
  `config_dir()` (the configuration file's own location, a different,
  unrelated directory).
- `src/main.rs` — wires the application-data storage to `data_dir()`.
- `src/doctor.rs` — the diagnostic command's web-interface check.
- `packaging/herdr-go.service` — the background-service definition
  `install.sh` and `dev-deploy.sh` install.
- `README.md`, `docs/installation.md`, `docs/deployment.md` — operator-facing
  install/setup documentation.

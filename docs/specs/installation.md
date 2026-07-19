---
area: installation
updated: 2026-07-19
sources: [embed-and-package-binary, rename-herdr-go, windows-support]
decisions: [b300856d, 3168932d, ee4af2f1-3877-4d92-91ed-a42c0351ec92, c202a89a-01f7-4f10-a310-2ebb4632535e, 5239acde-c517-4f8b-aea4-2d378972bcd5, 4827aae8-befd-43fe-b23b-fcdd19618482, 7e63cfd2-97fe-4a8c-bd8d-b4c15f84df1e, b590ff99-1360-4a91-93f4-27ae85c76ea4, f0b81ee1-6287-4250-b128-b63d967db115, edbcb0ff-b3ef-4456-8f61-239f1ddb8dd0, 86491143-a574-435f-b225-1c62dbd5c6b6]
coverage: partial
---

# Spec: Installation & Web Interface Serving

How an operator gets the application running on a machine, and how the
application serves its own web interface once running. This area covers the
install flow, where the application's persistent data lives, and how the
built-in web interface is chosen at startup — it does not cover the web
interface's own screens (see the `switcher` area and future screen specs) or
the day-to-day operation of an already-running instance (see the README until
that becomes its own spec).

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
| 4 | Application data location | Where the application stores its own persistent data (history, pending notifications) | a directory under the operator's local, non-roaming personal data area on Windows; the established per-user data area on Linux | yes | derived automatically, not configured by the operator |
| 5 | Product home | The canonical per-user identity used for configuration, persistent data, and background-service names | `herdr-go`; the retired `herdr-gateway` identity is accepted only as an upgrade source | yes | `herdr-go` |
| 6 | Running mode | Which mutually exclusive background instance owns the gateway port | installed production instance or current-checkout development instance | yes when run as a service | installed production instance |
| 7 | Configuration location | Where the operator's settings are stored | the roaming personal application-data area on Windows; the established per-user configuration area on Linux; an explicit operator-selected file on either platform | yes | derived automatically unless explicitly selected |
| 8 | Allowed workspace root | The default location agents may work within when the operator does not configure a narrower list | an absolute native user-profile location | yes | the operator's absolute user profile |
| 9 | Login token file | The local secret that authenticates web access | generated opaque secret, readable only by its owning user | yes outside throwaway/demo mode | created once and preserved across starts |

## Behaviors & Operations

### Install as a background service

- **Triggers:** operator runs the install script.
- **Availability:** the no-clone path is the normal supported Linux service
  install path. Public operator documentation assumes that supported release
  and install paths work; defects in those paths are tracked and fixed as bugs
  instead of being presented as the default README state.
- **Blocked when:** `systemctl` is absent or the systemd user service manager
  is unreachable. These prerequisites are checked before any state migration,
  download, file creation, installation, or service change. Otherwise the script
  always completes with either an
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
- **Login continuity:** only creation of a new secrets file prints its generated
  token. Repeat and migrated installs preserve it without printing it to logs;
  the operator retrieves or rotates it locally through the protected secrets
  file and restarts the service after rotation.
- **Workspace access:** service hardening keeps system locations read-only while
  leaving ordinary projects in the operator's home writable to supervised agents.

### Rebuild and run the current checkout as the live instance

- **Triggers:** operator runs the dev-as-live deploy helper.
- **Blocked when:** the host is not Linux, `cargo`, `npm`, or `systemctl` is
  unavailable, or the systemd user manager is unreachable. Every prerequisite
  is checked before state migration, dependency/build commands, filesystem
  writes, or service mutation. After that, a broken toolchain can still fail
  the build. This
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

### Select native per-user locations

- **Runs when:** the application starts without an explicit configuration file
  or explicit storage locations.
- **Blocked when:** the platform cannot provide an absolute native user profile
  or the required per-user application-data locations. Startup fails closed
  rather than selecting a relative or compatibility-shell path.
- **What changes:** Windows-style platform selection places configuration in
  the operator's roaming application-data area, persistent and runtime data in
  the operator's local application-data area, and the default allowed workspace
  under the absolute native user profile. The default and named local endpoints
  used to reach the agent runner are also rooted in that same native profile;
  they do not depend on a Unix-compatible home variable being present. Linux
  selection retains its existing per-user configuration, data, and endpoint
  locations.
- **Side effects:** no Unix-style location is discovered or migrated on Windows
  unless the operator explicitly selects it.
- **Afterwards:** every subsystem uses the same resolved locations; the operator
  sees no implicit cross-platform state movement.

### Validate Windows compatibility

- **Runs when:** the continuous compatibility checks exercise the Windows
  production branch.
- **Blocked when:** the exact upstream agent-runner executable cannot be fetched
  from its immutable versioned location, its recorded checksum does not match,
  or any runtime assertion fails.
- **What changes:** nothing in an operator's installation. The checks use only
  the checksum-verified executable they fetched; they never substitute another
  executable found elsewhere on the machine. Runtime restart recovery uses that
  same executable, so the check proves the supervised lifecycle rather than only
  the first process start.
- **Side effects:** no operator installation is created by this check; release
  publication remains a separate decision after proof is available.
- **Afterwards:** the Windows Server 2022 production branch is treated as
  runtime-proven for the foreground lifecycle checked here: native per-user
  locations, login, live agent listing, screen observation, input/reply,
  subscription observation, restart recovery, and denial of token reads by a
  distinct ordinary local user.

### Create or validate the login token

- **Runs when:** every non-demo startup resolves the login token.
- **Blocked when:** the protected parent location cannot be established, a new
  token cannot be created atomically with owner-only access, or an existing
  token does not have effective owner-only protection. The network listener is
  not started after any of these failures.
- **What changes:** first startup creates one protected token without a readable
  pre-protection interval; later startups preserve the same token after
  validating its protection.
- **Side effects:** no token value, token filename, account identity, or full
  sensitive path is emitted by startup or diagnostics.
- **Afterwards:** the owning operator can authenticate with the preserved token;
  other local users are denied by policy. A real second-user denial on Windows
  remains an open proof gap below.

### Upgrade from the retired product identity

- **Runs when:** normal startup without an explicit config, or an installer,
  finds configuration or persistent data under the retired `herdr-gateway`
  identity. A bind-only override remains normal startup and still migrates.
- **Does not run when:** the binary is invoked in doctor mode, demo mode
  (including combinations with other flags), or with an explicit config path.
- **Blocked when:** moving an old-only directory to the canonical sibling fails,
  or the retired path is unsafe to move. The application fails loudly instead of
  creating fresh replacement state.
- **What changes:** when only the retired directory exists, the directory itself
  is moved to the canonical `herdr-go` name before any canonical directory or
  starter file is created. Its contents are not inspected or merged.
- **Side effects:** existing file permissions, login secrets, and SQLite history
  move with the directory. When both old and new directories exist, the canonical
  directory wins, the retired one remains untouched, and startup reports a warning
  in its normal process/service logs.
- **Afterwards:** all new reads and writes use the canonical identity; the retired
  identity remains only as migration input.

### Select one background-service mode

- **Runs when:** production installation or development deployment changes the
  active background instance.
- **Blocked when:** a required migration/service command fails; the selected new
  instance is not enabled or started after such a failure. A service that simply
  does not exist is harmless and keeps the operation idempotent.
- **What changes:** production installation stops retired production/development
  and current development instances before enabling current production.
  Development deployment stops retired production/development and current
  production before starting current development. Retired service files are
  removed and the service manager reloads its definitions.
- **Afterwards:** exactly one selected gateway mode can own the configured address.

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
  Token diagnostics report only a location category and protection status;
  they never reveal the token, its filename, account identities, or a full
  sensitive path.

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
- **R6.** `herdr-go` is the sole canonical identity for current configuration,
  data, services, release archives, and documentation. The retired name is kept
  only where an existing installation must be discovered or migrated (per D
  ee4af2f1-3877-4d92-91ed-a42c0351ec92).
- **R7.** Release archive production and installer lookup/extraction use one
  atomic name contract; changing one side requires changing and smoke-testing
  the other (per D c202a89a-01f7-4f10-a310-2ebb4632535e).
- **R8.** The one-command installer is a systemd-based Linux path and must prove
  a reachable user manager before any mutation. Its service must not impose a
  blanket read-only policy on the operator's home (per D
  5239acde-c517-4f8b-aea4-2d378972bcd5).
- **R9.** Default legacy state moves only for normal default-config startup.
  Development deployment proves Linux, toolchain, and user-manager prerequisites
  before mutation. Public operator documentation presents supported install
  paths as working; regressions or missing assets are tracked as bugs, not as
  README caveats.
- **R10.** A Windows support claim is limited to behavior proven on real Windows;
  host-side selection and security tests do not establish full Windows support,
  and Windows 11 remains a separate support claim until proven directly (per D
  7e63cfd2-97fe-4a8c-bd8d-b4c15f84df1e and D
  86491143-a574-435f-b225-1c62dbd5c6b6).
- **R11.** Windows defaults use native roaming configuration, local persistent
  data, and an absolute native profile root without automatic Unix-style state
  migration (per D f0b81ee1-6287-4250-b128-b63d967db115).
- **R12.** The login token is protected for its owner before its bytes become
  visible, and every startup validates that protection before serving (per
  windows-support D3).
- **R13.** The first Windows lifecycle, once proven, is a target-specific archive
  run in the foreground; service installation, auto-start, installer automation,
  and development deployment are outside this slice (per D
  edbcb0ff-b3ef-4456-8f61-239f1ddb8dd0).
- **R14.** Windows compatibility checks bind both ordinary runtime calls and
  supervisor recovery to the same checksum-verified agent-runner executable;
  they do not fall back to another executable discovered on the machine (per D
  86491143-a574-435f-b225-1c62dbd5c6b6).

## Edge Cases Settled

- No matching published copy exists for the operator's machine → the no-clone
  installer fails with a named download error and points to source-build
  instructions; it does not silently build from a checkout that may not exist.
- An on-disk web-interface copy exists at the configured location → it is
  used instead of the built-in one, with no difference in behavior visible
  to the operator's browser.
- Both retired and canonical product-home directories exist → canonical wins;
  retired state is left untouched and a warning is emitted, never merged.
- A retired background service is missing during upgrade → migration continues
  idempotently; a real stop/disable/remove failure aborts before the new mode starts.
- A token file already exists without effective owner-only protection → startup
  fails before opening the listener; it does not overwrite, expose, or silently
  accept the token.
- A native per-user root is unavailable → startup fails closed; it does not
  fall back to a relative location.
- A fetched Windows compatibility executable has the wrong checksum → the
  compatibility run stops before executing it.

## Open Gaps

- The installer has not yet downloaded and installed a real archive published
  under the new `herdr-go-<platform>` name. The next renamed release must
  exercise download, extraction, execution, and service setup before this
  branch is treated as proven end to end.
- No documented behavior yet for what happens if a published copy's internal
  layout ever changes shape (e.g. the program moves to a different location
  inside the downloaded package) — today a missing program after extraction
  is treated the same as "no published copy available" and falls back to
  building from source, but this exact scenario has not been deliberately
  tested.
- The Windows Server 2022 foreground lifecycle is proven by continuous
  compatibility checks, but the one-command installer has not yet downloaded
  and installed a Windows archive. Windows service installation, auto-start,
  development deployment, and Windows 11 remain unproven until exercised
  directly.

## Pointers (implementation)

- `install.sh` — the self-contained Linux install script; it resolves and
  downloads the requested published copy, installs canonical configuration
  and service definitions, and migrates retired directories/services.
- `dev-deploy.sh` — the dev-as-live deploy helper.
- `.github/workflows/release.yml` — publishes the copies `install.sh`
  downloads for the currently supported Linux and macOS targets; Windows is
  excluded pending its blocking runtime proof.
- `src/web/mod.rs` — `router()` implements the on-disk-override-or-built-in
  choice (`Assets` embedded via `rust-embed`, served via `axum-embed`'s
  `ServeEmbed`); `build.rs` guarantees the crate always has something to
  embed even before the web interface has ever been built locally.
- `src/config/mod.rs` — `data_dir()` (Data Dictionary #4's location) and
  `config_dir()` (the configuration file's own location, a different,
  unrelated directory), native root selection, and protected token lifecycle.
- `src/main.rs` — wires the application-data storage to `data_dir()`.
- `src/doctor.rs` — diagnostic checks and redacted location/protection output.
- `src/herdr/socket.rs` — shared local-endpoint resolution and platform-specific
  connection setup.
- `.github/workflows/ci.yml`, `scripts/windows-runtime-smoke.ps1` — fetch and
  checksum the pinned upstream Windows executable, bind every smoke invocation
  to that exact file, and carry the real-Windows compile, interoperability,
  restart, native-root, and second-user token-isolation proof.
- `packaging/herdr-go.service` — the background-service definition
  `install.sh` and `dev-deploy.sh` install.
- `README.md` — operator-facing install, usage, configuration, deployment,
  source, and troubleshooting documentation.

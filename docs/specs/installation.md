---
area: installation
updated: 2026-07-22
sources: [embed-and-package-binary, rename-herdr-go, windows-support, binary-rename-herdr-go, release-packaging-p1-fix, windows-release-matrix, windows-username-length-fix, cross-platform-install, doctor-config-surface, windows-installer-runtime-smoke, macos-installer-runtime-smoke, default-agent-presets, self-update-merge-config, dedupe-default-config-templates]
decisions: [b300856d, 3168932d, ee4af2f1-3877-4d92-91ed-a42c0351ec92, c202a89a-01f7-4f10-a310-2ebb4632535e, 5239acde-c517-4f8b-aea4-2d378972bcd5, 4827aae8-befd-43fe-b23b-fcdd19618482, 7e63cfd2-97fe-4a8c-bd8d-b4c15f84df1e, b590ff99-1360-4a91-93f4-27ae85c76ea4, f0b81ee1-6287-4250-b128-b63d967db115, edbcb0ff-b3ef-4456-8f61-239f1ddb8dd0, 86491143-a574-435f-b225-1c62dbd5c6b6, 178345a6-768c-4645-909f-1ab0a61f523f, 8212ddcb-1fa7-4311-a4df-d60cc4a2ad1e, de8df760-b12d-4cb6-83ff-d13c7f0ddbe5, b8c3d4bc-6572-4036-bf63-b0bd679c117a, 15189a97-da67-42fe-9651-ead59cc907d7, 7e7d2990-7eff-4e7d-b2a0-aa957b11e56b, 60948b5f-4c8c-4b56-8811-57df7c48f554, d28eb685-c3b8-422d-a167-267f2b76d535, 0bfdcd6a-b339-4dc0-936a-05e7c94cb3e1, 168212ca-6a27-4a07-88c3-9a59a3ea1de2, ce0c5d55-5f06-4960-9fdd-014cfaa75a0b, 43c64cfa-f23c-4eda-8194-ae911d40acc7, 52648efc-03b7-411d-b4f5-4af3843845e0, 898c9cd5-33fe-4a7f-b0e8-fb7ab7c69b25, be8f0d8a-f762-4f0e-8a62-a61b76565c55, 10f5961f-593f-4846-b9bf-54397b02e7ac, a9110d35-ef73-4943-b084-5827c834751a, 1462ba5a-3166-4fb0-80e7-da39626aace3, ba58d5ab-fd6f-4ecd-82f3-9338e87d9594, 18ff92f2-fcde-4cd3-b989-3158aed50be1, 4e02f81c-91e4-437e-b67f-476a1e6efdf4, 1e494c17-d05f-47ec-b024-6f08b7c05c81, 1dd32f92-5dac-44b7-8e4a-3900c1bf33b1]
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
- Running the built-in self-update command → takes an already-installed
  operator straight to the latest published version, carrying their existing
  settings forward; see `docs/specs/self-update.md` for this command's own
  full behavior.

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
| 7a | Agent preset | One entry the operator declares so the phone can offer it as a way to start an agent: a display label plus the exact command to run. The gateway never checks that the command names a program that exists — it does not know what is installed on the machine, and the terminal host reports a failure to start. | a label and a non-empty command | no | a freshly created setup file ships 3 presets already declared — see R20a. An operator-edited setup file (any content, including a deliberately empty list) keeps exactly what the operator left it with. |
| 8 | Allowed workspace root | The default location agents may work within when the operator does not configure a narrower list | an absolute native user-profile location | yes | the operator's absolute user profile |
| 9 | Executable identity | The command name operators, services, release packages, and diagnostics use for this application | `herdr-go`; no retired prior name is an active alias | yes | `herdr-go` |
| 10 | Login token file | The local secret that authenticates web access | generated opaque secret stored in `herdr-go.env`, readable only by its owning user | yes outside throwaway/demo mode | created once and preserved across starts |
| 11 | Login token environment value | The process environment value that can provide the web login token without reading the token file | `HERDR_GO_WEB_SECRET` | no outside demo mode | unset; the protected token file is used |
| 12 | Application state file | The durable local database file that stores history and pending notifications | `herdr-go-state.sqlite` under the application data location | yes outside throwaway/demo mode | created automatically |
| 13 | GitHub integration secret | The credential the application uses to act on GitHub on the operator's behalf | any value; presence enables the GitHub integration, absence disables it | no | unset; feature disabled |
| 14 | Telegram integration secret | The bot credential the application uses to send Telegram notifications | any value; presence enables Telegram notifications, absence disables them | no | unset; feature disabled |
| 15 | Secrets source precedence | Where a secret's effective value comes from when both a process environment value and the protected secrets file could supply it | "process environment" always wins over "protected secrets file" for the same secret | yes (fixed rule) | process environment |

## Behaviors & Operations

### Install as a background service

- **Triggers:** operator runs the install script (Linux/macOS: one shell
  script; Windows: a separate PowerShell script fetched via its own
  Windows-native one-liner, distributed outside the release archive since the
  Windows archive is deliberately foreground-only).
- **Availability:** the no-clone path is the normal supported service install
  path on Linux, macOS, and Windows. Public operator documentation assumes
  that supported release and install paths work; defects in those paths are
  tracked and fixed as bugs instead of being presented as the default README
  state.
- **Blocked when:** on Linux, `systemctl` is absent or the systemd user
  service manager is unreachable; on Windows, nothing platform-specific
  blocks install (no elevation, no special service manager reachability
  check needed — the Windows path never requires administrator rights).
  These prerequisites are checked before any state migration, download, file
  creation, installation, or service change. Otherwise the script always
  completes with either an installed program or a clear, named reason it
  could not (e.g. no working toolchain and no matching published version to
  download instead, or an unsupported architecture).
- **What changes:** the program is placed into the operator's personal
  install area under the active executable identity; a starter configuration
  file and a login-token secrets file are created if none exist yet (an existing
  one is always left untouched by this install step — the self-update command
  is the one path that later carries new settings into an existing file, see
  `docs/specs/self-update.md`). The starter configuration file's content is
  always obtained from the just-installed program itself — the install script
  never hand-writes the file's content a second time — so a fresh install
  always gets exactly the same starter configuration the program would create
  on its own first run, including the default agent presets (R20a), on every
  platform (R20b). A background-service definition is installed so
  the program starts
  automatically and restarts itself if it ever exits, surviving a reboot. On
  macOS this is a per-user launchd agent instead of a systemd unit, loaded (and
  started immediately) rather than only enabled; it never carries the login
  token or any secret value itself — the running program resolves its own
  token by reading the secrets file directly, exactly as it does on Linux. On
  Windows this is a per-user, non-elevated Scheduled Task that starts the
  program at logon and makes a best-effort restart attempt if it crashes (not
  the sub-minute restart guarantee Linux/macOS provide — Windows Task
  Scheduler's own restart interval has a one-minute floor); the installer
  never creates or writes the login-token file itself on Windows either — the
  running program creates and protects that file on its own first start, the
  same as every other platform, and the installer only reads it once
  afterward to display the token.
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

### Package a published release

- **Runs when:** a maintainer publishes a versioned release.
- **Blocked when:** the release package lists a documentation item that is not
  present in the current operator documentation set. For the Windows target
  specifically, publication is additionally blocked whenever that release's own
  proof of a real, working agent-runner connection on Windows does not pass —
  every tagged release re-proves Windows at release time rather than trusting
  an earlier, separate proof run.
- **What changes:** each Linux and macOS target gets a release archive
  containing the executable, service definition, installer, and the current
  top-level operator installation guide. The Windows target gets its own
  archive containing only the executable and operator documentation — no
  service definition or installer, since the first Windows lifecycle is a
  foreground-only run (R13). After every platform's archive is produced, a
  single combined integrity proof covering every archive published in that
  release is generated and published alongside them — this is what the
  self-update command verifies against before ever installing anything (see
  `docs/specs/self-update.md`).
- **Side effects:** none beyond producing release archives for operators to
  download.
- **Afterwards:** every documentation item advertised inside a release archive
  exists in the same source version that produced the archive. A Windows
  archive exists for a given release only when that release's own proof
  passed; a failed proof yields no Windows archive for that release while the
  Linux/macOS archives still publish normally. Once the Windows or macOS
  archive is published, a separate install-lifecycle re-proof runs against it
  (R19) — unlike the pre-publish agent-runner proof above (Windows only),
  this one runs after publishing, on both platforms, and its failure does not
  withdraw the archive.

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
  they do not depend on a Unix-compatible home variable being present. macOS
  selection places both configuration and persistent data in the operator's
  single native per-user application-support location (no separate roaming/local
  split, unlike Windows). Linux selection retains its existing per-user
  configuration, data, and endpoint locations.
- **Side effects:** no Unix-style location is discovered or migrated on Windows
  unless the operator explicitly selects it; macOS never falls back to the
  Linux-style location either.
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
  pre-protection interval in the active token file; later startups preserve the
  same token after validating its protection. A provided login-token environment
  value is accepted only under the active environment name.
- **Side effects:** no token value, token filename, account identity, or full
  sensitive path is emitted by startup or diagnostics.
- **Afterwards:** the owning operator can authenticate with the preserved token;
  other local users are denied by policy. A real second-user denial on Windows
  remains an open proof gap below.

### Resolve an integration secret at startup

- **Runs when:** every startup, for each of the login token, the GitHub
  integration secret, and the Telegram integration secret.
- **What changes:** nothing persists — this is a read-only resolution. The
  process environment is checked first; if the value is present there, that
  value is used and the protected secrets file is not consulted for that
  secret. If the process environment does not supply the value, the protected
  secrets file is checked instead — but only when that file still has
  effective owner-only protection.
- **Blocked when:** the protected secrets file exists but no longer has
  effective owner-only protection — that file is not trusted as a source for
  any secret. A missing secrets file is not a failure; it simply means no
  fallback value is available.
- **Side effects:** an untrusted secrets file produces a non-fatal warning
  that never reveals which value it would have supplied.
- **Afterwards:** each secret's effective value is either the process
  environment's value, the protected secrets file's value, or absent —
  absence disables the dependent feature rather than blocking startup. No
  secret value is ever shown, logged, or included in diagnostic output,
  regardless of which source supplied it.

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
  identity remains only as migration input. The retired executable name, token
  filename, state filename, and environment-variable prefix are not active
  compatibility surfaces.

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

The diagnostic command exists and is one of this area's entry points (above).
Its own behavior — the checks it runs, the guided fixes it offers, and its
settings editor — is a full area in its own right: see `docs/specs/doctor.md`.
This area still owns what each setting *means* and where it's stored; the
`doctor` area owns how the operator inspects and changes them.

## Actors & Access

Single-operator system — the person running these commands has full local
access to the machine; there is no separate access-control layer for
install/diagnostic operations (they run with whatever permissions the
operator already has on their own machine).

## Business Rules

- **R1.** Obtaining a published copy of the program via the install script
  never verifies its integrity beyond the secure-transport connection itself
  — no separate checksum or signature check (per D 3168932d). The transport
  being secure is the entire trust boundary for a fresh install. The
  self-update command is a separate path with its own, stricter rule — see
  `docs/specs/self-update.md`.
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
  README caveats (per D 8212ddcb-1fa7-4311-a4df-d60cc4a2ad1e).
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
- **R15.** `herdr-go` is the only active executable identity before the first
  public release: services, release packages, diagnostics, token/state files,
  and environment-variable names use it directly, with no retired-name alias or
  fallback (per D 178345a6-768c-4645-909f-1ab0a61f523f).
- **R16.** Release archives include only current operator documentation; release
  validation blocks publication when the package list points at removed
  documentation (per D de8df760-b12d-4cb6-83ff-d13c7f0ddbe5).
- **R17.** The Windows release archive is produced by re-proving the exact
  binary being released against a real agent-runner connection at release time,
  not by trusting an earlier, separate proof run; a failed proof blocks only
  the Windows archive, never the Linux/macOS archives (per D
  b8c3d4bc-6572-4036-bf63-b0bd679c117a). The Windows archive never contains the
  Linux service definition or installer script — its lifecycle is
  foreground-only (per D 15189a97-da67-42fe-9651-ead59cc907d7, R13).
- **R18.** The protected secrets file is a startup fallback source for every
  integration secret (login token, GitHub, Telegram), not only the login
  token; the process environment always wins over the file for the same
  secret, and a file that has lost effective owner-only protection is never
  trusted regardless of what it contains (per D
  7e7d2990-7eff-4e7d-b2a0-aa957b11e56b).
- **R19.** After a Windows or macOS release publishes, the install lifecycle
  re-proves itself against that exact published copy: install, confirm the
  background service is live, simulate a crash and confirm the service
  recovers on its own, then uninstall and confirm the service registration
  and program are gone while configuration, data, and the login token are
  untouched. A failure here never unpublishes the release — it surfaces as a
  failed pipeline run, the same way every other pipeline check does (per D
  60948b5f-4c8c-4b56-8811-57df7c48f554 for Windows, D
  577ec951-271d-48f4-a67f-f8d6094284bf and D
  668ecd72-55fe-4c50-a3b5-f5ff4ce41962 for macOS). Proving the service is live
  uses only its own health signal; it never requires the agent-runner
  connection itself to be present or working (per D
  168212ca-6a27-4a07-88c3-9a59a3ea1de2 for Windows, D
  d1eb32cd-6523-479c-8012-b244ba3120b7 for macOS). The login token this proof
  observes is never written to the pipeline's own logs (per D
  ce0c5d55-5f06-4960-9fdd-014cfaa75a0b for Windows, D
  6291bf7c-544f-4acc-bd17-0be2a81a2d73 for macOS). On macOS, simulating the
  crash uses a termination that produces an unsuccessful process exit, since
  the platform's own auto-restart only fires on that condition (per D
  d6beb3e6-15c1-4078-b836-63a1058fd8d8).

- **R20.** The setup file may declare agent presets: a list of entries, each a
  display label and the command to run. The list is optional. An entry with
  no label, no command, an empty first word, or a label another entry already
  uses makes the setup file invalid and the application refuses to start,
  naming the offending entry by its position and label — the same refusal
  posture the file takes for an unknown setting or an empty workspace-root
  list. A preset that loaded but quietly did nothing would be a failure the
  operator only meets later, on a phone, away from the machine.
- **R20a.** The first time the application creates a setup file (no file
  existed yet at startup, or the diagnostic command rebuilds one after
  finding the existing one unreadable), it declares 3 agent presets already:
  one running the Claude agent CLI, one running the Codex agent CLI, and one
  running an agent CLI named "Agy" — each with its command's confirmation/
  sandbox prompts turned off, so the phone can start any of the three without
  the operator ever opening the setup file. Any setup file the application
  did not just create — including one an operator has edited down to no
  presets at all — is never rewritten to add these back; the 3 presets exist
  only so a brand-new setup works immediately, never as an enforced minimum
  (per D 898c9cd5-33fe-4a7f-b0e8-fb7ab7c69b25).
- **R20b.** R20a's guarantee holds for every path that can create a setup
  file, including the install script on every platform — not only the
  application's own first-run creation. The install script never derives its
  own copy of the starter configuration's content; it always obtains that
  content from the just-installed program itself, so an install-script-created
  setup file and a program-created one are always identical, agent presets
  included. Before this rule, the install script wrote its own, separately
  maintained copy of the starter configuration that had drifted out of sync
  with the program's own copy and did not declare agent presets — so a
  fresh install done through the install script (as opposed to one where the
  file did not exist and the program created it on first run) silently never
  got them, until an operator ran the self-update command (per D
  a9110d35-ef73-4943-b084-5827c834751a, D 1462ba5a-3166-4fb0-80e7-da39626aace3,
  D 18ff92f2-fcde-4cd3-b989-3158aed50be1, D
  1e494c17-d05f-47ec-b024-6f08b7c05c81).

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
  compatibility checks and, as of the current release packaging, by a
  per-release re-proof before the Windows archive is produced. A one-command
  installer now exists for Windows too (Scheduled Task auto-start, no
  elevation), and an automated end-to-end install-lifecycle proof (R19) now
  runs in the release pipeline after each Windows archive publishes —
  download the real archive, run the installer, confirm the Scheduled Task
  actually starts and supervises the process (including a real crash-restart
  wait, not simulated), then uninstall and confirm cleanup. This proof has
  not yet executed against a real published release (it ships with this
  change; the next tagged release is its first live run) — until then it is
  verified only by local structural checks (script exists, syntax-valid
  workflow, no regression to the rest of the codebase), not by an actual
  Windows Scheduled Task. The proof also does not exercise the Scheduled
  Task's own logon trigger firing on a real interactive logon — only its
  manually-started path — because CI runners have no real interactive logon
  session to exercise. Windows development deployment and Windows 11 remain
  unproven until exercised directly.
- The macOS install path's platform-specific pieces (native path resolution,
  the launchd service definition) are proven by real macOS CI compiling and
  unit-testing them and by a real release-packaging proof that the archive
  ships the correct service file. An automated end-to-end install-lifecycle
  proof (R19) now runs in the release pipeline after each macOS archive
  publishes — download the real archive, run the installer, confirm the
  launchd agent actually starts and supervises the process (including a real
  crash-restart wait sized to the platform's own short restart floor, not
  simulated), then uninstall and confirm cleanup. This proof has not yet
  executed against a real published release (it ships with this change; the
  next tagged release is its first live run) — until then it is verified only
  by local structural checks (script exists, syntax-valid, no regression to
  the rest of the codebase), not by an actual launchd agent. Unlike Windows'
  equivalent gap (above), this proof is expected to also validate its own
  trigger mechanism for real, since macOS CI runners run with a genuine
  logged-in session — but that expectation itself is unconfirmed until the
  first real run. Separately, no automated proof yet exists that the herdr-go
  binary launched this way can actually reach and drive a real agent-runner on
  macOS (the launchd counterpart to Windows' dedicated agent-runner
  round-trip smoke) — filed as a distinct, not-yet-started gap.
- Intel Macs (`x86_64-apple-darwin`) have no published binary and no installer
  support; the installer fails with a named, actionable error pointing to a
  source build.
- An internal capability to set or update a secret's value in the protected
  secrets file, without ever producing a duplicate entry for the same secret,
  now exists and is unit-proven — but no operator-facing command reaches it
  yet. It becomes reachable once the diagnostic command's interactive
  configuration surface is wired in (a later slice of the same feature); until
  then, editing the secrets file by hand remains the only way to set a value.

## Pointers (implementation)

- `install.sh` — the self-contained Linux and macOS install script; it
  resolves and downloads the requested published copy, installs canonical
  configuration and service definitions, and migrates retired
  directories/services. The starter configuration file is written by
  capturing `herdr-go --internal-print-default-config`'s output (a hidden,
  undocumented flag, see `src/config/mod.rs` below) rather than a
  hand-written literal (R20b).
- `install.ps1` — the self-contained Windows install script (PowerShell,
  `irm | iex` distribution); registers a per-user Scheduled Task instead of
  a systemd unit or LaunchAgent, never creates the token file itself. Writes
  the starter configuration file the same way as `install.sh` (R20b), via
  `[System.IO.File]::WriteAllText` with an explicit no-BOM encoding — not
  `Out-File`, which would prepend a byte-order mark under Windows PowerShell
  that the configuration parser does not tolerate.
- `config.example.json` — a static, hand-maintained documentation sample
  showing every setup-file field (including agent presets); not read by any
  code path, kept in sync by hand when the field set changes.
- `dev-deploy.sh` — the dev-as-live deploy helper.
- `.github/workflows/release.yml` — publishes the copies `install.sh`
  downloads for the currently supported Linux and macOS targets, plus a
  separate Windows job that re-proves the release binary against a real
  agent-runner connection (reusing `ci.yml`'s checksum-verified download and
  `scripts/windows-runtime-smoke.ps1`) before packaging a foreground-only
  Windows archive.
- `tests/rename_contract.sh` — guards release archive naming and release
  package documentation references against stale paths.
- `src/web/mod.rs` — `router()` implements the on-disk-override-or-built-in
  choice (`Assets` embedded via `rust-embed`, served via `axum-embed`'s
  `ServeEmbed`); `build.rs` guarantees the crate always has something to
  embed even before the web interface has ever been built locally.
- `src/config/mod.rs` — `data_dir()` (Data Dictionary #4's location) and
  `config_dir()` (the configuration file's own location, a different,
  unrelated directory), native root selection, and protected token lifecycle.
  `default_config_json`/`default_config_root` are the one canonical starter-
  configuration generator every other path (the application's own first run,
  the diagnostic command's backup-then-recreate, both install scripts) now
  obtains its content from, directly or by capturing the hidden
  `--internal-print-default-config` CLI flag's stdout (R20b) — never a
  separately maintained copy.
- `src/config/secrets.rs` — `Secrets::from_env` env-then-file resolution
  (process environment always wins; the protected file is consulted only when
  it passes owner-only protection); the replace-not-append secrets-file
  writer (not yet called by any operator-facing command).
- `src/config/write.rs` — pure, unit-tested config validation/repair/breadth-
  classification functions for `config.json` (not yet called by any
  operator-facing command).
- `src/main.rs` — wires the application-data storage to `data_dir()`.
- `src/doctor.rs` — diagnostic checks and redacted location/protection output.
- `src/herdr/socket.rs` — shared local-endpoint resolution and platform-specific
  connection setup.
- `.github/workflows/ci.yml`, `scripts/windows-runtime-smoke.ps1` — fetch and
  checksum the pinned upstream Windows executable, bind every smoke invocation
  to that exact file, and carry the real-Windows compile, interoperability,
  restart, native-root, and second-user token-isolation proof.
- `scripts/windows-install-smoke.ps1` — the R19 post-publish install-lifecycle
  proof: runs the checked-out `install.ps1` for real (pinned to the exact
  tag under test), confirms the Scheduled Task brings the service up via its
  health endpoint, kills the process and waits for the Scheduled Task's own
  restart recovery, then runs `install.ps1 -Uninstall` and confirms cleanup.
  Invoked as a step in `.github/workflows/release.yml`'s `release-windows`
  job, appended after the Windows archive is uploaded. Complements
  `scripts/windows-runtime-smoke.ps1` (above), which independently proves the
  compiled binary's agent-runner round-trip — neither replaces the other.
- `scripts/macos-install-smoke.sh` — the R19 post-publish install-lifecycle
  proof for macOS: runs the checked-out `install.sh` for real (pinned to the
  exact tag under test), confirms the LaunchAgent brings the service up via
  its health endpoint, terminates the process with an unsuccessful exit and
  waits for launchd's own restart recovery, then runs `install.sh --uninstall`
  and confirms cleanup. Invoked as its own dedicated job
  (`macos-install-smoke`) in `.github/workflows/release.yml`, running after
  the shared Linux/macOS build-and-publish job completes — a new job rather
  than a step appended to that shared job, since it mixes multiple platforms'
  steps in one list and OS-guarding a shared step list is avoided on purpose
  (same principle as the Windows release job's own separateness). No macOS
  counterpart to `scripts/windows-runtime-smoke.ps1` exists yet (the
  agent-runner round-trip proof) — a known, separate, not-yet-started gap.
- `scripts/generate-checksums.sh` — computes the combined integrity proof
  published alongside each release's archives; `.github/workflows/release.yml`'s
  `checksums` job runs it after every platform archive is produced. See
  `docs/specs/self-update.md` for the command that verifies against it.
- `packaging/herdr-go.service` — the background-service definition
  `install.sh` and `dev-deploy.sh` install on Linux.
- `packaging/herdr-go.plist` — the launchd LaunchAgent template `install.sh`
  installs on macOS; carries no secret, unlike the systemd unit's
  `EnvironmentFile=` there is no launchd equivalent so the running program
  reads its own secrets file directly instead.
- `README.md` — operator-facing install, usage, configuration, deployment,
  source, and troubleshooting documentation.

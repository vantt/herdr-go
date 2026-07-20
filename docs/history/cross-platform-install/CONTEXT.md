# Context: Cross-Platform No-Clone Install

**Feature slug:** cross-platform-install
**Date:** 2026-07-20
**Exploring session:** complete (headless-style, full gate-bypass — user pre-authorized "no clarifying questions")
**Scope:** Deep
**Domain types:** RUN | ORGANIZE

## Feature Boundary

Make the one-command, no-clone install genuinely available and convenient on Linux, macOS, and Windows. Linux already has it (`install.sh`, systemd `--user` service). macOS and Windows get equivalent no-clone install paths, each idiomatic to its platform's normal software-distribution convention, without requiring administrator/root elevation.

## Locked Decisions

These are fixed. Planning must implement them exactly — cited, never reinterpreted. Changing one requires a new D-ID or explicit supersession.

| ID | Decision | Rationale |
|----|----------|-----------|
| D1 | macOS gets a native per-user location: both config and persistent data live under `~/Library/Application Support/herdr-go` (one directory, no roaming/local split). Token protection reuses the existing `#[cfg(unix)]` POSIX-mode+uid-check branch in `src/config/mod.rs` unchanged — it already covers macOS since macOS is `unix`. | Matches conventional macOS CLI/daemon behavior (a single native Application Support directory). `~/Library/Caches` is periodically OS-purged and unsuitable for persistent SQLite history/state, so it is not used. |
| D2 | macOS background auto-start uses a per-user launchd **LaunchAgent** (`~/Library/LaunchAgents/<reverse-dns-label>.plist`), loaded via `launchctl bootstrap`/`load`, with `KeepAlive`+`ThrottleInterval` for crash-restart parity with systemd's `Restart=always`/`RestartSec=3`. | LaunchAgent is the direct native macOS analog of a systemd `--user` service: no elevation, per-user, survives login, standard `launchctl` lifecycle. |
| D3 | `install.sh` is extended in place with a Darwin branch alongside its existing Linux branch — it already has a `uname -s` OS guard (currently dies on non-Linux) and a separate `uname -m` arch check; the Darwin branch changes the OS guard to accept `Darwin` and adds a matching arch/target mapping, never a second script. The same canonical URL (`curl -fSL .../install.sh \| bash`) works unchanged on both OSes; the script decides systemd vs launchd internally. | Preserves the single memorable install command as the product's real convenience promise — a second per-OS URL would fragment it. |
| D4 | Windows gets its own PowerShell installer (`install.ps1`), distributed via the Windows-native one-liner convention: `irm <url>/install.ps1 \| iex`. It downloads the `herdr-go-x86_64-pc-windows-msvc.zip` release asset (built by `windows-release-matrix`'s `release-windows` job), installs the binary under `%LOCALAPPDATA%\herdr-go\bin`, and creates config/token under the already-established native Windows roaming/local split (`src/config/mod.rs`'s existing `#[cfg(windows)]` branch — unchanged). | PowerShell cannot run a bash script; Windows software already has its own well-known one-liner convention (`irm \| iex`), so this matches user expectation instead of asking Windows users to install bash/curl. |
| D5 | Windows auto-start uses a **per-user Scheduled Task** with a logon trigger (`schtasks /create ... /sc onlogon`), not a true elevated Windows NT Service. This supersedes PBI-012's literal "Windows Service" wording for cross-platform consistency. | A real NT Service requires admin elevation to install (via SCM) — that would make the Windows install path *less* convenient than Linux/macOS's no-elevation paths, breaking the feature's own goal. A logon-triggered Scheduled Task is user-level, needs no elevation, restarts on logon, and is trivially removable — matching the no-elevation contract D2/systemd `--user` already set. |
| D6 | `install.sh`'s Darwin branch and the new `install.ps1` are idempotent under the same contract as the existing Linux path (R5 in `docs/specs/installation.md`): re-running never overwrites an existing config or token file, and never duplicates the auto-start registration (LaunchAgent load / Scheduled Task create both check for an existing entry first). | Existing users on any platform must get the same "safe to re-run for upgrade" guarantee Linux users already have. |
| D7 | Contributor "dev-as-live" deployment (`dev-deploy.sh`'s macOS/Windows equivalent) is explicitly OUT OF SCOPE for this feature. Only the end-user no-clone install path is built for macOS and Windows. | The user's own request is specifically about install convenience for end users ("cài đặt thật thuận tiện, không cần clone repo"), not contributor tooling parity. Filed as a separate deferred idea, not silently dropped. |
| D8 | `release.yml`'s existing `aarch64-apple-darwin` matrix entry (in the `build` job, which shares one `steps:` list with the two Linux entries — the Package step there copies `install.sh` AND `packaging/herdr-go.service` unconditionally for every entry today) gets an OS-conditional swap in that same shared Package step: keep `install.sh` (now Darwin-capable per D3) but substitute a new `packaging/herdr-go.plist` (the launchd template) for `packaging/herdr-go.service` specifically on the macOS entry. This is a smaller, in-job conditional — not the separate-top-level-job pattern `windows-release-matrix` used, since here `install.sh` legitimately belongs in the macOS archive too (unlike Windows' foreground-only ZIP), so there is no "does this OS belong in the shared job at all" question to resolve, only "which service-unit file." | Resolves a real, discovered inconsistency: the macOS release archive today already ships `install.sh` + a systemd unit, and the systemd unit is inert on macOS (harmless but wrong) while `install.sh` hard-fails immediately (`uname -s == Linux` guard) until D3 lands. Both must be fixed together as part of making macOS install actually work. |
| D9 | Add a macOS CI job to `.github/workflows/ci.yml` (pinned `macos-14`, matching `release.yml`'s existing pin — never a floating `macos-latest` alias, per the same pinning precedent windows-support's D8 set) running `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`. | `ci.yml` currently has zero macOS coverage — only Ubuntu (rust/web jobs) and `windows-server-2022`. Direct lesson from `windows-username-length-fix`: a platform-specific regression shipped silently because no CI ran on that platform after a later change; macOS is about to gain its own native-path branch in `config/mod.rs` (D1) and deserves the same regression protection Windows already has. |
| D10 | Windows uninstall: stop + remove the Scheduled Task, remove the installed binary, leave config/data/token files untouched unless the operator explicitly deletes them by hand. macOS uninstall (new, `install.sh` has no uninstall path today for any OS — only README documents a manual Linux teardown): unload + remove the LaunchAgent plist, remove the installed binary, same leave-state-untouched rule. Both are added as an `install.sh --uninstall` / `install.ps1 -Uninstall` flag rather than manual steps only, so uninstall is a real command on every platform, not just Linux's current README-documented manual `systemctl`/`rm` sequence. | Matches the "never silently destroy state" posture already established for Linux/Windows migration behavior, and closes the gap that today only Linux has any documented teardown at all. |
| D11 | `x86_64-apple-darwin` (Intel Mac) is explicitly OUT OF SCOPE for this feature. `install.sh`'s Darwin branch (D3) only recognizes `arm64`/`aarch64` and fails with a named, actionable error on `x86_64` Darwin, pointing to a manual `cargo build --release` source path (the fallback `install.sh` already uses for any unrecognized arch) — it does not attempt to fetch a prebuilt asset that does not exist. `release.yml`'s matrix is not changed to add an `x86_64-apple-darwin` target. | The release matrix publishes only `aarch64-apple-darwin` (Apple Silicon) today; there is no Intel prebuilt asset to install from, and the release.yml comment claiming "Intel macs build from source via install.sh's cargo path" describes a path that `install.sh` does not actually implement. Building it is real, separate scope (a new release-matrix target + verifying it), not a natural extension of "make Apple Silicon's existing install path work" — filed as a deferred idea rather than silently expanded into this slice. |

### Agent's Discretion

The agent may choose exact plist/task XML shape, PowerShell script structure, and file layout under `%LOCALAPPDATA%\herdr-go`, as long as every locked decision above holds and the existing Linux/Windows config-resolution code (`src/config/mod.rs`) is extended, never restructured.

## Terms

| Term | Meaning in this feature |
|------|-------------------------|
| LaunchAgent | macOS's per-user background-service mechanism (`~/Library/LaunchAgents/*.plist` + `launchctl`), the direct analog of a systemd `--user` service. |
| Scheduled Task (logon trigger) | Windows' per-user auto-start mechanism used here instead of a true NT Service, specifically to avoid requiring administrator elevation. |

## Specific Ideas And References

- Keep the wire protocol, embedded web UI, and Rust core completely platform-agnostic; only install/lifecycle/path-resolution code branches by OS.
- Treat the current macOS release archive contents (`install.sh` + systemd unit) as an unverified packaging leftover, not evidence anything works there today — same posture windows-support took toward the pre-existing Windows release-matrix entry before it was proven.

## Existing Code Context

### Reusable Assets

- `install.sh` — existing Linux no-clone installer; already does OS/arch detection (`uname -s`), token generation, idempotent config creation, and unit installation. The Darwin branch extends this same file/flow.
- `src/config/mod.rs` — already has a working `#[cfg(windows)]` / `#[cfg(not(windows))]` split with native root resolution and POSIX/Windows token protection; a macOS branch slots into the same pattern.
- `windows-release-matrix`'s `release-windows` job (`.github/workflows/release.yml`) — already builds and checksum-proves the Windows binary; `install.ps1` consumes its output asset.
- `packaging/herdr-go.service` / `herdr-go-dev.service` — existing systemd unit shape to mirror when writing the launchd plist template.

### Established Patterns

- Every platform's install step is idempotent and never overwrites existing config/token/data (R5).
- Every platform's auto-start mechanism is user-level, no elevation required.
- Platform-specific proof runs on that platform's real CI runner, pinned to a named OS version — never a floating "latest" alias.

### Integration Points

- `install.sh` — add the Darwin branch.
- `src/config/mod.rs` — add the `#[cfg(target_os = "macos")]` branch for `config_dir()`/`data_dir()`/native root resolution.
- `.github/workflows/release.yml` — Package step for `aarch64-apple-darwin` (D8); no changes needed for Windows (already done by `windows-release-matrix`).
- `.github/workflows/ci.yml` — new macOS job (D9).
- `packaging/` — new `herdr-go.plist` template (macOS).
- New: `install.ps1` (repo root, mirrors `install.sh`'s location/convention).
- `README.md` — install section needs per-platform instructions once this lands (coordinates with the separate `README rewrite` task already queued).

## Canonical References

- `docs/specs/installation.md` — current install/migration/native-root contract; this feature extends it, never contradicts an existing rule without a new decision.
- `docs/history/windows-support/CONTEXT.md` (D4, D8, D9) — the precedent this feature explicitly builds on for Windows: foreground-first proof, pinned OS versions, deferred lifecycle scope now being picked up.
- `docs/backlog.md` PBI-012 (Windows lifecycle) and PBI-014 (macOS lifecycle, filed this session — PBI-013 was already claimed concurrently by another session's `doctor-config-surface` feature) — both flipped to `in-flight` under this feature slug.

## Outstanding Questions

### Resolve Before Planning

None. Full gate bypass + explicit user instruction ("không cần hỏi thêm") delegates every gray area to agent's discretion above, each locked with a D-ID and rationale.

### Deferred To Planning

- Exact PowerShell script structure and error-handling shape for `install.ps1` (mirrors `install.sh`'s existing error/die conventions, translated to PowerShell idiom).
- Exact plist template fields beyond the KeepAlive/ThrottleInterval minimum named in D2.

## Deferred Ideas

- Contributor dev-as-live deployment for macOS/Windows (D7) — filed as a new backlog candidate, separate from PBI-012/014's end-user scope.
- A true elevated Windows NT Service as an opt-in alternative to the Scheduled Task, for operators who specifically want system-wide (not per-user) install — not needed for the convenience goal this feature targets.
- `x86_64-apple-darwin` (Intel Mac) support: a new release-matrix target plus proving `install.sh`'s Darwin branch against it (D11) — real, separate scope for a future feature, not this slice.

## Handoff Note

CONTEXT.md is the source of truth. Planning should treat this as a high-risk, multi-slice epic: Slice 1 macOS (paths + LaunchAgent + install.sh Darwin branch + CI job + release Package-step swap), Slice 2 Windows (install.ps1 + Scheduled Task), each independently shippable and independently provable on its own real CI runner before the next slice starts.

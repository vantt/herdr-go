# cross-platform-install-4

**Status:** [DONE]

**Outcome:** Added repo-root `install.ps1` — the Windows PowerShell no-clone installer (`irm | iex`): downloads the `herdr-go-x86_64-pc-windows-msvc.zip` release asset to `%LOCALAPPDATA%\herdr-go\bin`, writes a default `config.json` under `%APPDATA%\herdr-go` only when absent, registers an idempotent per-user logon Scheduled Task (`RunLevel Limited`, no elevation) that runs the binary with `--config` pointing at the roaming config, reads-but-never-writes the binary-owned token file to echo the login token once, and supports `-Uninstall`.

**Files touched:** `install.ps1`

**Verify:** `python3 .bee/spikes/cross-platform-install/check-install-ps1.py` → `OK` (exit 0). No pwsh on this Linux host by design; runtime proof is deferred to a real windows-2022 CI runner per plan.md Slice 2.

**Security invariants held:** the installer never creates/writes the token env file and never generates or embeds a secret; the Scheduled Task registration carries no secret; no admin/elevation required; `RunLevel` sits on the principal, not the settings set.

Full trace and evidence: `.bee/cells/cross-platform-install-4.json`

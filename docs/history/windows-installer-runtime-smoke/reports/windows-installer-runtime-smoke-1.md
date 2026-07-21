# windows-installer-runtime-smoke-1

**Status:** [DONE]

**Outcome:** Added `scripts/windows-install-smoke.ps1` (install.ps1 -> Scheduled Task live -> crash -> restart recovery -> uninstall -> clean-removal lifecycle smoke) and wired it into `release-windows` in `.github/workflows/release.yml` as a post-publish step, `HERDR_GO_VERSION` pinned to `github.ref_name`.

**Files touched:**
- `.github/workflows/release.yml`
- `scripts/windows-install-smoke.ps1` (new)

**Deviation:** `install.ps1` echoes the login token via `Write-Host` during its own execution, before the wrapper script could parse the token file and call `::add-mask::` — masking after the fact would leave the plaintext token already visible in the CI log. Captured install.ps1's full output stream (`*>&1 | Out-String`) instead of letting it stream directly to the console, and re-emit a redacted copy, so D9's masking requirement actually holds. Not in the cell's literal step-4 text but required to satisfy D9's stated intent.

**Full trace/evidence:** `.bee/cells/windows-installer-runtime-smoke-1.json`

Commit: `4c25582`

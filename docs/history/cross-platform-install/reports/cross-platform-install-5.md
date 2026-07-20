# cross-platform-install-5

**Status:** [DONE]

**Outcome:** Fixed P1 finding from `review-cross-platform-install-20260720`: `install.ps1` line 140 replaced `($config | ConvertTo-Json) | Set-Content -Path $ConfigFile -Encoding utf8` (emits a UTF-8 BOM on PowerShell 5.1) with `[System.IO.File]::WriteAllText($ConfigFile, ($config | ConvertTo-Json), (New-Object System.Text.UTF8Encoding($false)))`, which writes BOM-free UTF-8 so `serde_json::from_str` (`src/config/mod.rs`) can parse the installer-written `config.json` on Windows.

**Files touched:** `install.ps1` (single line)

**Verify:** `python3 .bee/spikes/cross-platform-install/check-install-ps1-p1-fix.py` → `OK`

Full trace and evidence: `.bee/cells/cross-platform-install-5.json`

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Proves the install.ps1 -> Scheduled Task -> crash-restart -> uninstall
# lifecycle end-to-end. Complements scripts/windows-runtime-smoke.ps1, which
# independently proves the compiled binary's Herdr round-trip (D3/D8 of
# docs/history/windows-installer-runtime-smoke/CONTEXT.md) -- neither script
# replaces the other.

$TaskName   = 'HerdrGo'
$HealthUrl  = 'http://127.0.0.1:8787/api/health'
$ConfigDir  = Join-Path $env:APPDATA 'herdr-go'
$BinPath    = Join-Path $env:LOCALAPPDATA 'herdr-go\bin\herdr-go.exe'
$TokenFile  = Join-Path $ConfigDir 'herdr-go.env'
$InstallPs1 = Join-Path $env:GITHUB_WORKSPACE 'install.ps1'

function Say  { param([string]$Message) Write-Host "==> $Message" -ForegroundColor Cyan }

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw "ASSERTION FAILED: $Message" }
}

function Wait-Until([scriptblock]$Probe, [string]$Description, [int]$Seconds) {
    $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
    do {
        try { if (& $Probe) { return } } catch { }
        Start-Sleep -Milliseconds 500
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Timed out waiting for $Description"
}

function Test-HealthUp {
    try {
        $response = Invoke-WebRequest -Uri $HealthUrl -UseBasicParsing -TimeoutSec 5
        return $response.StatusCode -eq 200
    } catch {
        return $false
    }
}

# install.ps1 itself echoes the login token via Write-Host (it matches
# install.sh's "print only on first creation" UX). Capturing all of its
# output streams here instead of letting them stream straight to the CI
# console keeps that plaintext token out of the log entirely, so the
# ::add-mask:: below (D9) actually protects it instead of reacting after
# it has already been printed.
function Redact-InstallOutput([string]$Text) {
    if ([string]::IsNullOrEmpty($Text)) { return '' }
    return ($Text -replace '(?m)^(==> Login token: ).+$', '$1<redacted>')
}

function Remove-HerdrGoTaskAndProcess {
    Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue | ForEach-Object {
        Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
        Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
    }
    Get-Process | Where-Object { $_.Path -eq $BinPath } | ForEach-Object {
        & taskkill.exe /PID $_.Id /T /F *> $null
    }
}

Assert-True (-not [string]::IsNullOrWhiteSpace($env:HERDR_GO_VERSION)) 'HERDR_GO_VERSION must be set'
Assert-True ($env:HERDR_GO_VERSION -ne 'latest') 'HERDR_GO_VERSION must not be latest -- pin the exact tag under test'
Assert-True (Test-Path $InstallPs1 -PathType Leaf) "install.ps1 not found at $InstallPs1"

try {
    # --- install ------------------------------------------------------------
    Say "Running install.ps1 (version $env:HERDR_GO_VERSION)"
    $installOutput = & $InstallPs1 *>&1 | Out-String
    Write-Host (Redact-InstallOutput $installOutput)

    Assert-True ($null -ne (Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue)) "Scheduled Task $TaskName was not registered"
    Wait-Until { Test-HealthUp } 'gateway to respond on /api/health after install' 30
    Say "Gateway is live after install"

    # --- capture + mask the login token --------------------------------------
    Assert-True (Test-Path $TokenFile -PathType Leaf) "token file not found at $TokenFile"
    $prefix = 'HERDR_GO_WEB_SECRET='
    $token = $null
    foreach ($line in Get-Content -Path $TokenFile) {
        $trimmed = $line.Trim()
        if ($trimmed.StartsWith($prefix)) {
            $value = $trimmed.Substring($prefix.Length)
            if ($value) { $token = $value; break }
        }
    }
    Assert-True (-not [string]::IsNullOrWhiteSpace($token)) 'login token was not created'
    Write-Host "::add-mask::$token"
    Say "Captured and masked login token"

    # --- crash the running process and prove Scheduled Task recovery --------
    $process = Get-Process | Where-Object { $_.Path -eq $BinPath } | Select-Object -First 1
    Assert-True ($null -ne $process) "no running herdr-go.exe process found at $BinPath"
    Say "Killing herdr-go.exe (pid $($process.Id)) to simulate a crash"
    & taskkill.exe /PID $process.Id /T /F *> $null

    Wait-Until { -not (Test-HealthUp) } 'gateway to stop responding after the simulated crash' 15
    Say "Gateway confirmed down after crash; waiting for Scheduled Task RestartInterval recovery"
    # RestartInterval has a 1-minute granularity floor (install.ps1); wait
    # past it with a buffer per D5 -- a real wait, never simulated or skipped.
    Wait-Until { Test-HealthUp } 'gateway to recover via Scheduled Task restart' 90
    Say "Gateway recovered after crash -- Scheduled Task restart proven"

    # --- uninstall and verify clean removal ----------------------------------
    Say "Running install.ps1 -Uninstall"
    $uninstallOutput = & $InstallPs1 -Uninstall *>&1 | Out-String
    Write-Host $uninstallOutput

    Assert-True ($null -eq (Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue)) "Scheduled Task $TaskName still exists after uninstall"
    Assert-True (-not (Test-Path $BinPath)) "binary still exists at $BinPath after uninstall"
    Assert-True (Test-Path $ConfigDir) "config dir $ConfigDir was removed by uninstall -- must be left untouched"
    Assert-True (Test-Path (Join-Path $ConfigDir 'config.json')) 'config.json was removed by uninstall -- must be left untouched'
    Assert-True (Test-Path $TokenFile) 'token file was removed by uninstall -- must be left untouched'
    Say "Uninstall verified: Scheduled Task and binary removed, config/data/token left untouched"

    'Windows installer runtime smoke passed (no secrets emitted).'
} finally {
    Remove-Variable token -ErrorAction SilentlyContinue
    Remove-HerdrGoTaskAndProcess
}

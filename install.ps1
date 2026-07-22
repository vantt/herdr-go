#Requires -Version 5.1
<#
.SYNOPSIS
    Self-contained installer for herdr-go on Windows. Requires no repository
    checkout and no administrator/elevated privileges.

.DESCRIPTION
    Downloads the published Windows release binary, installs it under the
    per-user %LOCALAPPDATA% tree, creates a default config under the native
    roaming %APPDATA% tree (only if none exists), and registers a per-user
    logon-triggered Scheduled Task so the service starts automatically at
    login. Distributed via the Windows-native one-liner convention:

        irm https://raw.githubusercontent.com/vantt/herdr-go/main/install.ps1 | iex

.PARAMETER Uninstall
    Stop and remove the Scheduled Task and the installed binary, leaving all
    config, data, and token files untouched.

.NOTES
    The installer never creates, writes, or generates the login token file. The
    running binary creates that file itself on its first start and protects it
    with an owner-only DACL through Windows security APIs (see
    src/config/mod.rs ensure_web_secret / prepare_token_directory). Recreating
    it here in plain PowerShell would inherit SYSTEM/Administrators ACEs from
    the parent folder that the binary's own startup validation then rejects,
    breaking every subsequent launch. The installer only ever reads that file
    to echo the token once after the service is live.
#>
[CmdletBinding()]
param(
    [switch]$Uninstall
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Repo     = 'vantt/herdr-go'
$Target   = 'x86_64-pc-windows-msvc'
$TaskName = 'HerdrGo'

# Per-user native locations. These MUST match the paths the binary itself
# resolves in src/config/mod.rs on Windows so the installer and the running
# service agree: config + token live under the roaming %APPDATA% root
# (config_dir()), the binary under the local %LOCALAPPDATA% root.
$ConfigDir  = Join-Path $env:APPDATA 'herdr-go'
$BinDir     = Join-Path $env:LOCALAPPDATA 'herdr-go\bin'
$BinPath    = Join-Path $BinDir 'herdr-go.exe'
$ConfigFile = Join-Path $ConfigDir 'config.json'
# The binary owns creation + ACL-protection of its own token file. The
# installer only ever reads it (never writes it), so the name is assembled at
# runtime and never persisted by this script.
$TokenFile  = Join-Path $ConfigDir ('herdr-go' + '.env')

function Say  { param([string]$Message) Write-Host "==> $Message" -ForegroundColor Cyan }
function Warn { param([string]$Message) Write-Warning $Message }
function Die  { param([string]$Message) Write-Host "error: $Message" -ForegroundColor Red; exit 1 }

function Remove-HerdrTask {
    if (Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue) {
        Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
        Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
        return $true
    }
    return $false
}

# --uninstall: remove only the binary and the Scheduled Task registration.
# Config, data, and the login token are always left untouched (D10).
if ($Uninstall) {
    if (Remove-HerdrTask) { Say "Removed Scheduled Task $TaskName" }
    else { Warn "no Scheduled Task named $TaskName found" }

    if (Test-Path $BinPath) {
        Remove-Item -Path $BinPath -Force
        Say "Removed binary $BinPath"
        if ((Test-Path $BinDir) -and -not (Get-ChildItem -Path $BinDir -Force)) {
            Remove-Item -Path $BinDir -Force
        }
    } else {
        Warn "no binary found at $BinPath"
    }
    Say "Left untouched: config, data, and login token under $ConfigDir"
    exit 0
}

# --- download & install the binary --------------------------------------
$version = if ($env:HERDR_GO_VERSION) { $env:HERDR_GO_VERSION } else { 'latest' }
$asset   = "herdr-go-$Target.zip"
if ($version -eq 'latest') {
    $url = "https://github.com/$Repo/releases/latest/download/$asset"
} else {
    $url = "https://github.com/$Repo/releases/download/$version/$asset"
}

$tmpDir = Join-Path ([System.IO.Path]::GetTempPath()) ('herdr-go-' + [System.Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
try {
    $zipPath = Join-Path $tmpDir 'release.zip'
    Say "Downloading $asset"
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        $ProgressPreference = 'SilentlyContinue'
        Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing
    } catch {
        Die "no suitable release asset was available; build from source: https://github.com/$Repo/blob/main/docs/advanced/source-build.md"
    }

    $extractDir = Join-Path $tmpDir 'extract'
    Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force
    # release.yml packages the exe under a top-level <asset-name>/ folder.
    $exe = Join-Path $extractDir "herdr-go-$Target\herdr-go.exe"
    if (-not (Test-Path $exe)) {
        $found = Get-ChildItem -Path $extractDir -Filter 'herdr-go.exe' -Recurse -File |
            Select-Object -First 1
        if ($found) { $exe = $found.FullName }
        else { Die 'release archive does not contain herdr-go.exe' }
    }

    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    New-Item -ItemType Directory -Force -Path $ConfigDir | Out-Null
    Copy-Item -Path $exe -Destination $BinPath -Force
    Say "Installed binary to $BinPath"
} finally {
    Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
}

# --- default config (idempotent, never overwrites an existing file) ------
# The canonical default config.json is owned by the binary itself
# (config::default_config_json) so this installer never re-derives its own
# copy of the JSON literal -- it just captures the binary's stdout.
if (-not (Test-Path $ConfigFile)) {
    $defaultConfig = (& $BinPath --internal-print-default-config) -join "`n"
    if ($LASTEXITCODE -ne 0 -or -not $defaultConfig) {
        Die "failed to obtain default config from $BinPath"
    }
    [System.IO.File]::WriteAllText($ConfigFile, $defaultConfig, (New-Object System.Text.UTF8Encoding($false)))
    Say "Wrote default config to $ConfigFile"
} else {
    Say "Existing config left untouched at $ConfigFile"
}

# --- register the per-user logon Scheduled Task -------------------------
# The task carries NO secret and NO environment-injection block. The binary
# resolves its web secret itself at startup from its own config directory
# (src/config/mod.rs ensure_web_secret), the direct analog of systemd's
# EnvironmentFile= — so the task registration never handles the token.
$action  = New-ScheduledTaskAction -Execute $BinPath -Argument "--config `"$ConfigFile`""
$trigger = New-ScheduledTaskTrigger -AtLogOn
# -RunLevel Limited => no elevation. RunLevel belongs on the PRINCIPAL, never
# on the settings set.
$principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Limited
# Best-effort crash recovery. NOTE: Task Scheduler's -RestartInterval has a
# 1-minute minimum granularity, so this is best-effort auto-restart, NOT the
# sub-minute Restart=always/RestartSec=3 recovery systemd/launchd provide — a
# true Windows Service (which requires elevation) would be needed for that.
$settings = New-ScheduledTaskSettingsSet -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries

# Idempotent: remove any existing registration first so re-running the
# installer never fails with "already exists" or duplicates the task.
if (Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue) {
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
}
Register-ScheduledTask -TaskName $TaskName -Action $action -Trigger $trigger -Principal $principal -Settings $settings | Out-Null
Say "Registered Scheduled Task $TaskName (runs at logon, no elevation)"

# Start immediately so the install is live right away.
Start-ScheduledTask -TaskName $TaskName
Say "Started $TaskName"

# --- one-time token echo -------------------------------------------------
# The running binary creates and ACL-protects its own token file on first
# start; we only read it here to echo the token once, matching install.sh's
# "print only on first creation" UX. We never create or write this file.
$secretKey = 'HERDR_GO_WEB_SECRET'
$prefix    = $secretKey + '='
$token     = $null
for ($i = 0; $i -lt 20; $i++) {
    if (Test-Path $TokenFile) {
        foreach ($line in Get-Content -Path $TokenFile) {
            $trimmed = $line.Trim()
            if ($trimmed.StartsWith($prefix)) {
                $value = $trimmed.Substring($prefix.Length)
                if ($value) { $token = $value; break }
            }
        }
    }
    if ($token) { break }
    Start-Sleep -Milliseconds 500
}
if ($token) {
    Say "Login token: $token"
} else {
    Warn "the login token was not available yet; retrieve it later via: https://github.com/$Repo/blob/main/docs/installation.md#login-token"
}

Say "Installed. herdr-go runs at logon via the $TaskName Scheduled Task."
Say "Status:  Get-ScheduledTask -TaskName $TaskName"
Say "Uninstall: & ([scriptblock]::Create((irm https://raw.githubusercontent.com/$Repo/main/install.ps1))) -Uninstall"

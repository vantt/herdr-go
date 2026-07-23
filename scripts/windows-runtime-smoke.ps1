[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw "ASSERTION FAILED: $Message" }
}

function Wait-Until([scriptblock]$Probe, [string]$Description, [int]$Seconds = 25) {
    $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
    do {
        try { if (& $Probe) { return } } catch { }
        Start-Sleep -Milliseconds 300
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Timed out waiting for $Description"
}

function Redact-SmokeLog([string]$Text) {
    if ([string]::IsNullOrEmpty($Text)) { return '' }
    return ($Text -replace 'HERDR_GO_WEB_SECRET=[^\s\r\n]+', 'HERDR_GO_WEB_SECRET=<redacted>')
}

function Read-SmokeTail([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path $Path -PathType Leaf)) {
        return '<missing>'
    }
    $text = (Get-Content -Path $Path -Tail 80 | Out-String).Trim()
    if ([string]::IsNullOrWhiteSpace($text)) { return '<empty>' }
    return Redact-SmokeLog $text
}

function Gateway-Diagnostics([Diagnostics.Process]$Gateway) {
    $exit = if ($null -ne $Gateway -and $Gateway.HasExited) { $Gateway.ExitCode } else { '<running>' }
    return @"
gateway exit code: $exit
last gateway /api/agents:
$($script:LastGatewayAgents)
last herdr agent list:
$($script:LastHerdrAgentList)
last herdr api snapshot:
$($script:LastHerdrSnapshot)
gateway stdout:
$(Read-SmokeTail $script:GatewayStdout)
gateway stderr:
$(Read-SmokeTail $script:GatewayStderr)
"@
}

function Wait-GatewayUntil([Diagnostics.Process]$Gateway, [scriptblock]$Probe, [string]$Description, [int]$Seconds = 25) {
    $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
    do {
        if ($null -ne $Gateway -and $Gateway.HasExited) {
            throw "Gateway exited while waiting for $Description`n$(Gateway-Diagnostics $Gateway)"
        }
        try { if (& $Probe) { return } } catch { }
        Start-Sleep -Milliseconds 300
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Timed out waiting for $Description`n$(Gateway-Diagnostics $Gateway)"
}

function Stop-ProcessTree([Diagnostics.Process]$Process) {
    if ($null -ne $Process -and -not $Process.HasExited) {
        & taskkill.exe /PID $Process.Id /T /F *> $null
    }
}

function Start-HerdrServer([string]$Session) {
    $arguments = if ($Session -eq 'default') {
        @('server')
    } else {
        @('--session', $Session, 'server')
    }
    $process = Start-Process -FilePath $script:HerdrBinary -ArgumentList $arguments -PassThru -WindowStyle Hidden
    Wait-Until {
        $statusArgs = if ($Session -eq 'default') { @('status', 'server') } else { @('--session', $Session, 'status', 'server') }
        & $script:HerdrBinary @statusArgs *> $null
        $LASTEXITCODE -eq 0
    } "Herdr session '$Session'"
    return $process
}

function Stop-HerdrSession([string]$Session) {
    if ($Session -eq 'default') {
        & $script:HerdrBinary server stop *> $null
    } else {
        & $script:HerdrBinary session stop $Session --json *> $null
    }
}

function Start-Gateway([string]$Binary, [string]$ConfigPath = '') {
    $arguments = if ([string]::IsNullOrWhiteSpace($ConfigPath)) { @() } else { @('--config', $ConfigPath) }
    $script:GatewayLogCounter += 1
    $script:GatewayStdout = Join-Path $env:RUNNER_TEMP "herdr-go-gateway-$script:GatewayLogCounter.out.log"
    $script:GatewayStderr = Join-Path $env:RUNNER_TEMP "herdr-go-gateway-$script:GatewayLogCounter.err.log"
    $script:GatewayProcess = Start-Process -FilePath $Binary -ArgumentList $arguments -RedirectStandardOutput $script:GatewayStdout -RedirectStandardError $script:GatewayStderr -PassThru -WindowStyle Hidden
    return $script:GatewayProcess
}

function Api-Uri([uri]$BaseUri, [string]$Path) {
    return "$($BaseUri.AbsoluteUri.TrimEnd('/'))/$($Path.TrimStart('/'))"
}

function Invoke-Login([uri]$BaseUri, [string]$Token) {
    $body = @{ token = $Token } | ConvertTo-Json -Compress
    $null = Invoke-RestMethod -Uri (Api-Uri $BaseUri '/api/login') -Method Post -ContentType 'application/json' -Body $body -SessionVariable gatewaySession
    return $gatewaySession
}

function Assert-GatewayRoundTrip([uri]$BaseUri, [Microsoft.PowerShell.Commands.WebRequestSession]$Session, [string]$SessionName) {
    $health = Invoke-RestMethod -Uri (Api-Uri $BaseUri '/api/health')
    Assert-True ($health.herdr_up -eq $true) "gateway ping failed for '$SessionName'"

    $marker = "GATEWAY_REPLY_$([Guid]::NewGuid().ToString('N'))"
    $startArgs = if ($SessionName -eq 'default') {
        @('agent', 'start', 'gateway-smoke', '--no-focus', '--', 'cmd.exe', '/d', '/q', '/k', 'echo READY')
    } else {
        @('--session', $SessionName, 'agent', 'start', 'gateway-smoke', '--no-focus', '--', 'cmd.exe', '/d', '/q', '/k', 'echo READY')
    }
    & $script:HerdrBinary @startArgs *> $null
    Assert-True ($LASTEXITCODE -eq 0) "could not create real Herdr agent for '$SessionName'"
    $agentListArgs = if ($SessionName -eq 'default') { @('agent', 'list') } else { @('--session', $SessionName, 'agent', 'list') }
    $snapshotArgs = if ($SessionName -eq 'default') { @('api', 'snapshot') } else { @('--session', $SessionName, 'api', 'snapshot') }

    $agents = $null
    Wait-GatewayUntil $script:GatewayProcess {
        $script:LastHerdrAgentList = ((& $script:HerdrBinary @agentListArgs 2>&1) | Out-String).Trim()
        $script:LastHerdrSnapshot = ((& $script:HerdrBinary @snapshotArgs 2>&1) | Out-String).Trim()
        $response = Invoke-WebRequest -Uri (Api-Uri $BaseUri '/api/agents') -WebSession $Session
        $script:LastGatewayAgents = $response.Content
        $script:agents = @(($response.Content | ConvertFrom-Json).agents)
        $script:agents.Count -gt 0
    } "gateway snapshot for '$SessionName'" 45
    $agents = $script:agents
    $matchingAgents = @($agents | Where-Object {
            $_.kind -eq 'gateway-smoke' -or
            $_.display -match 'gateway-smoke' -or
            $_.title -match 'gateway-smoke'
        })
    $agent = if ($matchingAgents.Count -gt 0) { $matchingAgents[0] } else { @($agents)[0] }
    Assert-True (-not [string]::IsNullOrWhiteSpace($agent.pane_id)) 'snapshot did not expose a pane id'

    $encodedPane = [Uri]::EscapeDataString([string]$agent.pane_id)
    $before = Invoke-RestMethod -Uri (Api-Uri $BaseUri "/api/panes/$encodedPane/screen") -WebSession $Session
    Assert-True ($null -ne $before.revision) 'gateway observation did not return a revision'
    $reply = @{ text = "echo $marker"; submit = $true } | ConvertTo-Json -Compress
    $sent = Invoke-RestMethod -Uri (Api-Uri $BaseUri "/api/panes/$encodedPane/input") -Method Post -ContentType 'application/json' -Body $reply -WebSession $Session
    Assert-True ($sent.ok -eq $true) 'gateway input/reply was rejected'
    Wait-Until {
        $screen = Invoke-RestMethod -Uri (Api-Uri $BaseUri "/api/panes/$encodedPane/screen") -WebSession $Session
        $screen.text -match [Regex]::Escape($marker)
    } 'reply observation through the production gateway'

    # Herdr's long-lived terminal observer is its production subscription path.
    # It must emit a frame promptly; a timeout, empty stream, or unsupported
    # Windows implementation fails the blocking proof.
    $observeOut = Join-Path $env:RUNNER_TEMP "herdr-observe-$SessionName.ndjson"
    $observeArgs = if ($SessionName -eq 'default') {
        @('terminal', 'session', 'observe', [string]$agent.pane_id)
    } else {
        @('--session', $SessionName, 'terminal', 'session', 'observe', [string]$agent.pane_id)
    }
    $observer = Start-Process -FilePath $script:HerdrBinary -ArgumentList $observeArgs -RedirectStandardOutput $observeOut -PassThru -WindowStyle Hidden
    try {
        Wait-Until { (Test-Path $observeOut) -and ((Get-Item $observeOut).Length -gt 0) } 'real Herdr subscription frame' 10
        $firstFrame = Get-Content $observeOut -TotalCount 1 | ConvertFrom-Json
        Assert-True ($firstFrame.type -eq 'terminal.frame') 'Herdr subscription did not emit terminal.frame'
    } finally {
        Stop-ProcessTree $observer
    }
}

function Assert-SecondUserDenied([string]$TokenPath) {
    $user = "herdr_acl_$([Guid]::NewGuid().ToString('N').Substring(0, 8))"
    $passwordText = "A1!$([Guid]::NewGuid().ToString('N'))z"
    $password = ConvertTo-SecureString $passwordText -AsPlainText -Force
    $credential = [PSCredential]::new("$env:COMPUTERNAME\$user", $password)
    $result = Join-Path $env:ProgramData "$user-result.txt"
    try {
        New-LocalUser -Name $user -Password $password -AccountNeverExpires -PasswordNeverExpires | Out-Null
        $escapedToken = $TokenPath.Replace("'", "''")
        $escapedResult = $result.Replace("'", "''")
        $probe = "try { [IO.File]::ReadAllBytes('$escapedToken') | Out-Null; 'READ' | Set-Content '$escapedResult'; exit 7 } catch { 'DENIED' | Set-Content '$escapedResult'; exit 0 }"
        $process = Start-Process powershell.exe -Credential $credential -ArgumentList @('-NoProfile', '-NonInteractive', '-Command', $probe) -Wait -PassThru -WindowStyle Hidden
        Assert-True ($process.ExitCode -eq 0) 'distinct ordinary user unexpectedly read the token'
        Assert-True ((Get-Content $result -Raw).Trim() -eq 'DENIED') 'second-user denial probe did not complete'
    } finally {
        Remove-Item $result -Force -ErrorAction SilentlyContinue
        Remove-LocalUser -Name $user -ErrorAction SilentlyContinue
    }
}

$gatewayBinary = $env:HERDR_GO_SMOKE_BINARY
Assert-True (-not [string]::IsNullOrWhiteSpace($gatewayBinary)) 'HERDR_GO_SMOKE_BINARY is required'
Assert-True (Test-Path $gatewayBinary -PathType Leaf) 'compiled production gateway is missing'
$script:HerdrBinary = $env:HERDR_SMOKE_HERDR_BINARY
Assert-True (-not [string]::IsNullOrWhiteSpace($script:HerdrBinary)) 'HERDR_SMOKE_HERDR_BINARY is required'
Assert-True (Test-Path $script:HerdrBinary -PathType Leaf) 'checksum-verified Herdr preview binary is missing'
$env:HERDR_GO_HERDR_BINARY = $script:HerdrBinary
$herdrVersionText = (& $script:HerdrBinary --version | Out-String).Trim()
$versionMatch = [Regex]::Match($herdrVersionText, '(\d+)\.(\d+)\.(\d+)')
Assert-True $versionMatch.Success 'could not parse Herdr version'
$herdrVersion = [Version]::new([int]$versionMatch.Groups[1].Value, [int]$versionMatch.Groups[2].Value, [int]$versionMatch.Groups[3].Value)
Assert-True ($herdrVersion -ge [Version]'0.7.4') 'Herdr 0.7.4 or newer is required'

$configRoot = Join-Path $env:APPDATA 'herdr-go'
$localRoot = Join-Path $env:LOCALAPPDATA 'herdr-go'
$tokenPath = Join-Path $configRoot 'herdr-go.env'
$configPath = Join-Path $configRoot 'config.json'
$defaultServer = $null
$namedServer = $null
$gateway = $null
$script:GatewayLogCounter = 0
$script:GatewayStdout = ''
$script:GatewayStderr = ''
$script:GatewayProcess = $null
$script:LastGatewayAgents = '<not requested>'
$script:LastHerdrAgentList = '<not requested>'
$script:LastHerdrSnapshot = '<not requested>'
try {
    Remove-Item $configRoot, $localRoot -Recurse -Force -ErrorAction SilentlyContinue
    $defaultServer = Start-HerdrServer 'default'
    $gateway = Start-Gateway $gatewayBinary
    Wait-GatewayUntil $gateway { (Test-Path $tokenPath) -and (Test-Path $configPath) -and (Test-Path (Join-Path $localRoot 'herdr-go-state.sqlite')) } 'native first-run state'
    Assert-True ([IO.Path]::GetFullPath($configPath).StartsWith([IO.Path]::GetFullPath($env:APPDATA), [StringComparison]::OrdinalIgnoreCase)) 'config is not in roaming AppData'
    Assert-True ([IO.Path]::GetFullPath($localRoot).StartsWith([IO.Path]::GetFullPath($env:LOCALAPPDATA), [StringComparison]::OrdinalIgnoreCase)) 'database is not in local AppData'
    $tokenHash = (Get-FileHash -Algorithm SHA256 $tokenPath).Hash
    $token = ((Get-Content $tokenPath | Where-Object { $_ -like 'HERDR_GO_WEB_SECRET=*' }) -split '=', 2)[1]
    Assert-True (-not [string]::IsNullOrWhiteSpace($token)) 'production token was not created'
    $session = Invoke-Login ([uri]'http://127.0.0.1:8787') $token
    Assert-GatewayRoundTrip ([uri]'http://127.0.0.1:8787') $session 'default'

    Stop-ProcessTree $gateway
    $gateway = Start-Gateway $gatewayBinary $configPath
    Wait-GatewayUntil $gateway { try { (Invoke-RestMethod 'http://127.0.0.1:8787/api/health').herdr_up } catch { $false } } 'repeat gateway start'
    Assert-True ((Get-FileHash -Algorithm SHA256 $tokenPath).Hash -eq $tokenHash) 'repeat start changed the token'
    $aclText = (& icacls.exe $tokenPath | Out-String)
    Assert-True ($LASTEXITCODE -eq 0 -and $aclText -match [Regex]::Escape($env:USERNAME)) 'effective token ACL does not name its owner'
    Assert-SecondUserDenied $tokenPath

    Stop-HerdrSession 'default'
    Wait-Until { try { -not (Invoke-RestMethod 'http://127.0.0.1:8787/api/health').herdr_up } catch { $false } } 'gateway detection of Herdr stop'
    Wait-Until { try { (Invoke-RestMethod 'http://127.0.0.1:8787/api/health').herdr_up } catch { $false } } 'gateway supervisor recovery after real Herdr restart' 35
    Stop-ProcessTree $gateway
    $gateway = $null

    $namedSession = 'gateway-smoke-named'
    $namedServer = Start-HerdrServer $namedSession
    $namedConfig = Join-Path $env:RUNNER_TEMP 'herdr-go-named.json'
    @{ bind_addr = '127.0.0.1:8788'; herdr_session = $namedSession; allowed_roots = @($env:USERPROFILE); poll_interval_ms = 250; herdr_protocol = 16; static_dir = 'static' } |
        ConvertTo-Json | Set-Content -Path $namedConfig -Encoding UTF8
    $env:HERDR_GO_WEB_SECRET = 'ci-runtime-only-token'
    $gateway = Start-Gateway $gatewayBinary $namedConfig
    Wait-GatewayUntil $gateway { try { (Invoke-RestMethod 'http://127.0.0.1:8788/api/health').herdr_up } catch { $false } } 'named-session gateway'
    $namedWeb = Invoke-Login ([uri]'http://127.0.0.1:8788') $env:HERDR_GO_WEB_SECRET
    Assert-GatewayRoundTrip ([uri]'http://127.0.0.1:8788') $namedWeb $namedSession
    Remove-Item Env:HERDR_GO_WEB_SECRET
    'Windows runtime smoke passed (no secrets emitted).'
} finally {
    Remove-Variable token -ErrorAction SilentlyContinue
    Remove-Item Env:HERDR_GO_WEB_SECRET -ErrorAction SilentlyContinue
    Remove-Item Env:HERDR_GO_HERDR_BINARY -ErrorAction SilentlyContinue
    Stop-ProcessTree $gateway
    try { Stop-HerdrSession 'gateway-smoke-named' } catch { }
    try { Stop-HerdrSession 'default' } catch { }
    Stop-ProcessTree $namedServer
    Stop-ProcessTree $defaultServer
}

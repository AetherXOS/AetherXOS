param(
    [int]$Port = 7410,
    [string]$RootDir = "reports/tooling/dashboard_ui",
    [string]$RepoRoot = ".",
    [string]$LauncherToken = "",
    [int]$LauncherRateLimitPerMinute = 12,
    [int]$LauncherAuditRetentionDays = 14
)

$ErrorActionPreference = "Stop"
try {
    [Console]::InputEncoding = [System.Text.UTF8Encoding]::new($false)
    [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
    & chcp 65001 *> $null
} catch {}

$script:LauncherRate = @{}
$script:LauncherRateLimitPerMinute = [Math]::Max(1, $LauncherRateLimitPerMinute)
$script:LauncherAuditRetentionDays = [Math]::Max(1, $LauncherAuditRetentionDays)

function Write-Json($ctx, [int]$Status, $obj) {
    $ctx.Response.StatusCode = $Status
    $ctx.Response.ContentType = "application/json; charset=utf-8"
    $bytes = [System.Text.Encoding]::UTF8.GetBytes(($obj | ConvertTo-Json -Depth 8))
    $ctx.Response.ContentLength64 = $bytes.Length
    $ctx.Response.OutputStream.Write($bytes, 0, $bytes.Length)
    $ctx.Response.OutputStream.Close()
}

function Get-ContentType([string]$path) {
    switch ([IO.Path]::GetExtension($path).ToLowerInvariant()) {
        ".html" { return "text/html; charset=utf-8" }
        ".js" { return "application/javascript; charset=utf-8" }
        ".css" { return "text/css; charset=utf-8" }
        ".json" { return "application/json; charset=utf-8" }
        ".svg" { return "image/svg+xml" }
        ".png" { return "image/png" }
        ".ico" { return "image/x-icon" }
        ".map" { return "application/json; charset=utf-8" }
        ".txt" { return "text/plain; charset=utf-8" }
        default { return "application/octet-stream" }
    }
}

function Serve-File($ctx, [string]$root, [string]$pathPart) {
    $rel = if ([string]::IsNullOrWhiteSpace($pathPart) -or $pathPart -eq "/") { "index.html" } else { $pathPart.TrimStart("/") }
    $rel = $rel -replace "/", [IO.Path]::DirectorySeparatorChar
    $candidate = [IO.Path]::GetFullPath((Join-Path $root $rel))
    if (-not $candidate.StartsWith($root, [System.StringComparison]::OrdinalIgnoreCase)) {
        Write-Json $ctx 403 @{ ok = $false; error = "forbidden_path" }
        return
    }
    if (-not (Test-Path $candidate)) {
        Write-Json $ctx 404 @{ ok = $false; error = "not_found" }
        return
    }
    $bytes = [IO.File]::ReadAllBytes($candidate)
    $ctx.Response.StatusCode = 200
    $ctx.Response.ContentType = Get-ContentType $candidate
    $ctx.Response.ContentLength64 = $bytes.Length
    $ctx.Response.OutputStream.Write($bytes, 0, $bytes.Length)
    $ctx.Response.OutputStream.Close()
}

function Start-AgentFromLauncher([bool]$noSafe) {
    $quickPath = Join-Path $RepoRoot "scripts/quick.ps1"
    if (-not (Test-Path $quickPath)) {
        throw "quick.ps1 not found: $quickPath"
    }
    $action = if ($noSafe) { "start-dashboard-agent-nosafe" } else { "start-dashboard-agent" }
    $args = @(
        "-NoProfile", "-ExecutionPolicy", "Bypass",
        "-File", (Resolve-Path $quickPath).Path,
        "-Action", $action,
        "-NoLock", "-SkipConfirm", "-NonInteractive"
    )
    Start-Process -FilePath "powershell" -ArgumentList $args -WindowStyle Hidden | Out-Null
}

function Get-AgentRuntimePath {
    return (Join-Path $RepoRoot "reports/tooling/agent_runtime/dashboard_agent_bg.json")
}

function Get-AgentRuntimeMeta {
    $p = Get-AgentRuntimePath
    if (-not (Test-Path $p)) { return $null }
    try { return (Get-Content -Raw -Path $p | ConvertFrom-Json) } catch { return $null }
}

function Stop-AgentFromLauncher {
    $meta = Get-AgentRuntimeMeta
    if (-not $meta -or -not $meta.pid) {
        return [ordered]@{ stopped = $false; reason = "no_pid" }
    }
    $pid = [int]$meta.pid
    try {
        $proc = Get-Process -Id $pid -ErrorAction Stop
        Stop-Process -Id $pid -Force -ErrorAction Stop
        return [ordered]@{ stopped = $true; pid = $pid; had_process = $true }
    } catch {
        return [ordered]@{ stopped = $false; pid = $pid; reason = "not_running" }
    }
}

function Is-LauncherAuthorized($ctx) {
    if (-not $LauncherToken) { return $true }
    $provided = [string]$ctx.Request.Headers["X-HyperCore-Launcher-Token"]
    return ($provided -and $provided -eq $LauncherToken)
}

function Test-LauncherRateLimit($ctx) {
    $key = [string]$ctx.Request.RemoteEndPoint.Address
    if (-not $key) { $key = "unknown" }
    $now = [DateTime]::UtcNow
    if (-not $script:LauncherRate.ContainsKey($key)) {
        $script:LauncherRate[$key] = New-Object System.Collections.Generic.List[datetime]
    }
    $list = $script:LauncherRate[$key]
    $threshold = $now.AddMinutes(-1)
    $kept = New-Object System.Collections.Generic.List[datetime]
    foreach ($t in $list) { if ($t -gt $threshold) { $kept.Add($t) } }
    $script:LauncherRate[$key] = $kept
    if ($kept.Count -ge $script:LauncherRateLimitPerMinute) {
        return $false
    }
    $kept.Add($now)
    return $true
}

function Get-LauncherAuditDir {
    return (Join-Path $RepoRoot "reports/tooling/launcher_audit")
}

function Write-LauncherAudit([string]$action, [int]$statusCode, [bool]$ok, $extra = $null) {
    try {
        $dir = Get-LauncherAuditDir
        if (-not (Test-Path $dir)) {
            New-Item -ItemType Directory -Path $dir -Force | Out-Null
        }
        $day = (Get-Date).ToString("yyyyMMdd")
        $path = Join-Path $dir ("launcher_audit_{0}.jsonl" -f $day)
        $entry = [ordered]@{
            ts_utc = [DateTime]::UtcNow.ToString("o")
            action = $action
            status_code = $statusCode
            ok = $ok
            pid = $PID
        }
        if ($extra) {
            foreach ($p in $extra.PSObject.Properties) {
                $entry[$p.Name] = $p.Value
            }
        }
        Add-Content -Path $path -Value ($entry | ConvertTo-Json -Compress) -Encoding UTF8
    } catch {}
}

function Cleanup-LauncherAudit {
    try {
        $dir = Get-LauncherAuditDir
        if (-not (Test-Path $dir)) { return }
        $threshold = [DateTime]::UtcNow.AddDays(-1 * $script:LauncherAuditRetentionDays)
        Get-ChildItem -Path $dir -File -Filter "launcher_audit_*.jsonl" -ErrorAction SilentlyContinue |
            Where-Object { $_.LastWriteTimeUtc -lt $threshold } |
            Remove-Item -Force -ErrorAction SilentlyContinue
    } catch {}
}

function Get-LauncherAuditTail([int]$Tail = 40) {
    $dir = Get-LauncherAuditDir
    if (-not (Test-Path $dir)) { return @() }
    $files = Get-ChildItem -Path $dir -File -Filter "launcher_audit_*.jsonl" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTimeUtc -Descending
    $lines = New-Object System.Collections.Generic.List[string]
    foreach ($f in $files) {
        $content = Get-Content -Path $f.FullName -ErrorAction SilentlyContinue
        foreach ($line in ($content | Select-Object -Last $Tail)) {
            if ($line) { $lines.Add([string]$line) }
        }
        if ($lines.Count -ge $Tail) { break }
    }
    $parsed = @()
    foreach ($line in ($lines | Select-Object -Last $Tail)) {
        try {
            $parsed += ($line | ConvertFrom-Json)
        } catch {}
    }
    return @($parsed | Sort-Object ts_utc -Descending)
}

function Test-AgentHealthy {
    try {
        $args = @{
            Method = "GET"
            Uri = "http://127.0.0.1:7401/health"
            TimeoutSec = 2
        }
        if ((Get-Command Invoke-WebRequest).Parameters.ContainsKey("UseBasicParsing")) {
            $args["UseBasicParsing"] = $true
        }
        $resp = Invoke-WebRequest @args
        return ([int]$resp.StatusCode -eq 200)
    } catch {
        return $false
    }
}

$rootFull = [IO.Path]::GetFullPath((Resolve-Path $RootDir).Path)
$repoFull = [IO.Path]::GetFullPath((Resolve-Path $RepoRoot).Path)
$RepoRoot = $repoFull

$cfgPath = Join-Path $RepoRoot "scripts/config/hypercore.defaults.json"
if (Test-Path $cfgPath) {
    try {
        $cfg = Get-Content -Raw -Path $cfgPath | ConvertFrom-Json
        if ((-not $LauncherToken) -and $cfg.PSObject.Properties.Name.Contains("ui") -and $cfg.ui.PSObject.Properties.Name.Contains("launcher_token")) {
            $LauncherToken = [string]$cfg.ui.launcher_token
        }
        if ($cfg.PSObject.Properties.Name.Contains("ui") -and $cfg.ui.PSObject.Properties.Name.Contains("launcher_rate_limit_per_minute")) {
            $script:LauncherRateLimitPerMinute = [Math]::Max(1, [int]$cfg.ui.launcher_rate_limit_per_minute)
        }
        if ($cfg.PSObject.Properties.Name.Contains("ui") -and $cfg.ui.PSObject.Properties.Name.Contains("launcher_audit_retention_days")) {
            $script:LauncherAuditRetentionDays = [Math]::Max(1, [int]$cfg.ui.launcher_audit_retention_days)
        }
    } catch {}
}

Cleanup-LauncherAudit

$listener = [System.Net.HttpListener]::new()
$listener.Prefixes.Add(("http://127.0.0.1:{0}/" -f $Port))
$listener.Start()
Write-Host ("[dashboard-ui-server] listening http://127.0.0.1:{0}" -f $Port) -ForegroundColor DarkGray

try {
    while ($listener.IsListening) {
        $ctx = $listener.GetContext()
        try {
            $path = [string]$ctx.Request.Url.AbsolutePath
            $method = [string]$ctx.Request.HttpMethod

            if ($method -eq "GET" -and $path -eq "/health") {
                Write-Json $ctx 200 @{
                    ok = $true
                    server = "dashboard_ui_server"
                    now_utc = [DateTime]::UtcNow.ToString("o")
                    root = $rootFull
                }
                continue
            }

            if ($method -eq "POST" -and $path -eq "/api/launcher/start-agent") {
                if (-not (Is-LauncherAuthorized $ctx)) {
                    Write-LauncherAudit -action "start-agent" -statusCode 401 -ok $false -extra ([pscustomobject]@{ error = "unauthorized_launcher" })
                    Write-Json $ctx 401 @{ ok = $false; error = "unauthorized_launcher" }
                    continue
                }
                if (-not (Test-LauncherRateLimit $ctx)) {
                    Write-LauncherAudit -action "start-agent" -statusCode 429 -ok $false -extra ([pscustomobject]@{ error = "rate_limited" })
                    Write-Json $ctx 429 @{ ok = $false; error = "rate_limited" }
                    continue
                }
                $body = ""
                try {
                    $reader = New-Object IO.StreamReader($ctx.Request.InputStream, $ctx.Request.ContentEncoding)
                    $body = $reader.ReadToEnd()
                    $reader.Dispose()
                } catch {}
                $noSafe = $false
                if ($body) {
                    try {
                        $payload = $body | ConvertFrom-Json
                        $noSafe = [bool]$payload.no_safe
                    } catch {}
                }
                $alreadyHealthy = Test-AgentHealthy
                if (-not $noSafe -and $alreadyHealthy) {
                    Write-LauncherAudit -action "start-agent" -statusCode 200 -ok $true -extra ([pscustomobject]@{ already_running = $true; no_safe = $false })
                    Write-Json $ctx 200 @{
                        ok = $true
                        started = $false
                        already_running = $true
                        no_safe = $false
                        message = "agent already healthy"
                    }
                } else {
                    Start-AgentFromLauncher -noSafe:$noSafe
                    Write-LauncherAudit -action "start-agent" -statusCode 202 -ok $true -extra ([pscustomobject]@{ started = $true; no_safe = $noSafe })
                    Write-Json $ctx 202 @{
                        ok = $true
                        started = $true
                        no_safe = $noSafe
                        message = "launcher triggered"
                    }
                }
                continue
            }

            if ($method -eq "POST" -and $path -eq "/api/launcher/stop-agent") {
                if (-not (Is-LauncherAuthorized $ctx)) {
                    Write-LauncherAudit -action "stop-agent" -statusCode 401 -ok $false -extra ([pscustomobject]@{ error = "unauthorized_launcher" })
                    Write-Json $ctx 401 @{ ok = $false; error = "unauthorized_launcher" }
                    continue
                }
                if (-not (Test-LauncherRateLimit $ctx)) {
                    Write-LauncherAudit -action "stop-agent" -statusCode 429 -ok $false -extra ([pscustomobject]@{ error = "rate_limited" })
                    Write-Json $ctx 429 @{ ok = $false; error = "rate_limited" }
                    continue
                }
                $st = Stop-AgentFromLauncher
                Write-LauncherAudit -action "stop-agent" -statusCode 200 -ok $true -extra $st
                Write-Json $ctx 200 @{
                    ok = $true
                    result = $st
                    now_utc = [DateTime]::UtcNow.ToString("o")
                }
                continue
            }

            if ($method -eq "POST" -and $path -eq "/api/launcher/restart-agent") {
                if (-not (Is-LauncherAuthorized $ctx)) {
                    Write-LauncherAudit -action "restart-agent" -statusCode 401 -ok $false -extra ([pscustomobject]@{ error = "unauthorized_launcher" })
                    Write-Json $ctx 401 @{ ok = $false; error = "unauthorized_launcher" }
                    continue
                }
                if (-not (Test-LauncherRateLimit $ctx)) {
                    Write-LauncherAudit -action "restart-agent" -statusCode 429 -ok $false -extra ([pscustomobject]@{ error = "rate_limited" })
                    Write-Json $ctx 429 @{ ok = $false; error = "rate_limited" }
                    continue
                }
                $body = ""
                try {
                    $reader = New-Object IO.StreamReader($ctx.Request.InputStream, $ctx.Request.ContentEncoding)
                    $body = $reader.ReadToEnd()
                    $reader.Dispose()
                } catch {}
                $noSafe = $false
                if ($body) {
                    try {
                        $payload = $body | ConvertFrom-Json
                        $noSafe = [bool]$payload.no_safe
                    } catch {}
                }
                $st = Stop-AgentFromLauncher
                Start-AgentFromLauncher -noSafe:$noSafe
                Write-LauncherAudit -action "restart-agent" -statusCode 202 -ok $true -extra ([pscustomobject]@{ no_safe = $noSafe; stop_result = $st })
                Write-Json $ctx 202 @{
                    ok = $true
                    restarted = $true
                    stop_result = $st
                    no_safe = $noSafe
                }
                continue
            }

            if ($method -eq "GET" -and $path -eq "/api/launcher/agent-status") {
                $meta = Get-AgentRuntimeMeta
                Write-Json $ctx 200 @{
                    ok = $true
                    healthy = [bool](Test-AgentHealthy)
                    runtime = $meta
                    auth_required = [bool]([string]$LauncherToken)
                    rate_limit_per_minute = $script:LauncherRateLimitPerMinute
                    now_utc = [DateTime]::UtcNow.ToString("o")
                }
                continue
            }

            if ($method -eq "GET" -and $path -eq "/api/launcher/audit") {
                $tail = 40
                try {
                    $rawTail = [string]$ctx.Request.QueryString["tail"]
                    if ($rawTail) { $tail = [Math]::Max(1, [Math]::Min(200, [int]$rawTail)) }
                } catch {}
                $rows = Get-LauncherAuditTail -Tail $tail
                Write-Json $ctx 200 @{
                    ok = $true
                    rows = $rows
                    retention_days = $script:LauncherAuditRetentionDays
                }
                continue
            }

            if ($method -eq "OPTIONS") {
                $ctx.Response.StatusCode = 204
                $ctx.Response.OutputStream.Close()
                continue
            }

            if ($method -eq "GET") {
                Serve-File $ctx $rootFull $path
                continue
            }

            Write-Json $ctx 405 @{ ok = $false; error = "method_not_allowed" }
        } catch {
            try {
                Write-LauncherAudit -action "server-error" -statusCode 500 -ok $false -extra ([pscustomobject]@{ error = [string]$_.Exception.Message })
                Write-Json $ctx 500 @{ ok = $false; error = "server_error"; detail = [string]$_.Exception.Message }
            } catch {}
        }
    }
} finally {
    if ($listener.IsListening) {
        $listener.Stop()
    }
    $listener.Close()
}

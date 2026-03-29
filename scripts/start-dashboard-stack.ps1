param(
    [switch]$NoAgent,
    [switch]$NoDashboard,
    [switch]$UnsafeNoAuth,
    [switch]$DryRun,
    [int]$DashboardPort = 5173
)

$ErrorActionPreference = 'Stop'

function Write-Info([string]$Message) {
    Write-Host "[stack] $Message" -ForegroundColor Cyan
}

function Write-Warn([string]$Message) {
    Write-Host "[stack] $Message" -ForegroundColor Yellow
}

function Test-AgentReady {
    param([int]$TimeoutSec = 2)

    foreach ($uri in @('http://127.0.0.1:7401/health', 'http://127.0.0.1:7401/api/health')) {
        try {
            $resp = Invoke-WebRequest -Uri $uri -UseBasicParsing -TimeoutSec $TimeoutSec
            if ($resp.StatusCode -ge 200 -and $resp.StatusCode -lt 500) {
                return $true
            }
        } catch {
            continue
        }
    }
    return $false
}

function Wait-AgentReady {
    param([int]$TimeoutSec = 30)

    $start = Get-Date
    while ((Get-Date) -lt $start.AddSeconds($TimeoutSec)) {
        if (Test-AgentReady -TimeoutSec 2) {
            return $true
        }
        Start-Sleep -Milliseconds 600
    }
    return $false
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$dashboardDir = Join-Path $repoRoot 'dashboard'
$agentScript = Join-Path $PSScriptRoot 'dashboard_agent.ps1'

if (-not (Test-Path $dashboardDir)) {
    throw "Dashboard directory not found: $dashboardDir"
}

if (-not $NoAgent) {
    if (Test-AgentReady -TimeoutSec 2) {
        Write-Info 'Agent already reachable on http://127.0.0.1:7401'
    } else {
        if (-not (Test-Path $agentScript)) {
            throw "Agent script not found: $agentScript"
        }

        $args = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $agentScript)
        if ($UnsafeNoAuth) {
            $args += '-NoSafe'
        }

        Write-Info 'Starting dashboard agent in detached PowerShell window...'
        if (-not $DryRun) {
            Start-Process -FilePath 'pwsh' -ArgumentList $args -WorkingDirectory $repoRoot | Out-Null
            if (Wait-AgentReady -TimeoutSec 35) {
                Write-Info 'Agent is ready.'
            } else {
                Write-Warn 'Agent did not become ready within timeout. Dashboard will still be started.'
            }
        }
    }
}

if (-not $NoDashboard) {
    Write-Info "Starting dashboard dev server on http://127.0.0.1:$DashboardPort"
    if (-not $DryRun) {
        Set-Location $dashboardDir
        npm run i18n:compile:deno
        deno run -A npm:vite dev --host 127.0.0.1 --port $DashboardPort
    }
}

if ($DryRun) {
    Write-Info 'DryRun complete. No processes were started.'
}

function Get-CommandRegistry {
    $path = Join-Path $PSScriptRoot "..\config\hypercore.commands.json"
    if (-not (Test-Path $path)) { return @() }
    return (Get-Content -Raw -Path $path | ConvertFrom-Json)
}

function Run-DoctorFix {
    param($Cfg, $Settings)
    Run-Doctor -Settings $Settings
    $report = New-DoctorReport -Settings $Settings
    if (-not [bool]$report.ok) {
        $missing = @()
        foreach ($kv in $report.checks.PSObject.Properties) {
            if (-not [bool]$kv.Value) { $missing += [string]$kv.Name }
        }
        Write-HcStep "hypercore:$Command" (Get-Msg "doctor_fix_missing" @($missing -join ", "))
        $approved = [bool]$AutoApprove
        if (-not $approved -and -not $DryRun) {
            $answer = Read-Host (Get-Msg "doctor_fix_prompt")
            $approved = ($answer -match "^(y|yes|e|evet)$")
        }
        if (-not $approved) {
            Write-HcStep "hypercore:$Command" (Get-Msg "doctor_fix_skipped")
            return
        }
        Write-HcStep "hypercore:$Command" (Get-Msg "doctor_fix_installing")
        Run-Install -Cfg $Cfg -Settings $Settings
        Run-Doctor -Settings $Settings
    }
}

function Run-Bootstrap {
    param($Cfg, $Settings)
    Run-DoctorFix -Cfg $Cfg -Settings $Settings
    Run-BuildIso
    Run-QemuSmoke -Settings $Settings
    Run-Triage -Settings $Settings
}

function Run-FirstRun {
    param($Cfg, $Settings)
    Write-Host (Get-Msg "firstrun_banner")
    Run-Bootstrap -Cfg $Cfg -Settings $Settings
}

function Run-Help {
    $registry = @(Get-CommandRegistry)
    if ($registry.Count -eq 0) {
        Write-Host (Get-Msg "help_title")
        return
    }
    Write-Host (Get-Msg "help_title")
    foreach ($c in $registry) {
        $desc = if ($Lang -eq "tr") { [string]$c.desc_tr } else { [string]$c.desc_en }
        Write-Host ("- {0}: {1}" -f [string]$c.name, $desc)
    }
}

function Run-OpenReport {
    # Ensure the Svelte UI root always has dashboard_data.json before serving.
    $uiDashboardDir = "reports/tooling/dashboard_ui"
    $uiDashboardDataPath = Join-Path $uiDashboardDir "dashboard_data.json"
    if (-not (Test-Path $uiDashboardDir)) {
        New-Item -ItemType Directory -Force -Path $uiDashboardDir | Out-Null
    }
    if (-not (Test-Path $uiDashboardDataPath)) {
        if (Test-Path "reports/tooling/dashboard_data.json") {
            Copy-Item -Force -Path "reports/tooling/dashboard_data.json" -Destination $uiDashboardDataPath
        } elseif (Test-Path "dashboard-ui/src/generated/dashboard_data.json") {
            Copy-Item -Force -Path "dashboard-ui/src/generated/dashboard_data.json" -Destination $uiDashboardDataPath
        } elseif ($script:Settings -and (Get-Command Run-ExportDashboardData -ErrorAction SilentlyContinue)) {
            try {
                Run-ExportDashboardData -Settings $script:Settings
                if (Test-Path "reports/tooling/dashboard_data.json") {
                    Copy-Item -Force -Path "reports/tooling/dashboard_data.json" -Destination $uiDashboardDataPath
                }
            } catch {}
        }
    }

    $candidateOrder = @()
    if ($ReportTarget -eq "ui") {
        $candidateOrder = @(
            "reports/tooling/dashboard_ui/index.html",
            "reports/tooling/dashboard.html",
            $DashboardPath,
            $TriagePath,
            "reports/tooling/health_report.json"
        )
    } elseif ($ReportTarget -eq "html") {
        $candidateOrder = @(
            "reports/tooling/dashboard.html",
            "reports/tooling/dashboard_ui/index.html",
            $DashboardPath,
            $TriagePath,
            "reports/tooling/health_report.json"
        )
    } else {
        $candidateOrder = @(
            "reports/tooling/dashboard_ui/index.html",
            "reports/tooling/dashboard.html",
            $DashboardPath,
            $TriagePath,
            "reports/tooling/health_report.json"
        )
    }
    $candidate = @($candidateOrder | Where-Object { Test-Path $_ } | Select-Object -First 1)[0]
    if (-not $candidate) {
        Fail-Hc -Code "dependency_missing" -Message "no report found to open"
    }
    if ($DryRun) {
        Write-HcStep "hypercore:$Command" (Get-Msg "open_report_dry_run" @($candidate))
        return
    }
    $normalized = [string]$candidate -replace "\\","/"
    if ($normalized -like "*reports/tooling/dashboard_ui/index.html") {
        $uiDir = Resolve-Path "reports/tooling/dashboard_ui"
        $pidPath = Join-Path $uiDir "server.pid"
        $portPath = Join-Path $uiDir "server.port"
        $port = $null
        $reuse = $false

        if ((Test-Path $pidPath) -and (Test-Path $portPath)) {
            try {
                $existingPid = [int](Get-Content -Raw -Path $pidPath).Trim()
                $existingPort = [int](Get-Content -Raw -Path $portPath).Trim()
                $proc = Get-Process -Id $existingPid -ErrorAction Stop
                if ($proc -and -not $proc.HasExited) {
                    $probeOk = $false
                    try {
                        $probeArgs = @{
                            Method = "GET"
                            Uri = ("http://127.0.0.1:{0}/index.html" -f $existingPort)
                            TimeoutSec = 2
                        }
                        if ((Get-Command Invoke-WebRequest).Parameters.ContainsKey("UseBasicParsing")) {
                            $probeArgs["UseBasicParsing"] = $true
                        }
                        $probe = Invoke-WebRequest @probeArgs
                        if ([int]$probe.StatusCode -eq 200) { $probeOk = $true }
                    } catch {}
                    if ($probeOk) {
                        $port = $existingPort
                        $reuse = $true
                    }
                }
            } catch {}
        }

        if (-not $port) {
            $listener = New-Object System.Net.Sockets.TcpListener([System.Net.IPAddress]::Loopback, 0)
            $listener.Start()
            $port = ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port
            $listener.Stop()

            $serverScript = Resolve-Path (Join-Path $PSScriptRoot "..\dashboard_ui_server.ps1")
            $repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
            $shell = if ($script:ShellExe) { $script:ShellExe } else { "powershell" }
            $argList = @(
                "-NoProfile", "-ExecutionPolicy", "Bypass",
                "-File", [string]$serverScript,
                "-Port", "$port",
                "-RootDir", [string]$uiDir,
                "-RepoRoot", [string]$repoRoot
            )
            $proc = Start-Process -FilePath $shell -ArgumentList $argList -PassThru -WindowStyle Hidden
            $ready = $false
            for ($i = 0; $i -lt 25; $i++) {
                Start-Sleep -Milliseconds 120
                try {
                    $probeArgs = @{
                        Method = "GET"
                        Uri = ("http://127.0.0.1:{0}/index.html" -f $port)
                        TimeoutSec = 2
                    }
                    if ((Get-Command Invoke-WebRequest).Parameters.ContainsKey("UseBasicParsing")) {
                        $probeArgs["UseBasicParsing"] = $true
                    }
                    $probe = Invoke-WebRequest @probeArgs
                    if ([int]$probe.StatusCode -eq 200) { $ready = $true; break }
                } catch {}
            }
            if ($ready) {
                Set-Content -Path $pidPath -Value ([string]$proc.Id) -Encoding ascii
                Set-Content -Path $portPath -Value ([string]$port) -Encoding ascii
                Write-HcStep "hypercore:$Command" (Get-Msg "open_report_server_started" @("http://127.0.0.1:$port"))
            } else {
                try { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue } catch {}
                $port = $null
            }
        }

        if ($port) {
            $url = "http://127.0.0.1:$port/index.html"
            if ($reuse) {
                Write-HcStep "hypercore:$Command" (Get-Msg "open_report_server_reused" @($url))
            }
            Start-Process $url
            Write-HcStep "hypercore:$Command" (Get-Msg "open_report_opened" @($url))
            return
        }

        if (Test-Path "reports/tooling/dashboard.html") {
            Write-HcStep "hypercore:$Command" (Get-Msg "open_report_ui_server_failed_fallback_html")
            Start-Process "reports/tooling/dashboard.html"
            Write-HcStep "hypercore:$Command" (Get-Msg "open_report_opened" @("reports/tooling/dashboard.html"))
            return
        }
    }

    Start-Process $candidate
    Write-HcStep "hypercore:$Command" (Get-Msg "open_report_opened" @($candidate))
}

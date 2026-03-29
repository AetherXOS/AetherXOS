<#
.SYNOPSIS
HyperCore Quick Launcher: simple, guided CLI/GUI for non-technical users.
#>

param(
    [ValidateSet("menu", "wizard", "gui", "environment-audit", "preflight", "environment-repair", "setup", "dashboard-workspace-setup", "setup-dashboard", "agent-service-start", "start-dashboard-agent", "agent-service-start-unsafe", "start-dashboard-agent-nosafe", "agent-contract-verification", "agent-contract", "guided-bootstrap", "smart-run", "release-readiness", "go-live", "workspace-bootstrap", "all-in-one", "build-and-smoke", "build-os", "build-boot-iso", "create-iso", "emulator-smoke", "run-qemu", "emulator-interactive", "run-qemu-live", "quality-gate", "full-check", "dashboard-build", "create-dashboard", "dashboard-validation", "test-dashboard", "dashboard-open", "open-dashboard", "help")]
    [string]$Action = "menu",
    [ValidateSet("quick", "strict")]
    [string]$Profile = "quick",
    [ValidateSet("auto", "ui", "html")]
    [string]$DashboardTarget = "auto",
    [ValidateSet("auto", "en", "tr")]
    [string]$Lang = "auto",
    [switch]$NoLock,
    [switch]$SkipConfirm,
    [switch]$NonInteractive,
    [int]$MaxStepRetry = 1
)

$ErrorActionPreference = "Stop"
try {
    [Console]::InputEncoding = [System.Text.UTF8Encoding]::new($false)
    [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
    & chcp 65001 *> $null
} catch {}

$script:Root = $PSScriptRoot
$script:HypercorePath = Join-Path $script:Root "hypercore.ps1"
$script:ActionConfigPath = Join-Path $script:Root "config/quick.actions.json"
$script:QuickRunsDir = Join-Path $script:Root "..\reports\tooling\quick_runs"
$script:RunStarted = [DateTime]::UtcNow
$script:RunLogPath = ""
$script:LocaleCatalog = @{}
$script:EffectiveLang = "en"
$script:ForceNoConfirm = $false
$script:ShellExe = ""

function Resolve-QuickShell {
    if (Get-Command "pwsh" -ErrorAction SilentlyContinue) { return "pwsh" }
    return "powershell"
}

function Resolve-QuickLang {
    if ($Lang -ne "auto") { return $Lang }
    try {
        $culture = [System.Globalization.CultureInfo]::CurrentUICulture.Name
        if ($culture -match "^tr") { return "tr" }
    } catch {}
    return "en"
}

function Initialize-QuickLocale {
    $script:EffectiveLang = Resolve-QuickLang
    $path = Join-Path $script:Root ("i18n/{0}.json" -f $script:EffectiveLang)
    if (-not (Test-Path $path)) {
        $script:EffectiveLang = "en"
        $path = Join-Path $script:Root "i18n/en.json"
    }
    if (Test-Path $path) {
        $obj = Get-Content -Raw -Path $path -Encoding UTF8 | ConvertFrom-Json
        $map = @{}
        foreach ($p in $obj.PSObject.Properties) {
            $map[[string]$p.Name] = [string]$p.Value
        }
        $script:LocaleCatalog = $map
    }
}

function T {
    param(
        [string]$Key,
        [string]$Default,
        [Parameter(ValueFromRemainingArguments = $true)]
        [object[]]$FmtArgs = @()
    )
    $msg = if ($script:LocaleCatalog.ContainsKey($Key)) { [string]$script:LocaleCatalog[$Key] } else { $Default }
    if ($FmtArgs.Count -eq 1 -and $FmtArgs[0] -is [System.Array]) {
        $FmtArgs = @($FmtArgs[0])
    }
    if ($FmtArgs.Count -gt 0) {
        return ($msg -f $FmtArgs)
    }
    return $msg
}

function Get-ActionText {
    param($Item, [string]$Field)
    $localized = "{0}_{1}" -f $Field, $script:EffectiveLang
    if ($Item.PSObject.Properties.Name.Contains($localized)) {
        return [string]$Item.$localized
    }
    return [string]$Item.$Field
}

function Get-ActionRisk {
    param($Item)
    if ($Item.PSObject.Properties.Name.Contains("risk_level")) {
        return [string]$Item.risk_level
    }
    return "INFO"
}

function Get-ActionCategory {
    param($Item)
    if ($Item.PSObject.Properties.Name.Contains("category")) {
        return [string]$Item.category
    }
    return "misc"
}

function Get-RiskRank {
    param([string]$Risk)
    switch ($Risk) {
        "HIGH" { return 0 }
        "MED" { return 1 }
        default { return 2 }
    }
}

function Get-GroupRank {
    param([string]$GroupName)
    switch ($GroupName) {
        "Recommended Flows" { return 0 }
        "Önerilen Akışlar" { return 0 }
        "Workspace Setup" { return 1 }
        "Ortam Kurulumu" { return 1 }
        "Build and Validation" { return 2 }
        "Derleme ve Doğrulama" { return 2 }
        "Dashboard Operations" { return 3 }
        "Dashboard İşlemleri" { return 3 }
        "Validation" { return 4 }
        "Doğrulama" { return 4 }
        default { return 9 }
    }
}

function Get-RiskUiColors {
    param([string]$Risk)
    switch ($Risk) {
        "HIGH" { return @{ Foreground = "White"; Background = [System.Drawing.Color]::FromArgb(176, 54, 54) } }
        "MED" { return @{ Foreground = "Black"; Background = [System.Drawing.Color]::FromArgb(240, 188, 74) } }
        default { return @{ Foreground = "White"; Background = [System.Drawing.Color]::FromArgb(52, 108, 196) } }
    }
}

function Write-QuickBanner {
    try {
        Clear-Host
    } catch {}
    Write-Host "============================================================" -ForegroundColor DarkCyan
    Write-Host (" {0}" -f (T "quick_banner_title" "HyperCore Quick Launcher")) -ForegroundColor Cyan
    Write-Host (" {0}" -f (T "quick_banner_subtitle" "Beginner-friendly mode for setup + build + ISO + QEMU + Dashboard")) -ForegroundColor Gray
    Write-Host "============================================================" -ForegroundColor DarkCyan
    Write-Host ((T "quick_banner_meta" "Profile: {0} | NoLock: {1} | Retry: {2} | Lang: {3}" @($Profile, [bool]$NoLock, $MaxStepRetry, $script:EffectiveLang))) -ForegroundColor DarkGray
    Write-Host ((T "quick_banner_target" "Dashboard target: {0}" @($DashboardTarget))) -ForegroundColor DarkGray
    Write-Host ""
}

function Write-QuickStatus {
    param(
        [string]$Text,
        [ValidateSet("info", "ok", "warn", "error")]
        [string]$Level = "info"
    )
    $color = switch ($Level) {
        "ok" { "Green" }
        "warn" { "Yellow" }
        "error" { "Red" }
        default { "Cyan" }
    }
    $tag = [string]::Concat(($Level.Substring(0, 1).ToUpperInvariant()), ($Level.Substring(1).ToLowerInvariant()))
    Write-Host ("[{0}] {1}" -f $tag, $Text) -ForegroundColor $color
}

function Initialize-RunLog {
    New-Item -ItemType Directory -Path $script:QuickRunsDir -Force | Out-Null
    $stamp = (Get-Date).ToString("yyyyMMdd_HHmmss")
    $script:RunLogPath = Join-Path $script:QuickRunsDir ("quick_run_{0}.json" -f $stamp)
}

function Save-RunLog {
    param(
        [string]$ActionName,
        [array]$Results,
        [string]$Status
    )
    if (-not $script:RunLogPath) { return }
    $payload = [ordered]@{
        action = $ActionName
        profile = $Profile
        lang = $script:EffectiveLang
        no_lock = [bool]$NoLock
        started_utc = $script:RunStarted.ToString("o")
        finished_utc = [DateTime]::UtcNow.ToString("o")
        status = $Status
        results = $Results
    }
    $payload | ConvertTo-Json -Depth 6 | Set-Content -Path $script:RunLogPath -Encoding UTF8
}

function Get-ActionCatalog {
    if (-not (Test-Path $script:ActionConfigPath)) {
        throw "quick action config missing: $script:ActionConfigPath"
    }
    $raw = Get-Content -Raw -Path $script:ActionConfigPath -Encoding UTF8
    $cfg = $raw | ConvertFrom-Json
    if (-not $cfg) { throw "quick action config is empty" }
    return @($cfg)
}

function Invoke-Hypercore {
    param(
        [Parameter(Mandatory = $true)][string]$Command,
        [string[]]$ExtraArgs = @()
    )
    $args = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $script:HypercorePath, "-Command", $Command, "-Profile", $Profile, "-Lang", $script:EffectiveLang)
    if ($NoLock) { $args += "-NoLock" }
    if ($ExtraArgs.Count -gt 0) { $args += $ExtraArgs }
    & $script:ShellExe @args
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed: $Command (exit=$LASTEXITCODE)"
    }
}

function Invoke-FailureAssist {
    try {
        Invoke-Hypercore -Command "explain-last-failure"
    } catch {
        Write-QuickStatus -Text (T "quick_failure_assist_unavailable" "Failure assistant could not read the last run details.") -Level "warn"
    }
}

function Get-DoctorReportPath {
    return Join-Path $script:Root "..\reports\tooling\doctor_report.json"
}

function Show-PreflightDetails {
    $reportPath = Get-DoctorReportPath
    if (-not (Test-Path $reportPath)) {
        Write-QuickStatus -Text (T "quick_preflight_report_missing" "Doctor report not found yet.") -Level "warn"
        return [pscustomobject]@{ ok = $false; missing = @("doctor_report") }
    }
    try {
        $report = Get-Content -Raw -Path $reportPath -Encoding UTF8 | ConvertFrom-Json
        $missing = @()
        foreach ($p in $report.checks.PSObject.Properties) {
            if (-not [bool]$p.Value) { $missing += [string]$p.Name }
        }
        Write-Host ""
        Write-Host (T "quick_preflight_summary_title" "Preflight Summary:") -ForegroundColor Cyan
        $statusText = if ([bool]$report.ok) { T "quick_preflight_ready" "READY" } else { T "quick_preflight_missing" "MISSING_DEPENDENCIES" }
        $missingText = if ($missing.Count -eq 0) { T "quick_none" "none" } else { $missing -join ", " }
        Write-Host ((T "quick_preflight_status" "  Status : {0}" @($statusText)))
        Write-Host ((T "quick_preflight_missing_line" "  Missing: {0}" @($missingText)))
        return [pscustomobject]@{ ok = [bool]$report.ok; missing = $missing }
    } catch {
        Write-QuickStatus -Text (T "quick_preflight_parse_error" "Unable to parse doctor report.") -Level "warn"
        return [pscustomobject]@{ ok = $false; missing = @("parse_error") }
    }
}

function Maybe-RunSetupAfterPreflight {
    param($Preflight)
    if ($Preflight.ok) { return }
    if ($NonInteractive -or $SkipConfirm) {
        Write-QuickStatus -Text (T "quick_fix_now_auto" "Preflight not ready. Auto-running setup.") -Level "warn"
        $script:ForceNoConfirm = $true
        try { Run-Action -Name "setup" } finally { $script:ForceNoConfirm = $false }
        return
    }
    $ans = Read-Host (T "quick_fix_now_prompt" "Missing dependencies detected. Run Setup now? [Y/n]")
    if ($ans -eq "" -or $ans -match "^(y|yes|e|evet)$") {
        $script:ForceNoConfirm = $true
        try { Run-Action -Name "setup" } finally { $script:ForceNoConfirm = $false }
    } else {
        Write-QuickStatus -Text (T "quick_fix_now_skipped" "Setup skipped by user.") -Level "warn"
    }
}

function Split-RawCommand {
    param([string]$Raw)
    $parts = @($Raw -split "\s+")
    $cmd = $parts[0]
    $extra = @()
    if ($parts.Count -gt 1) {
        $extra = @($parts | Select-Object -Skip 1)
    }
    return [pscustomobject]@{ command = $cmd; extra = $extra }
}

function Resolve-OpenDashboardSteps {
    $uiPath = Join-Path $script:Root "..\reports\tooling\dashboard_ui\index.html"
    $htmlPath = Join-Path $script:Root "..\reports\tooling\dashboard.html"
    $steps = New-Object System.Collections.Generic.List[string]

    if ($DashboardTarget -eq "ui") {
        if (-not (Test-Path $uiPath)) {
            $steps.Add("dashboard")
            $steps.Add("html-dashboard")
            $steps.Add("dashboard-ui-build")
        }
    } elseif ($DashboardTarget -eq "html") {
        if (-not (Test-Path $htmlPath)) {
            $steps.Add("dashboard")
            $steps.Add("html-dashboard")
        }
    } else {
        if (-not (Test-Path $uiPath)) {
            $steps.Add("dashboard")
            $steps.Add("html-dashboard")
            $steps.Add("dashboard-ui-build")
        } elseif (-not (Test-Path $htmlPath)) {
            $steps.Add("html-dashboard")
        }
    }
    if ($DashboardTarget -ne "html" -and -not (Test-LocalDashboardAgentHealthy)) {
        $steps.Add("dashboard-agent-bg")
    }
    $steps.Add("open-report")
    return @($steps.ToArray())
}

function Get-OpenReportTargetArg {
    switch ($DashboardTarget) {
        "ui" { return "ui" }
        "html" { return "html" }
        default { return "auto" }
    }
}

function Test-AgentEndpoint {
    param([string]$Uri)
    try {
        $resp = Invoke-WebRequest -Uri $Uri -Method GET -TimeoutSec 2 -UseBasicParsing -ErrorAction Stop
        return ([int]$resp.StatusCode -eq 200)
    } catch {
        return $false
    }
}

function Test-LocalDashboardAgentHealthy {
    foreach ($uri in @("http://127.0.0.1:7401/health", "http://127.0.0.1:7401/api/health")) {
        if (Test-AgentEndpoint -Uri $uri) {
            return $true
        }
    }
    return $false
}

function Normalize-StepList {
    param($InputSteps)
    if ($null -eq $InputSteps) { return @() }
    if ($InputSteps -is [string]) { return @([string]$InputSteps) }
    if ($InputSteps -is [System.Collections.IEnumerable]) {
        $normalized = New-Object System.Collections.Generic.List[string]
        foreach ($s in $InputSteps) {
            if ($null -eq $s) { continue }
            $text = [string]$s
            if (-not [string]::IsNullOrWhiteSpace($text)) {
                $normalized.Add($text.Trim())
            }
        }
        return @($normalized.ToArray())
    }
    return @([string]$InputSteps)
}

function Invoke-QuickStep {
    param(
        [Parameter(Mandatory = $true)][string]$Raw,
        [int]$Index = 1,
        [int]$Total = 1,
        [string[]]$ExtraArgsAppend = @()
    )
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $parsed = Split-RawCommand -Raw $Raw
    $attempt = 0
    $maxAttempts = [Math]::Max(1, $MaxStepRetry)
    while ($attempt -lt $maxAttempts) {
        $attempt += 1
        Write-Progress -Activity (T "quick_progress_activity" "HyperCore Quick Run") -Status (T "quick_progress_status" "Step {0}/{1}: {2} (attempt {3}/{4})" @($Index, $Total, $Raw, $attempt, $maxAttempts)) -PercentComplete ([int](($Index - 1) * 100 / [Math]::Max(1, $Total)))
        Write-QuickStatus -Text ((T "quick_step_running" "[{0}/{1}] Running: {2} (attempt {3}/{4})" @($Index, $Total, $Raw, $attempt, $maxAttempts))) -Level "info"
        try {
            $mergedExtra = @([string[]]$parsed.extra + [string[]]$ExtraArgsAppend)
            Invoke-Hypercore -Command ([string]$parsed.command) -ExtraArgs $mergedExtra
            $sw.Stop()
            Write-QuickStatus -Text ((T "quick_step_done" "[{0}/{1}] Done: {2} ({3} ms)" @($Index, $Total, $Raw, [int]$sw.Elapsed.TotalMilliseconds))) -Level "ok"
            Write-Progress -Activity (T "quick_progress_activity" "HyperCore Quick Run") -Completed
            return [pscustomobject]@{ step = $Raw; ok = $true; duration_ms = [int]$sw.Elapsed.TotalMilliseconds; attempts = $attempt; error = "" }
        } catch {
            if ($attempt -ge $maxAttempts) {
                $sw.Stop()
                Write-QuickStatus -Text ((T "quick_step_failed" "[{0}/{1}] Failed: {2}" @($Index, $Total, $Raw))) -Level "error"
                Invoke-FailureAssist
                Write-Progress -Activity (T "quick_progress_activity" "HyperCore Quick Run") -Completed
                return [pscustomobject]@{ step = $Raw; ok = $false; duration_ms = [int]$sw.Elapsed.TotalMilliseconds; attempts = $attempt; error = [string]$_.Exception.Message }
            }
            Write-QuickStatus -Text ((T "quick_retrying_step" "Retrying step due to error: {0}" @([string]$_.Exception.Message))) -Level "warn"
            Start-Sleep -Seconds 1
        }
    }
}

function Write-RunSummary {
    param(
        [string]$ActionTitle,
        [array]$Results
    )
    $okCount = @($Results | Where-Object { [bool]$_.ok }).Count
    $total = @($Results).Count
    $totalMs = 0
    foreach ($r in @($Results)) {
        $totalMs += [int]$r.duration_ms
    }
    Write-Host ""
    Write-Host "+------------------ RUN SUMMARY ------------------+" -ForegroundColor DarkCyan
    Write-Host ((T "quick_summary_action" "| Action : {0}" @($ActionTitle)))
    Write-Host ((T "quick_summary_status" "| Status : {0}/{1} steps passed" @($okCount, $total)))
    Write-Host ((T "quick_summary_time" "| Time   : {0} ms" @($totalMs)))
    foreach ($r in $Results) {
        $mark = if ([bool]$r.ok) { "OK " } else { "ERR" }
        Write-Host ((T "quick_summary_row" "| {0} | {1} ({2} ms, tries={3})" @($mark, [string]$r.step, [int]$r.duration_ms, [int]$r.attempts)))
    }
    Write-Host ((T "quick_summary_log" "| Log    : {0}" @($script:RunLogPath)))
    Write-Host "+-------------------------------------------------+" -ForegroundColor DarkCyan
}

function Confirm-Action {
    param($Item)
    if ($SkipConfirm -or $NonInteractive -or $script:ForceNoConfirm) { return $true }
    Write-Host ""
    Write-Host ((T "quick_confirm_selected" "You selected: {0}" @(Get-ActionText -Item $Item -Field "title"))) -ForegroundColor Cyan
    Write-Host ((T "quick_confirm_desc" "Description : {0}" @(Get-ActionText -Item $Item -Field "desc"))) -ForegroundColor DarkGray
    Write-Host (T "quick_confirm_steps" "Planned steps:") -ForegroundColor Cyan
    $i = 1
    foreach ($s in @($Item.cmds)) {
        Write-Host ("  {0}. {1}" -f $i, $s)
        $i += 1
    }
    $ans = Read-Host (T "quick_confirm_prompt" "Continue? [Y/n]")
    return ($ans -eq "" -or $ans -match "^(y|yes|e|evet)$")
}

function Get-ActionItem {
    param([string]$Key)
    return @(
        $script:ActionCatalog |
            Where-Object {
                $_.key -eq $Key -or (
                    $_.PSObject.Properties.Name.Contains("aliases") -and
                    @($_.aliases) -contains $Key
                )
            } |
            Select-Object -First 1
    )[0]
}

function Run-Action {
    param([string]$Name)
    if ($Name -eq "help") {
        Show-Help
        return
    }

    if ($Name -in @("smart-run", "guided-bootstrap")) {
        Write-QuickStatus -Text (T "quick_smart_run_info" "Guided Bootstrap: audit + auto-repair + smoke build pipeline") -Level "info"
    }

    $item = Get-ActionItem -Key $Name
    if (-not $item) {
        throw "Unknown action: $Name"
    }
    if (-not (Confirm-Action -Item $item)) {
        Write-QuickStatus -Text (T "quick_cancelled" "Action cancelled by user.") -Level "warn"
        return
    }

    Write-QuickStatus -Text ((T "quick_action_start" "Action: {0} - {1}" @((Get-ActionText -Item $item -Field "title"), (Get-ActionText -Item $item -Field "desc")))) -Level "info"
    $results = New-Object System.Collections.Generic.List[object]
    $steps = @(Normalize-StepList -InputSteps $item.cmds)
    if ($Name -in @("open-dashboard", "dashboard-open")) {
        $steps = @(Normalize-StepList -InputSteps (Resolve-OpenDashboardSteps))
    }
    for ($i = 0; $i -lt $steps.Count; $i++) {
        $extraArgs = @()
        if ($Name -in @("open-dashboard", "dashboard-open") -and [string]$steps[$i] -eq "open-report") {
            $extraArgs = @("-ReportTarget", (Get-OpenReportTargetArg))
        }
        $res = Invoke-QuickStep -Raw ([string]$steps[$i]) -Index ($i + 1) -Total $steps.Count -ExtraArgsAppend $extraArgs
        $results.Add($res)
        if (($Name -in @("preflight", "environment-audit") -or [string]$steps[$i] -like "doctor*") -and [bool]$res.ok) {
            $preflight = Show-PreflightDetails
            if ($Name -in @("preflight", "environment-audit")) {
                Maybe-RunSetupAfterPreflight -Preflight $preflight
            }
        }
        if (-not $res.ok) {
            Write-RunSummary -ActionTitle (Get-ActionText -Item $item -Field "title") -Results $results.ToArray()
            Save-RunLog -ActionName $Name -Results $results.ToArray() -Status "failed"
            throw $res.error
        }
    }
    Write-RunSummary -ActionTitle (Get-ActionText -Item $item -Field "title") -Results $results.ToArray()
    Save-RunLog -ActionName $Name -Results $results.ToArray() -Status "ok"
    Write-QuickStatus -Text ((T "quick_completed" "Completed action: {0}" @((Get-ActionText -Item $item -Field "title")))) -Level "ok"
}

function Show-Help {
    Write-QuickBanner
    Write-Host (T "quick_help_usage" "Usage examples:") -ForegroundColor Cyan
    Write-Host "  powershell -ExecutionPolicy Bypass -File scripts/quick.ps1 -Action menu"
    Write-Host "  powershell -ExecutionPolicy Bypass -File scripts/quick.ps1 -Action guided-bootstrap -NoLock -SkipConfirm -Lang tr"
    Write-Host "  powershell -ExecutionPolicy Bypass -File scripts/quick.ps1 -Action dashboard-open -DashboardTarget ui"
    Write-Host ""
    Write-Host (T "quick_help_actions" "Actions:") -ForegroundColor Cyan
    Write-Host "+------------------+----------------------+----------------------------------------+" -ForegroundColor DarkCyan
    Write-Host (T "quick_help_table_header" "| Key              | Group                | Description                            |")
    Write-Host "+------------------+----------------------+----------------------------------------+" -ForegroundColor DarkCyan
    foreach ($item in $script:ActionCatalog) {
        $k = ([string]$item.key).PadRight(16)
        $g = (Get-ActionText -Item $item -Field "group")
        $g = $g.PadRight(20)
        $d = Get-ActionText -Item $item -Field "desc"
        if ($d.Length -gt 38) { $d = $d.Substring(0, 38) }
        $d = $d.PadRight(38)
        Write-Host ("| {0} | {1} | {2} |" -f $k, $g, $d)
    }
    Write-Host "+------------------+----------------------+----------------------------------------+" -ForegroundColor DarkCyan
    Write-Host ""
    Write-Host (T "quick_help_modes" "Modes:") -ForegroundColor Cyan
    Write-Host ("  - wizard: {0}" -f (T "quick_help_wizard" "guided flow"))
    Write-Host ("  - gui: {0}" -f (T "quick_help_gui" "button-based Windows launcher"))
    Write-Host ("  - menu: {0}" -f (T "quick_help_menu" "grouped numbered menu"))
}

function Show-Menu {
    while ($true) {
        Write-QuickBanner
        $sortedActions = @(
            $script:ActionCatalog |
                Sort-Object `
                    @{ Expression = { Get-GroupRank -GroupName (Get-ActionText -Item $_ -Field "group") } }, `
                    @{ Expression = { Get-RiskRank -Risk (Get-ActionRisk -Item $_) } }, `
                    @{ Expression = { Get-ActionText -Item $_ -Field "title" } }
        )
        $groups = @($sortedActions | Group-Object { Get-ActionText -Item $_ -Field "group" })
        $flat = New-Object System.Collections.Generic.List[object]
        $n = 1
        foreach ($grp in $groups) {
            Write-Host ("[{0}]" -f [string]$grp.Name) -ForegroundColor DarkCyan
            foreach ($item in $grp.Group) {
                $risk = (Get-ActionRisk -Item $item).PadRight(4)
                $category = (Get-ActionCategory -Item $item)
                Write-Host ("  {0}) [{1}] {2,-20} {3} ({4})" -f $n, $risk, (Get-ActionText -Item $item -Field "title"), (Get-ActionText -Item $item -Field "desc"), $category)
                $flat.Add($item)
                $n += 1
            }
            Write-Host ""
        }
        $helpNo = $n
        $exitNo = $n + 1
        Write-Host ("  {0}) {1}" -f $helpNo, (T "quick_menu_help" "Help"))
        Write-Host ("  {0}) {1}" -f $exitNo, (T "quick_menu_exit" "Exit"))

        $choice = Read-Host ((T "quick_menu_select" "Select [1-{0}]" @($exitNo)))
        if ($choice -eq [string]$exitNo) { return }
        if ($choice -eq [string]$helpNo) { Show-Help; [void](Read-Host (T "quick_menu_press_enter" "Press Enter")); continue }

        $idx = -1
        [void][int]::TryParse($choice, [ref]$idx)
        if ($idx -lt 1 -or $idx -gt $flat.Count) {
            Write-QuickStatus -Text (T "quick_invalid_selection" "Invalid selection. Press Enter to continue.") -Level "warn"
            [void](Read-Host "")
            continue
        }
        try {
            Run-Action -Name ([string]$flat[$idx - 1].key)
        } catch {
            Write-QuickStatus -Text $_.Exception.Message -Level "error"
        }
        Write-Host ""
        [void](Read-Host (T "quick_return_menu" "Press Enter to return to menu"))
    }
}

function Show-Wizard {
    Write-QuickBanner
    Write-Host (T "quick_wizard_intro" "Wizard: answer 3 questions and launcher runs the right pipeline.") -ForegroundColor Cyan
    Write-Host ""
    Write-Host (T "quick_wizard_q1" "Q1) Your goal?") -ForegroundColor Cyan
    Write-Host ("  1) {0}" -f (T "quick_wizard_q1_opt1" "First-time setup + everything"))
    Write-Host ("  2) {0}" -f (T "quick_wizard_q1_opt2" "Build + boot smoke"))
    Write-Host ("  3) {0}" -f (T "quick_wizard_q1_opt3" "Dashboard only"))
    Write-Host ("  4) {0}" -f (T "quick_wizard_q1_opt4" "Quality gate only"))
    $q1 = Read-Host (T "quick_wizard_pick_1_4" "Pick [1-4]")

    Write-Host ""
    Write-Host (T "quick_wizard_q2" "Q2) Dependency state?") -ForegroundColor Cyan
    Write-Host ("  1) {0}" -f (T "quick_wizard_q2_opt1" "Not sure / maybe missing"))
    Write-Host ("  2) {0}" -f (T "quick_wizard_q2_opt2" "Already installed"))
    $q2 = Read-Host (T "quick_wizard_pick_1_2" "Pick [1-2]")

    Write-Host ""
    Write-Host (T "quick_wizard_q3" "Q3) Run level?") -ForegroundColor Cyan
    Write-Host "  1) quick"
    Write-Host "  2) strict"
    $q3 = Read-Host (T "quick_wizard_pick_1_2" "Pick [1-2]")
    if ($q3 -eq "2") { $script:Profile = "strict" }

    $actionKey = switch ($q1) {
        "1" { if ($q2 -eq "2") { "workspace-bootstrap" } else { "guided-bootstrap" } }
        "2" { "build-and-smoke" }
        "3" { "dashboard-build" }
        "4" { "quality-gate" }
        default { "guided-bootstrap" }
    }
    Write-Host ""
    Run-Action -Name $actionKey
}

function Show-Gui {
    Add-Type -AssemblyName System.Windows.Forms
    Add-Type -AssemblyName System.Drawing

    $form = New-Object System.Windows.Forms.Form
    $form.Text = (T "quick_banner_title" "HyperCore Quick Launcher")
    $form.Size = New-Object System.Drawing.Size(1024, 680)
    $form.StartPosition = "CenterScreen"
    $form.BackColor = [System.Drawing.Color]::FromArgb(14, 18, 30)
    $form.ForeColor = [System.Drawing.Color]::White

    $title = New-Object System.Windows.Forms.Label
    $title.Text = (T "quick_banner_title" "HyperCore Quick Launcher")
    $title.Font = New-Object System.Drawing.Font("Segoe UI Semibold", 20, [System.Drawing.FontStyle]::Bold)
    $title.AutoSize = $true
    $title.Location = New-Object System.Drawing.Point(20, 18)
    $form.Controls.Add($title)

    $sub = New-Object System.Windows.Forms.Label
    $sub.Text = (T "quick_gui_subtitle" "Choose one action. Review impact, steps, and agent status before launching.")
    $sub.Font = New-Object System.Drawing.Font("Segoe UI", 10)
    $sub.AutoSize = $true
    $sub.Location = New-Object System.Drawing.Point(22, 56)
    $form.Controls.Add($sub)

    $agentStatus = New-Object System.Windows.Forms.Label
    $agentStatus.Text = if (Test-LocalDashboardAgentHealthy) { "Agent: online" } else { "Agent: offline" }
    $agentStatus.Font = New-Object System.Drawing.Font("Segoe UI", 9, [System.Drawing.FontStyle]::Bold)
    $agentStatus.AutoSize = $true
    $agentStatus.Location = New-Object System.Drawing.Point(22, 84)
    $agentStatus.ForeColor = if (Test-LocalDashboardAgentHealthy) { [System.Drawing.Color]::FromArgb(112, 223, 160) } else { [System.Drawing.Color]::FromArgb(255, 191, 87) }
    $form.Controls.Add($agentStatus)

    $status = New-Object System.Windows.Forms.Label
    $status.Text = (T "quick_gui_ready" "Ready")
    $status.Font = New-Object System.Drawing.Font("Segoe UI", 10)
    $status.AutoSize = $true
    $status.Location = New-Object System.Drawing.Point(22, 610)
    $form.Controls.Add($status)

    $catalog = @(
        $script:ActionCatalog |
            Sort-Object `
                @{ Expression = { Get-GroupRank -GroupName (Get-ActionText -Item $_ -Field "group") } }, `
                @{ Expression = { Get-RiskRank -Risk (Get-ActionRisk -Item $_) } }, `
                @{ Expression = { Get-ActionText -Item $_ -Field "title" } }
    )

    $listPanel = New-Object System.Windows.Forms.Panel
    $listPanel.Location = New-Object System.Drawing.Point(22, 120)
    $listPanel.Size = New-Object System.Drawing.Size(420, 470)
    $listPanel.BackColor = [System.Drawing.Color]::FromArgb(20, 26, 42)
    $form.Controls.Add($listPanel)

    $listTitle = New-Object System.Windows.Forms.Label
    $listTitle.Text = "Actions"
    $listTitle.Font = New-Object System.Drawing.Font("Segoe UI Semibold", 11, [System.Drawing.FontStyle]::Bold)
    $listTitle.AutoSize = $true
    $listTitle.Location = New-Object System.Drawing.Point(16, 14)
    $listPanel.Controls.Add($listTitle)

    $listBox = New-Object System.Windows.Forms.ListBox
    $listBox.Location = New-Object System.Drawing.Point(16, 44)
    $listBox.Size = New-Object System.Drawing.Size(388, 394)
    $listBox.BackColor = [System.Drawing.Color]::FromArgb(26, 34, 54)
    $listBox.ForeColor = [System.Drawing.Color]::White
    $listBox.BorderStyle = "None"
    $listBox.Font = New-Object System.Drawing.Font("Consolas", 10)
    foreach ($item in $catalog) {
        $label = "[{0}] {1} ({2})" -f (Get-ActionRisk -Item $item), (Get-ActionText -Item $item -Field "title"), (Get-ActionCategory -Item $item)
        [void]$listBox.Items.Add($label)
    }
    $listPanel.Controls.Add($listBox)

    $detailsPanel = New-Object System.Windows.Forms.Panel
    $detailsPanel.Location = New-Object System.Drawing.Point(466, 120)
    $detailsPanel.Size = New-Object System.Drawing.Size(530, 470)
    $detailsPanel.BackColor = [System.Drawing.Color]::FromArgb(20, 26, 42)
    $form.Controls.Add($detailsPanel)

    $detailsTitle = New-Object System.Windows.Forms.Label
    $detailsTitle.Text = "Action Details"
    $detailsTitle.Font = New-Object System.Drawing.Font("Segoe UI Semibold", 11, [System.Drawing.FontStyle]::Bold)
    $detailsTitle.AutoSize = $true
    $detailsTitle.Location = New-Object System.Drawing.Point(18, 14)
    $detailsPanel.Controls.Add($detailsTitle)

    $selectedTitle = New-Object System.Windows.Forms.Label
    $selectedTitle.Font = New-Object System.Drawing.Font("Segoe UI Semibold", 16, [System.Drawing.FontStyle]::Bold)
    $selectedTitle.AutoSize = $true
    $selectedTitle.Location = New-Object System.Drawing.Point(18, 48)
    $detailsPanel.Controls.Add($selectedTitle)

    $pillRisk = New-Object System.Windows.Forms.Label
    $pillRisk.Font = New-Object System.Drawing.Font("Segoe UI", 9, [System.Drawing.FontStyle]::Bold)
    $pillRisk.AutoSize = $true
    $pillRisk.Padding = New-Object System.Windows.Forms.Padding(8, 4, 8, 4)
    $pillRisk.Location = New-Object System.Drawing.Point(20, 88)
    $detailsPanel.Controls.Add($pillRisk)

    $pillCategory = New-Object System.Windows.Forms.Label
    $pillCategory.Font = New-Object System.Drawing.Font("Segoe UI", 9)
    $pillCategory.AutoSize = $true
    $pillCategory.Padding = New-Object System.Windows.Forms.Padding(8, 4, 8, 4)
    $pillCategory.Location = New-Object System.Drawing.Point(112, 88)
    $pillCategory.BackColor = [System.Drawing.Color]::FromArgb(42, 52, 84)
    $pillCategory.ForeColor = [System.Drawing.Color]::White
    $detailsPanel.Controls.Add($pillCategory)

    $selectedDesc = New-Object System.Windows.Forms.Label
    $selectedDesc.Location = New-Object System.Drawing.Point(20, 128)
    $selectedDesc.Size = New-Object System.Drawing.Size(490, 54)
    $selectedDesc.Font = New-Object System.Drawing.Font("Segoe UI", 10)
    $detailsPanel.Controls.Add($selectedDesc)

    $stepsLabel = New-Object System.Windows.Forms.Label
    $stepsLabel.Text = "Planned steps"
    $stepsLabel.Font = New-Object System.Drawing.Font("Segoe UI Semibold", 10, [System.Drawing.FontStyle]::Bold)
    $stepsLabel.AutoSize = $true
    $stepsLabel.Location = New-Object System.Drawing.Point(20, 196)
    $detailsPanel.Controls.Add($stepsLabel)

    $stepsBox = New-Object System.Windows.Forms.TextBox
    $stepsBox.Location = New-Object System.Drawing.Point(20, 224)
    $stepsBox.Size = New-Object System.Drawing.Size(490, 168)
    $stepsBox.Multiline = $true
    $stepsBox.ReadOnly = $true
    $stepsBox.BorderStyle = "None"
    $stepsBox.BackColor = [System.Drawing.Color]::FromArgb(26, 34, 54)
    $stepsBox.ForeColor = [System.Drawing.Color]::White
    $stepsBox.Font = New-Object System.Drawing.Font("Consolas", 10)
    $detailsPanel.Controls.Add($stepsBox)

    $groupLabel = New-Object System.Windows.Forms.Label
    $groupLabel.Location = New-Object System.Drawing.Point(20, 406)
    $groupLabel.Size = New-Object System.Drawing.Size(490, 22)
    $groupLabel.Font = New-Object System.Drawing.Font("Segoe UI", 9)
    $groupLabel.ForeColor = [System.Drawing.Color]::Gainsboro
    $detailsPanel.Controls.Add($groupLabel)

    $runButton = New-Object System.Windows.Forms.Button
    $runButton.Text = "Run Selected Action"
    $runButton.Size = New-Object System.Drawing.Size(220, 42)
    $runButton.Location = New-Object System.Drawing.Point(20, 426)
    $runButton.BackColor = [System.Drawing.Color]::FromArgb(30, 102, 214)
    $runButton.ForeColor = [System.Drawing.Color]::White
    $runButton.FlatStyle = "Flat"
    $runButton.FlatAppearance.BorderSize = 0
    $detailsPanel.Controls.Add($runButton)

    $refreshButton = New-Object System.Windows.Forms.Button
    $refreshButton.Text = "Refresh Agent Status"
    $refreshButton.Size = New-Object System.Drawing.Size(170, 42)
    $refreshButton.Location = New-Object System.Drawing.Point(256, 426)
    $refreshButton.BackColor = [System.Drawing.Color]::FromArgb(42, 52, 84)
    $refreshButton.ForeColor = [System.Drawing.Color]::White
    $refreshButton.FlatStyle = "Flat"
    $refreshButton.FlatAppearance.BorderSize = 0
    $detailsPanel.Controls.Add($refreshButton)

    $selectedIndex = 0

    function Update-SelectedActionDetails {
        param([int]$Index)
        if ($Index -lt 0 -or $Index -ge $catalog.Count) { return }
        $item = $catalog[$Index]
        $selectedTitle.Text = Get-ActionText -Item $item -Field "title"
        $selectedDesc.Text = Get-ActionText -Item $item -Field "desc"
        $pillRisk.Text = Get-ActionRisk -Item $item
        $pillCategory.Text = (Get-ActionCategory -Item $item).ToUpperInvariant()
        $riskColors = Get-RiskUiColors -Risk (Get-ActionRisk -Item $item)
        $pillRisk.ForeColor = $riskColors.Foreground
        $pillRisk.BackColor = $riskColors.Background
        $groupLabel.Text = "Group: {0}  |  Key: {1}" -f (Get-ActionText -Item $item -Field "group"), [string]$item.key
        $stepLines = @()
        $stepIndex = 1
        foreach ($step in @(Normalize-StepList -InputSteps $item.cmds)) {
            $stepLines += ("{0}. {1}" -f $stepIndex, [string]$step)
            $stepIndex += 1
        }
        if ([string]$item.key -eq "dashboard-open") {
            $stepLines += ""
            $stepLines += "Note: launcher can auto-build missing dashboard artifacts and auto-start the local agent."
        }
        $stepsBox.Text = ($stepLines -join [Environment]::NewLine)
        $script:selectedActionKey = [string]$item.key
    }

    $listBox.Add_SelectedIndexChanged({
        if ($listBox.SelectedIndex -ge 0) {
            $selectedIndex = $listBox.SelectedIndex
            Update-SelectedActionDetails -Index $selectedIndex
        }
    })

    $runButton.Add_Click({
        if (-not $script:selectedActionKey) { return }
        $status.Text = (T "quick_gui_running" "Running: {0}" @($script:selectedActionKey))
        $form.Refresh()
        try {
            Run-Action -Name $script:selectedActionKey
            $status.Text = (T "quick_gui_done" "Done: {0}" @($script:selectedActionKey))
        } catch {
            $status.Text = (T "quick_gui_failed" "Failed: {0}" @($script:selectedActionKey))
            [System.Windows.Forms.MessageBox]::Show($_.Exception.Message, (T "quick_gui_error" "Quick Launcher Error"), "OK", "Error") | Out-Null
        }
    })

    $refreshButton.Add_Click({
        $online = Test-LocalDashboardAgentHealthy
        $agentStatus.Text = if ($online) { "Agent: online" } else { "Agent: offline" }
        $agentStatus.ForeColor = if ($online) { [System.Drawing.Color]::FromArgb(112, 223, 160) } else { [System.Drawing.Color]::FromArgb(255, 191, 87) }
    })

    if ($catalog.Count -gt 0) {
        $listBox.SelectedIndex = 0
        Update-SelectedActionDetails -Index 0
    }

    [void]$form.ShowDialog()
}

Initialize-QuickLocale
$script:ActionCatalog = Get-ActionCatalog
$script:ShellExe = Resolve-QuickShell

try {
    Initialize-RunLog
    if (-not (Test-Path $script:HypercorePath)) {
        throw "hypercore.ps1 not found: $script:HypercorePath"
    }
    switch ($Action) {
        "menu" { Show-Menu }
        "wizard" { Show-Wizard }
        "gui" { Show-Gui }
        "help" { Show-Help }
        default {
            Write-QuickBanner
            Run-Action -Name $Action
        }
    }
} catch {
    Save-RunLog -ActionName $Action -Results @() -Status "failed"
    Write-QuickStatus -Text $_.Exception.Message -Level "error"
    exit 1
}

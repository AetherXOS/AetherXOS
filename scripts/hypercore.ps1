<#
.SYNOPSIS
Unified tooling entrypoint for HyperCore boot/test workflows.
#>

param(
    [ValidateSet("doctor", "report", "install", "install-deno", "build-iso", "qemu-smoke", "qemu-live", "dashboard-agent", "dashboard-agent-nosafe", "dashboard-agent-bg", "dashboard-agent-nosafe-bg", "dashboard-agent-contract", "ci-smoke", "verify", "list-tasks", "run-task", "gate", "cleanup", "health", "test-scripts", "dashboard", "trends", "collect-diagnostics", "explain-last-failure", "triage", "pre-release", "html-dashboard", "check-updates", "plugins", "run-plugin", "detect-flaky", "bootstrap", "first-run", "doctor-fix", "help", "open-report", "support-bundle", "dry-run-diff", "prereq-check", "apply-fix", "artifact-manifest", "verify-artifacts", "anomaly-report", "bisect-helper", "dependency-drift", "policy-gate", "linux-abi-gate", "linux-platform-readiness", "merge-telemetry", "validate-schemas", "canary", "replay-run", "lint-i18n", "release-notes", "tier-status", "dashboard-ui-build", "dashboard-ui-dev", "dashboard-ui-test", "dashboard-ui-e2e", "dashboard-ui-e2e-setup", "idempotency-check", "tooling-quality-gate", "os-smoke-dashboard", "os-full-dashboard", "secureboot-sign", "secureboot-sbat", "secureboot-mok-plan", "secureboot-ovmf-matrix", "secureboot-pcr-report")]
    [string]$Command = "doctor",
    [ValidateSet("quick", "strict")]
    [string]$Profile = "quick",
    [int]$Rounds = -1,
    [string]$MemoryMb = "",
    [string]$Cores = "",
    [int]$RoundTimeoutSec = -1,
    [double]$ChaosRate = -1,
    [switch]$InstallWslDeps,
    [switch]$NoTimeoutAsSuccess,
    [switch]$Offline,
    [switch]$UseCache,
    [switch]$DryRun,
    [string]$OutDir = "",
    [string]$TaskName = "",
    [ValidateSet("p0", "p1", "p2", "rc", "all")]
    [string]$GateStage = "all",
    [switch]$ParallelGates,
    [int]$MaxParallel = 2,
    [int]$KeepLatest = -1,
    [int]$TrendWindow = 20,
    [int]$FailureTail = 5,
    [int]$FlakyWindow = 100,
    [string]$DashboardPath = "reports/tooling/dashboard.md",
    [ValidateSet("auto", "ui", "html")]
    [string]$ReportTarget = "auto",
    [string]$TriagePath = "reports/tooling/triage.md",
    [string]$DiagnosticsZipPath = "artifacts/diagnostics/hypercore_diagnostics.zip",
    [string]$PluginName = "",
    [switch]$ListTaskGraph,
    [switch]$NoLock,
    [switch]$AutoApprove,
    [switch]$JsonOutput,
    [switch]$Quiet,
    [switch]$VerboseMode,
    [switch]$NoColor,
    [switch]$Deterministic,
    [int]$Seed = 42,
    [ValidateSet("en", "tr")]
    [string]$Lang = "",
    [string]$ReplayManifestPath = "",
    [string]$MergeTelemetryDir = "reports/tooling/telemetry_inputs",
    [string]$ArtifactManifestPath = "reports/tooling/artifact_manifest.json",
    [string]$ConfigPath = ".\scripts\config\hypercore.defaults.json",
    [switch]$WriteDoctorReport
)

$ErrorActionPreference = "Stop"
Import-Module (Join-Path $PSScriptRoot "lib/Hypercore.Common.psm1") -Force

$modulePaths = @(
    (Join-Path $PSScriptRoot "hypercore/localization.ps1"),
    (Join-Path $PSScriptRoot "hypercore/plugins.ps1"),
    (Join-Path $PSScriptRoot "hypercore/novice.ps1")
)
foreach ($mp in $modulePaths) {
    if (-not (Test-Path $mp)) { throw "required module file missing: $mp" }
    . $mp
}

$script:RunId = [guid]::NewGuid().ToString()
$script:Settings = $null
$script:RunLockPath = "artifacts/tooling/hypercore.lock.json"
$script:RunManifestPath = ""
$script:RunStartUtc = [DateTime]::UtcNow
$script:RunStatus = "unknown"
$script:PluginRegistry = @{}
$script:LocaleCatalog = @{}
$script:ShellExe = ""
$script:PluginApiVersion = "1.0"
$script:CommandHandlers = @{}
$script:ExitCodeMap = @{
    unknown = 1
    dependency_missing = 10
    config_invalid = 11
    command_failed = 12
    test_failed = 13
    qemu_failed = 14
    timeout = 15
    lock_conflict = 16
    plugin_invalid = 17
}

function Get-ScopedProfileConfig {
    param($Cfg, [string]$ProfileName)
    if (-not $Cfg.profiles.PSObject.Properties.Name.Contains($ProfileName)) {
        throw "profile not found in config: $ProfileName"
    }
    return $Cfg.profiles.$ProfileName
}

function Resolve-EffectiveSettings {
    param($Cfg, [string]$ProfileName)
    $p = Get-ScopedProfileConfig -Cfg $Cfg -ProfileName $ProfileName
    return [ordered]@{
        rounds = if ($Rounds -gt 0) { $Rounds } else { [int]$p.rounds }
        memory_mb = if ($MemoryMb) { $MemoryMb } else { [string]$p.memory_mb }
        cores = if ($Cores) { $Cores } else { [string]$p.cores }
        round_timeout_sec = if ($RoundTimeoutSec -gt 0) { $RoundTimeoutSec } else { [int]$p.round_timeout_sec }
        chaos_rate = if ($ChaosRate -ge 0) { $ChaosRate } else { [double]$p.chaos_rate }
        allow_timeout_success = if ($NoTimeoutAsSuccess) { $false } else { [bool]$p.allow_timeout_success }
        limine_bin_dir = [string]$Cfg.paths.limine_bin_dir
        shim_bin_dir = if ($Cfg.paths -and $Cfg.paths.shim_bin_dir) { [string]$Cfg.paths.shim_bin_dir } else { "artifacts/shim/bin" }
        shim_mode_enabled = if ($Cfg.boot -and $Cfg.boot.shim_limine_combo_enabled) { [bool]$Cfg.boot.shim_limine_combo_enabled } else { $false }
        shim_chainloader = if ($Cfg.boot -and $Cfg.boot.shim_chainloader) { [string]$Cfg.boot.shim_chainloader } else { "grubx64.efi" }
        grub_limine_target = if ($Cfg.boot -and $Cfg.boot.grub_limine_target) { [string]$Cfg.boot.grub_limine_target } else { "liminex64.efi" }
        write_grub_fallback = if ($Cfg.boot -and $Cfg.boot.write_grub_fallback) { [bool]$Cfg.boot.write_grub_fallback } else { $true }
        cargo_features = if ($Cfg.build -and $Cfg.build.cargo_features) { [string]$Cfg.build.cargo_features } else { "" }
        cargo_no_default_features = if ($Cfg.build -and $Cfg.build.cargo_no_default_features) { [bool]$Cfg.build.cargo_no_default_features } else { $false }
        qemu_default_out_dir = [string]$Cfg.paths.qemu_default_out_dir
        doctor_report_path = [string]$Cfg.paths.doctor_report_path
        health_report_path = [string]$Cfg.paths.health_report_path
        telemetry_jsonl_path = [string]$Cfg.paths.telemetry_jsonl_path
        tasks_path = [string]$Cfg.paths.tasks_path
        plugin_dir = if ($Cfg.paths.plugin_dir) { [string]$Cfg.paths.plugin_dir } else { "scripts/plugins" }
        cleanup_keep_latest = if ($KeepLatest -gt 0) { $KeepLatest } else { [int]$Cfg.cleanup.keep_latest_runs }
        cleanup_keep_days = if ($Cfg.cleanup.keep_days) { [int]$Cfg.cleanup.keep_days } else { 14 }
        cleanup_max_artifacts_gb = if ($Cfg.cleanup.max_artifacts_gb) { [double]$Cfg.cleanup.max_artifacts_gb } else { 8.0 }
        cleanup_targets = @($Cfg.cleanup.targets)
        health_weights = $Cfg.health.weights
        retry_count = if ($Cfg.execution.retry_count) { [int]$Cfg.execution.retry_count } else { 2 }
        retry_delay_sec = if ($Cfg.execution.retry_delay_sec) { [int]$Cfg.execution.retry_delay_sec } else { 2 }
        retry_jitter_ms = if ($Cfg.execution.retry_jitter_ms) { [int]$Cfg.execution.retry_jitter_ms } else { 150 }
        lock_ttl_min = if ($Cfg.execution.lock_ttl_min) { [int]$Cfg.execution.lock_ttl_min } else { 240 }
        telemetry_rotate_mb = if ($Cfg.execution.telemetry_rotate_mb) { [int]$Cfg.execution.telemetry_rotate_mb } else { 8 }
        telemetry_rotate_keep = if ($Cfg.execution.telemetry_rotate_keep) { [int]$Cfg.execution.telemetry_rotate_keep } else { 5 }
        cache_ttl_sec = if ($Cfg.execution.cache_ttl_sec) { [int]$Cfg.execution.cache_ttl_sec } else { 120 }
        script_version = if ($Cfg.tooling.version) { [string]$Cfg.tooling.version } else { "0.1.0" }
        language = if ($Cfg.tooling.language) { [string]$Cfg.tooling.language } else { "en" }
        disk_usage_aggressive_cleanup_pct = if ($Cfg.cleanup.disk_usage_aggressive_cleanup_pct) { [int]$Cfg.cleanup.disk_usage_aggressive_cleanup_pct } else { 90 }
        ui_feature_flags = if ($Cfg.ui -and $Cfg.ui.feature_flags) { $Cfg.ui.feature_flags } else { [ordered]@{
            alert_center = $true
            telemetry_heatmap = $true
            event_charts = $true
            timeline_chart = $true
            release_cockpit = $true
            topology_graph = $true
        } }
        agent_port = if ($Cfg.agent -and $Cfg.agent.port) { [int]$Cfg.agent.port } else { 7401 }
        agent_auth_token = if ($Cfg.agent -and $Cfg.agent.auth_token) { [string]$Cfg.agent.auth_token } else { "" }
        agent_allowed_origins = if ($Cfg.agent -and $Cfg.agent.allowed_origins) { @($Cfg.agent.allowed_origins) } else { @("http://127.0.0.1","http://localhost","null") }
        agent_max_concurrency = if ($Cfg.agent -and $Cfg.agent.max_concurrency) { [int]$Cfg.agent.max_concurrency } else { 1 }
        agent_max_queue = if ($Cfg.agent -and $Cfg.agent.max_queue) { [int]$Cfg.agent.max_queue } else { 100 }
        agent_log_retention_days = if ($Cfg.agent -and $Cfg.agent.log_retention_days) { [int]$Cfg.agent.log_retention_days } else { 14 }
    }
}

function Initialize-UiMode {
    $env:HYPERCORE_JSON_OUTPUT = if ($JsonOutput) { "1" } else { "0" }
    $env:HYPERCORE_QUIET = if ($Quiet) { "1" } else { "0" }
    if ($NoColor) {
        $env:NO_COLOR = "1"
        if (Get-Variable -Name PSStyle -ErrorAction SilentlyContinue) {
            $PSStyle.OutputRendering = "PlainText"
        }
    }
    if ($VerboseMode) { $VerbosePreference = "Continue" }
    if ($Deterministic) {
        $env:HYPERCORE_DETERMINISTIC = "1"
        $env:HYPERCORE_SEED = [string]$Seed
    } else {
        $env:HYPERCORE_DETERMINISTIC = "0"
    }
}

function Migrate-ConfigIfNeeded {
    param($Cfg)
    $cv = 1
    if ($Cfg.PSObject.Properties.Name.Contains("config_version")) {
        $cv = [int]$Cfg.config_version
    }
    if ($cv -lt 2) {
        if (-not $Cfg.PSObject.Properties.Name.Contains("execution")) {
            $Cfg | Add-Member -NotePropertyName execution -NotePropertyValue ([ordered]@{}) -Force
        }
        if (-not $Cfg.execution.PSObject.Properties.Name.Contains("retry_count")) { $Cfg.execution | Add-Member -NotePropertyName retry_count -NotePropertyValue 2 -Force }
        if (-not $Cfg.execution.PSObject.Properties.Name.Contains("retry_delay_sec")) { $Cfg.execution | Add-Member -NotePropertyName retry_delay_sec -NotePropertyValue 2 -Force }
        if (-not $Cfg.execution.PSObject.Properties.Name.Contains("retry_jitter_ms")) { $Cfg.execution | Add-Member -NotePropertyName retry_jitter_ms -NotePropertyValue 150 -Force }
        if (-not $Cfg.execution.PSObject.Properties.Name.Contains("lock_ttl_min")) { $Cfg.execution | Add-Member -NotePropertyName lock_ttl_min -NotePropertyValue 240 -Force }
        if (-not $Cfg.execution.PSObject.Properties.Name.Contains("telemetry_rotate_mb")) { $Cfg.execution | Add-Member -NotePropertyName telemetry_rotate_mb -NotePropertyValue 8 -Force }
        if (-not $Cfg.execution.PSObject.Properties.Name.Contains("telemetry_rotate_keep")) { $Cfg.execution | Add-Member -NotePropertyName telemetry_rotate_keep -NotePropertyValue 5 -Force }
        if (-not $Cfg.execution.PSObject.Properties.Name.Contains("cache_ttl_sec")) { $Cfg.execution | Add-Member -NotePropertyName cache_ttl_sec -NotePropertyValue 120 -Force }
        if (-not $Cfg.PSObject.Properties.Name.Contains("tooling")) {
            $Cfg | Add-Member -NotePropertyName tooling -NotePropertyValue ([ordered]@{ version = "0.4.0"; language = "en" }) -Force
        }
        if (-not $Cfg.PSObject.Properties.Name.Contains("cleanup")) {
            $Cfg | Add-Member -NotePropertyName cleanup -NotePropertyValue ([ordered]@{}) -Force
        }
        if (-not $Cfg.cleanup.PSObject.Properties.Name.Contains("disk_usage_aggressive_cleanup_pct")) {
            $Cfg.cleanup | Add-Member -NotePropertyName disk_usage_aggressive_cleanup_pct -NotePropertyValue 90 -Force
        }
        $Cfg.config_version = 2
    }
    if ($cv -lt 3) {
        if (-not $Cfg.execution.PSObject.Properties.Name.Contains("telemetry_rotate_mb")) { $Cfg.execution | Add-Member -NotePropertyName telemetry_rotate_mb -NotePropertyValue 8 -Force }
        if (-not $Cfg.execution.PSObject.Properties.Name.Contains("telemetry_rotate_keep")) { $Cfg.execution | Add-Member -NotePropertyName telemetry_rotate_keep -NotePropertyValue 5 -Force }
        if (-not $Cfg.execution.PSObject.Properties.Name.Contains("cache_ttl_sec")) { $Cfg.execution | Add-Member -NotePropertyName cache_ttl_sec -NotePropertyValue 120 -Force }
        if (-not $Cfg.cleanup.PSObject.Properties.Name.Contains("disk_usage_aggressive_cleanup_pct")) {
            $Cfg.cleanup | Add-Member -NotePropertyName disk_usage_aggressive_cleanup_pct -NotePropertyValue 90 -Force
        }
        $Cfg.config_version = 3
    }
    if ($cv -lt 4) {
        if (-not $Cfg.PSObject.Properties.Name.Contains("agent")) {
            $Cfg | Add-Member -NotePropertyName agent -NotePropertyValue ([ordered]@{}) -Force
        }
        if (-not $Cfg.agent.PSObject.Properties.Name.Contains("port")) { $Cfg.agent | Add-Member -NotePropertyName port -NotePropertyValue 7401 -Force }
        if (-not $Cfg.agent.PSObject.Properties.Name.Contains("auth_token")) { $Cfg.agent | Add-Member -NotePropertyName auth_token -NotePropertyValue "change-me-hypercore-agent-token" -Force }
        if (-not $Cfg.agent.PSObject.Properties.Name.Contains("tokens")) { $Cfg.agent | Add-Member -NotePropertyName tokens -NotePropertyValue ([ordered]@{ viewer = ""; operator = ""; admin = "change-me-hypercore-agent-token" }) -Force }
        if (-not $Cfg.agent.PSObject.Properties.Name.Contains("allowed_origins")) { $Cfg.agent | Add-Member -NotePropertyName allowed_origins -NotePropertyValue @("http://127.0.0.1","http://localhost","null") -Force }
        if (-not $Cfg.agent.PSObject.Properties.Name.Contains("max_concurrency")) { $Cfg.agent | Add-Member -NotePropertyName max_concurrency -NotePropertyValue 1 -Force }
        if (-not $Cfg.agent.PSObject.Properties.Name.Contains("max_queue")) { $Cfg.agent | Add-Member -NotePropertyName max_queue -NotePropertyValue 100 -Force }
        if (-not $Cfg.agent.PSObject.Properties.Name.Contains("log_retention_days")) { $Cfg.agent | Add-Member -NotePropertyName log_retention_days -NotePropertyValue 14 -Force }
        if (-not $Cfg.agent.PSObject.Properties.Name.Contains("hosts")) { $Cfg.agent | Add-Member -NotePropertyName hosts -NotePropertyValue @([ordered]@{ id = "local"; name = "Localhost"; url = "http://127.0.0.1:7401"; enabled = $true; role_hint = "admin" }) -Force }
        $Cfg.config_version = 4
    }
    if (-not $Cfg.PSObject.Properties.Name.Contains("ui")) {
        $Cfg | Add-Member -NotePropertyName ui -NotePropertyValue ([ordered]@{}) -Force
    }
    if (-not $Cfg.ui.PSObject.Properties.Name.Contains("feature_flags")) {
        $Cfg.ui | Add-Member -NotePropertyName feature_flags -NotePropertyValue ([ordered]@{
            alert_center = $true
            telemetry_heatmap = $true
            event_charts = $true
            timeline_chart = $true
            release_cockpit = $true
            topology_graph = $true
        }) -Force
    }
    if (-not $Cfg.PSObject.Properties.Name.Contains("install")) {
        $Cfg | Add-Member -NotePropertyName install -NotePropertyValue ([ordered]@{}) -Force
    }
    if (-not $Cfg.install.PSObject.Properties.Name.Contains("node")) {
        $Cfg.install | Add-Member -NotePropertyName node -NotePropertyValue $true -Force
    }
    if (-not $Cfg.install.PSObject.Properties.Name.Contains("deno")) {
        $Cfg.install | Add-Member -NotePropertyName deno -NotePropertyValue $true -Force
    }
    if (-not $Cfg.PSObject.Properties.Name.Contains("agent")) {
        $Cfg | Add-Member -NotePropertyName agent -NotePropertyValue ([ordered]@{}) -Force
    }
    if (-not $Cfg.agent.PSObject.Properties.Name.Contains("port")) { $Cfg.agent | Add-Member -NotePropertyName port -NotePropertyValue 7401 -Force }
    if (-not $Cfg.agent.PSObject.Properties.Name.Contains("auth_token")) { $Cfg.agent | Add-Member -NotePropertyName auth_token -NotePropertyValue "change-me-hypercore-agent-token" -Force }
    if (-not $Cfg.agent.PSObject.Properties.Name.Contains("tokens")) { $Cfg.agent | Add-Member -NotePropertyName tokens -NotePropertyValue ([ordered]@{ viewer = ""; operator = ""; admin = "change-me-hypercore-agent-token" }) -Force }
    if (-not $Cfg.agent.PSObject.Properties.Name.Contains("allowed_origins")) { $Cfg.agent | Add-Member -NotePropertyName allowed_origins -NotePropertyValue @("http://127.0.0.1","http://localhost","null") -Force }
    if (-not $Cfg.agent.PSObject.Properties.Name.Contains("max_concurrency")) { $Cfg.agent | Add-Member -NotePropertyName max_concurrency -NotePropertyValue 1 -Force }
    if (-not $Cfg.agent.PSObject.Properties.Name.Contains("max_queue")) { $Cfg.agent | Add-Member -NotePropertyName max_queue -NotePropertyValue 100 -Force }
    if (-not $Cfg.agent.PSObject.Properties.Name.Contains("log_retention_days")) { $Cfg.agent | Add-Member -NotePropertyName log_retention_days -NotePropertyValue 14 -Force }
    if (-not $Cfg.agent.PSObject.Properties.Name.Contains("hosts")) { $Cfg.agent | Add-Member -NotePropertyName hosts -NotePropertyValue @([ordered]@{ id = "local"; name = "Localhost"; url = "http://127.0.0.1:7401"; enabled = $true; role_hint = "admin" }) -Force }
    if (-not $Cfg.PSObject.Properties.Name.Contains("build")) {
        $Cfg | Add-Member -NotePropertyName build -NotePropertyValue ([ordered]@{}) -Force
    }
    if (-not $Cfg.build.PSObject.Properties.Name.Contains("cargo_features")) {
        $Cfg.build | Add-Member -NotePropertyName cargo_features -NotePropertyValue "" -Force
    }
    if (-not $Cfg.build.PSObject.Properties.Name.Contains("cargo_no_default_features")) {
        $Cfg.build | Add-Member -NotePropertyName cargo_no_default_features -NotePropertyValue $false -Force
    }
    return $Cfg
}

function Get-HcCachePath {
    param([string]$Key)
    $safe = ($Key -replace '[^a-zA-Z0-9_\-]','_')
    return ("artifacts/tooling/cache/{0}.json" -f $safe)
}

function Get-HcCache {
    param([string]$Key, [int]$TtlSec = 60)
    $p = Get-HcCachePath -Key $Key
    if (-not (Test-Path $p)) { return $null }
    try {
        $obj = Get-HcJsonFile -Path $p
        $ts = [DateTime]::Parse([string]$obj.timestamp_utc)
        if (([DateTime]::UtcNow - $ts).TotalSeconds -gt $TtlSec) { return $null }
        return $obj.payload
    } catch {
        return $null
    }
}

function Set-HcCache {
    param([string]$Key, $Payload)
    $p = Get-HcCachePath -Key $Key
    $obj = [ordered]@{
        timestamp_utc = [DateTime]::UtcNow.ToString("o")
        payload = $Payload
    }
    Save-HcJsonFile -Object $obj -Path $p
}

function Rotate-TelemetryIfNeeded {
    param($Settings)
    $out = $Settings.telemetry_jsonl_path
    if (-not (Test-Path $out -PathType Leaf)) { return }
    $maxBytes = [int64]$Settings.telemetry_rotate_mb * 1MB
    $fi = Get-Item $out
    if ($fi.Length -lt $maxBytes) { return }
    $stamp = [DateTime]::UtcNow.ToString("yyyyMMdd_HHmmss")
    $rot = "{0}.{1}.jsonl" -f $out, $stamp
    Move-Item -Force -Path $out -Destination $rot
    $gz = "$rot.gz"
    $inS = [System.IO.File]::OpenRead($rot)
    try {
        $outS = [System.IO.File]::Create($gz)
        try {
            $gzS = New-Object System.IO.Compression.GZipStream($outS, [System.IO.Compression.CompressionMode]::Compress)
            try { $inS.CopyTo($gzS) } finally { $gzS.Dispose() }
        } finally { $outS.Dispose() }
    } finally { $inS.Dispose() }
    Remove-Item -Force -Path $rot
    $dir = Split-Path -Parent $out
    $base = Split-Path -Leaf $out
    $old = Get-ChildItem -Path $dir -Filter "$base.*.jsonl.gz" | Sort-Object LastWriteTime -Descending
    $drop = @($old | Select-Object -Skip ([int]$Settings.telemetry_rotate_keep))
    foreach ($d in $drop) { Remove-Item -Force -Path $d.FullName -ErrorAction SilentlyContinue }
}

function Add-TelemetryEvent {
    param(
        [string]$Event,
        [string]$Status = "ok",
        [hashtable]$Data = @{},
        [string]$Level = "info",
        [string]$Code = "none",
        [string]$Component = "tooling",
        [double]$DurationMs = 0
    )
    if ($null -eq $script:Settings) { return }
    Rotate-TelemetryIfNeeded -Settings $script:Settings
    $payload = [ordered]@{
        timestamp_utc = [DateTime]::UtcNow.ToString("o")
        run_id = $script:RunId
        correlation_id = $script:RunId
        command = $Command
        profile = $Profile
        event = $Event
        status = $Status
        level = $Level
        code = $Code
        component = $Component
        duration_ms = [math]::Round($DurationMs, 3)
        data = $Data
    }
    $line = ($payload | ConvertTo-Json -Depth 8 -Compress)
    $out = $script:Settings.telemetry_jsonl_path
    $dir = Split-Path -Parent $out
    if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
    if (Test-Path $out -PathType Container) {
        Remove-Item -Recurse -Force -Path $out
    }
    Invoke-HcWithRetry -RetryCount 3 -DelaySeconds 1 -Action {
        $mutexName = "Global\HyperCoreTelemetryLock"
        $mtx = New-Object System.Threading.Mutex($false, $mutexName)
        try {
            [void]$mtx.WaitOne(10000)
            $line | Out-File -FilePath $out -Append -Encoding utf8
        } finally {
            try { $mtx.ReleaseMutex() | Out-Null } catch {}
            $mtx.Dispose()
        }
    }
}

function Write-HcMsg {
    param(
        [string]$Key,
        [object[]]$FormatArgs = @()
    )
    $flat = @($FormatArgs)
    if ($flat.Count -eq 1 -and $flat[0] -is [System.Array]) {
        $flat = @($flat[0])
    }
    Write-HcStep "hypercore:$Command" (Get-Msg -Key $Key -FormatArgs $flat)
}

function Validate-ConfigSchema {
    param($Cfg)
    $missing = @()
    foreach ($k in @("config_version","profiles","paths","install","cleanup","health")) {
        if (-not $Cfg.PSObject.Properties.Name.Contains($k)) { $missing += $k }
    }
    if ($missing.Count -gt 0) {
        Fail-Hc -Code "config_invalid" -Message ("config schema invalid; missing keys: {0}" -f ($missing -join ", "))
    }
    if (-not $Cfg.paths.PSObject.Properties.Name.Contains("tasks_path")) {
        Fail-Hc -Code "config_invalid" -Message "config schema invalid; paths.tasks_path missing"
    }
    if (-not $Cfg.profiles.PSObject.Properties.Name.Contains("quick")) {
        Fail-Hc -Code "config_invalid" -Message "config schema invalid; profiles.quick missing"
    }
    if (-not $Cfg.profiles.PSObject.Properties.Name.Contains("strict")) {
        Fail-Hc -Code "config_invalid" -Message "config schema invalid; profiles.strict missing"
    }
    foreach ($pn in @("quick","strict")) {
        $p = $Cfg.profiles.$pn
        if ([int]$p.rounds -lt 1) { Fail-Hc -Code "config_invalid" -Message "profiles.$pn.rounds must be >= 1" }
        if ([int]$p.round_timeout_sec -lt 5) { Fail-Hc -Code "config_invalid" -Message "profiles.$pn.round_timeout_sec must be >= 5" }
        $cr = [double]$p.chaos_rate
        if ($cr -lt 0 -or $cr -gt 1) { Fail-Hc -Code "config_invalid" -Message "profiles.$pn.chaos_rate must be in [0,1]" }
    }
    if ($Cfg.execution) {
        if ([int]$Cfg.execution.retry_count -lt 0 -or [int]$Cfg.execution.retry_count -gt 10) { Fail-Hc -Code "config_invalid" -Message "execution.retry_count out of range (0..10)" }
        if ([int]$Cfg.execution.retry_delay_sec -lt 0 -or [int]$Cfg.execution.retry_delay_sec -gt 60) { Fail-Hc -Code "config_invalid" -Message "execution.retry_delay_sec out of range (0..60)" }
        if ([int]$Cfg.execution.telemetry_rotate_mb -lt 1) { Fail-Hc -Code "config_invalid" -Message "execution.telemetry_rotate_mb must be >= 1" }
    }
}

function Resolve-ExitCode {
    param([string]$ErrorText)
    $t = ($ErrorText | Out-String)
    if ($t -match "HCERR\[([a-z_]+)\]") {
        $key = $Matches[1]
        if ($script:ExitCodeMap.ContainsKey($key)) { return [int]$script:ExitCodeMap[$key] }
    }
    if ($t -match "lock file exists") { return [int]$script:ExitCodeMap.lock_conflict }
    if ($t -match "config schema invalid|profile not found in config|json file not found") { return [int]$script:ExitCodeMap.config_invalid }
    if ($t -match "missing|not found") { return [int]$script:ExitCodeMap.dependency_missing }
    if ($t -match "Pester|FailedCount|test") { return [int]$script:ExitCodeMap.test_failed }
    if ($t -match "qemu|QEMU") { return [int]$script:ExitCodeMap.qemu_failed }
    if ($t -match "timeout") { return [int]$script:ExitCodeMap.timeout }
    if ($t -match "command failed") { return [int]$script:ExitCodeMap.command_failed }
    return [int]$script:ExitCodeMap.unknown
}

function Get-ErrorPlaybookHint {
    param([string]$Code)
    $p = "scripts/config/hc_error_playbook.json"
    if (-not (Test-Path $p)) { return "" }
    try {
        $obj = Get-HcJsonFile -Path $p
        if ($obj.PSObject.Properties.Name.Contains($Code)) {
            return [string]$obj.$Code
        }
    } catch {}
    return ""
}

function Initialize-RunManifest {
    $script:RunManifestPath = "reports/tooling/run_manifest_$($script:RunId).json"
    $manifest = [ordered]@{
        run_id = $script:RunId
        command = $Command
        profile = $Profile
        start_utc = $script:RunStartUtc.ToString("o")
        end_utc = $null
        status = "running"
        exit_code = $null
        script_version = ""
        output_paths = @{}
    }
    Save-HcJsonFile -Object $manifest -Path $script:RunManifestPath
}

function Complete-RunManifest {
    param([string]$Status, [int]$ExitCode = 0)
    if (-not $script:RunManifestPath) { return }
    $existing = Get-HcJsonFile -Path $script:RunManifestPath
    $existing.end_utc = [DateTime]::UtcNow.ToString("o")
    $existing.status = $Status
    $existing.exit_code = $ExitCode
    $existing.script_version = if ($script:Settings) { $script:Settings.script_version } else { "" }
    $existing.output_paths = [ordered]@{
        doctor = if ($script:Settings) { $script:Settings.doctor_report_path } else { "" }
        health = if ($script:Settings) { $script:Settings.health_report_path } else { "" }
        telemetry = if ($script:Settings) { $script:Settings.telemetry_jsonl_path } else { "" }
        dashboard = $DashboardPath
        triage = $TriagePath
        diagnostics = $DiagnosticsZipPath
    }
    Save-HcJsonFile -Object $existing -Path $script:RunManifestPath
}

function Acquire-RunLock {
    param($Settings)
    if ($NoLock) { return }
    $lockPath = $script:RunLockPath
    $lockDir = Split-Path -Parent $lockPath
    if ($lockDir -and -not (Test-Path $lockDir)) { New-Item -ItemType Directory -Force -Path $lockDir | Out-Null }

    if (Test-Path $lockPath) {
        $canReuse = $false
        try {
            $existing = Get-HcJsonFile -Path $lockPath
            $ts = [DateTime]::Parse([string]$existing.timestamp_utc)
            $isStale = (([DateTime]::UtcNow - $ts).TotalMinutes -gt [double]$Settings.lock_ttl_min)
            $existingPid = 0
            if ($existing.PSObject.Properties.Name.Contains("pid")) {
                $existingPid = [int]$existing.pid
            }
            $alive = $false
            if ($existingPid -gt 0) {
                $proc = Get-Process -Id $existingPid -ErrorAction SilentlyContinue
                if ($null -ne $proc) { $alive = $true }
            }
            $sameMachine = $true
            if ($existing.PSObject.Properties.Name.Contains("machine_id")) {
                $sameMachine = ([string]$existing.machine_id -eq [string]$env:COMPUTERNAME)
            }
            $sameStart = $false
            if ($alive -and $existing.PSObject.Properties.Name.Contains("process_start_utc")) {
                try {
                    $procNow = Get-Process -Id $existingPid -ErrorAction SilentlyContinue
                    if ($procNow) {
                        $ps = $procNow.StartTime.ToUniversalTime().ToString("o")
                        $sameStart = ($ps -eq [string]$existing.process_start_utc)
                    }
                } catch {}
            }
            if ($isStale -or -not $alive -or -not $sameMachine -or -not $sameStart) {
                $canReuse = $true
            }
        } catch {
            $canReuse = $true
        }
        if ($canReuse) {
            Remove-Item -Force -Path $lockPath -ErrorAction SilentlyContinue
        } else {
            Fail-Hc -Code "lock_conflict" -Message ("lock file exists: {0}" -f $lockPath)
        }
    }

    $payload = [ordered]@{
        timestamp_utc = [DateTime]::UtcNow.ToString("o")
        run_id = $script:RunId
        pid = $PID
        process_start_utc = (Get-Process -Id $PID).StartTime.ToUniversalTime().ToString("o")
        machine_id = [string]$env:COMPUTERNAME
        command = $Command
        profile = $Profile
    }
    Save-HcJsonFile -Object $payload -Path $lockPath
}

function Release-RunLock {
    if ($NoLock) { return }
    if (Test-Path $script:RunLockPath) {
        try { Remove-Item -Force -Path $script:RunLockPath } catch {}
    }
}

function Invoke-External {
    param(
        [string]$FilePath,
        [string[]]$Arguments
    )
    if ($DryRun) {
        Write-HcMsg "step_dry_run_exec" @($FilePath, ($Arguments -join " "))
        return
    }
    $retries = if ($script:Settings) { [int]$script:Settings.retry_count } else { 2 }
    $delay = if ($script:Settings) { [int]$script:Settings.retry_delay_sec } else { 2 }
    $jitter = if ($script:Settings) { [int]$script:Settings.retry_jitter_ms } else { 150 }
    Invoke-HcWithRetry -RetryCount $retries -DelaySeconds $delay -Action {
        if ($jitter -gt 0) { Start-Sleep -Milliseconds (Get-Random -Minimum 0 -Maximum $jitter) }
        Invoke-HcChecked $FilePath $Arguments
    }
}

function Invoke-HcPowerShellFile {
    param(
        [string]$ScriptPath,
        [string[]]$Arguments = @()
    )
    $args = @("-ExecutionPolicy","Bypass","-File",$ScriptPath) + $Arguments
    Invoke-External $script:ShellExe $args
}

function Invoke-HcPowerShellCommand {
    param([string]$InlineCommand)
    $args = @("-NoProfile","-ExecutionPolicy","Bypass","-Command",$InlineCommand)
    Invoke-External $script:ShellExe $args
}

function Classify-Failure {
    param([string]$Message)
    $py = Resolve-HcPython
    try {
        $raw = & $py "scripts/tools/classify_failure.py" --message $Message
        return ($raw | ConvertFrom-Json)
    } catch {
        return [pscustomobject]@{
            category = "unknown"
            severity = "medium"
            hint = "Inspect logs"
        }
    }
}

function Invoke-Op {
    param(
        [string]$OpName,
        [scriptblock]$Action
    )
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    Add-TelemetryEvent -Event "$OpName.start" -Status "start" -Component "operation"
    try {
        & $Action
        $sw.Stop()
        Add-TelemetryEvent -Event "$OpName.end" -Status "ok" -Component "operation" -DurationMs $sw.Elapsed.TotalMilliseconds
    } catch {
        $sw.Stop()
        $msg = [string]$_
        $classified = Classify-Failure -Message $msg
        Add-TelemetryEvent -Event "$OpName.end" -Status "fail" -Data @{
            message = $msg
            category = $classified.category
            severity = $classified.severity
            hint = $classified.hint
        } -Component "operation" -Code "operation_failed" -Level "error" -DurationMs $sw.Elapsed.TotalMilliseconds
        Write-HcMsg "op_failure_category" @($classified.category, $classified.severity)
        Write-HcMsg "op_failure_suggestion" @($classified.hint)
        Write-HcMsg "op_duration_ms" @([math]::Round($sw.Elapsed.TotalMilliseconds,2))
        throw
    }
}

function Get-ToolVersion {
    param([string]$Cmd, [string[]]$CmdArgs = @("--version"))
    if (-not (Test-HcCommand $Cmd)) { return "" }
    try { return [string]((& $Cmd @CmdArgs 2>&1 | Select-Object -First 1)) } catch { return "" }
}

function Get-ToolVersionOrPath {
    param([string]$Cmd, [string]$FallbackPath, [string[]]$CmdArgs = @("--version"))
    $v = Get-ToolVersion -Cmd $Cmd -CmdArgs $CmdArgs
    if ($v) { return $v }
    if ($FallbackPath -and (Test-Path $FallbackPath)) {
        try { return [string]((& $FallbackPath @CmdArgs 2>&1 | Select-Object -First 1)) } catch { return "" }
    }
    return ""
}

function New-DoctorReport {
    param($Settings)
    $limineDir = $Settings.limine_bin_dir
    $shimDir = $Settings.shim_bin_dir
    $checks = [ordered]@{
        python = (Test-HcCommand "python") -or (Test-HcCommand "py")
        git = Test-HcCommand "git"
        rustup = Test-HcCommand "rustup"
        cargo = Test-HcCommand "cargo"
        npm = Test-HcCommand "npm"
        deno = Test-HcCommand "deno"
        qemu = (Test-HcCommand "qemu-system-x86_64") -or (Test-Path "$env:ProgramFiles\qemu\qemu-system-x86_64.exe")
        xorriso = (Test-HcCommand "xorriso") -or (Test-Path "C:\msys64\usr\bin\xorriso.exe")
        limine_bin_dir = Test-Path $limineDir
        limine_bios_cd = Test-Path (Join-Path $limineDir "limine-bios-cd.bin")
        limine_uefi_cd = Test-Path (Join-Path $limineDir "limine-uefi-cd.bin")
        limine_bootx64 = Test-Path (Join-Path $limineDir "BOOTX64.EFI")
    }
    if ([bool]$Settings.shim_mode_enabled) {
        $checks.shim_mode_enabled = $true
        $checks.shim_bin_dir = Test-Path $shimDir
        $checks.shimx64 = Test-Path (Join-Path $shimDir "shimx64.efi")
        $checks.mmx64 = Test-Path (Join-Path $shimDir "mmx64.efi")
    }
    $versions = [ordered]@{
        rustc = Get-ToolVersion -Cmd "rustc" -CmdArgs @("-V")
        cargo = Get-ToolVersion -Cmd "cargo" -CmdArgs @("-V")
        python = if (Test-HcCommand "python") { Get-ToolVersion -Cmd "python" -CmdArgs @("--version") } else { "" }
        git = Get-ToolVersion -Cmd "git" -CmdArgs @("--version")
        node = Get-ToolVersion -Cmd "node" -CmdArgs @("--version")
        npm = Get-ToolVersion -Cmd "npm" -CmdArgs @("--version")
        deno = Get-ToolVersion -Cmd "deno" -CmdArgs @("--version")
        qemu = Get-ToolVersionOrPath -Cmd "qemu-system-x86_64" -FallbackPath "$env:ProgramFiles\qemu\qemu-system-x86_64.exe" -CmdArgs @("--version")
        xorriso = Get-ToolVersionOrPath -Cmd "xorriso" -FallbackPath "C:\msys64\usr\bin\xorriso.exe" -CmdArgs @("-version")
    }
    $ok = $true
    foreach ($kv in $checks.GetEnumerator()) {
        $state = if ($kv.Value) { "OK" } else { "MISSING" }
        Write-Host ("  {0,-16} : {1}" -f $kv.Key, $state)
        if (-not $kv.Value) { $ok = $false }
    }
    return [ordered]@{
        timestamp_utc = [DateTime]::UtcNow.ToString("o")
        profile = $Profile
        ok = $ok
        checks = $checks
        versions = $versions
    }
}

function Run-Doctor {
    param($Settings)
    $report = $null
    if ($UseCache) {
        $cached = Get-HcCache -Key ("doctor.{0}" -f $Profile) -TtlSec $Settings.cache_ttl_sec
        if ($cached) {
            $report = $cached
            Write-HcMsg "doctor_cache_hit"
        }
    }
    if ($null -eq $report) {
        $report = New-DoctorReport -Settings $Settings
        Set-HcCache -Key ("doctor.{0}" -f $Profile) -Payload $report
    }
    if ($WriteDoctorReport -or $Command -in @("report", "health")) {
        Save-HcJsonFile -Object $report -Path $Settings.doctor_report_path
        Write-HcMsg "doctor_report_written" @($Settings.doctor_report_path)
    }
    if (-not $report.ok -and $Profile -eq "strict") { throw "doctor strict mode failed" }
}

function Run-Install {
    param($Cfg, $Settings)
    $inst = $Cfg.install
    Invoke-Op -OpName "install.rust" -Action {
        $args = @()
        if ($Offline) { $args += "-Offline" }
        Invoke-HcPowerShellFile ".\scripts\setup\setup_rust.ps1" $args
    }
    Invoke-Op -OpName "install.tools" -Action {
        $bootArgs = @()
        if ($Offline) { $bootArgs += "-Offline" }
        if (-not [bool]$inst.python) { $bootArgs += "-InstallPython:`$false" }
        if (-not [bool]$inst.git) { $bootArgs += "-InstallGit:`$false" }
        if (-not [bool]$inst.qemu) { $bootArgs += "-InstallQemu:`$false" }
        if (-not [bool]$inst.node) { $bootArgs += "-InstallNode:`$false" }
        if (-not [bool]$inst.deno) { $bootArgs += "-InstallDeno:`$false" }
        if (-not [bool]$inst.msys2) { $bootArgs += "-InstallMsys2:`$false" }
        if (-not [bool]$inst.xorriso) { $bootArgs += "-InstallXorriso:`$false" }
        if (-not [bool]$inst.msys2_deps) { $bootArgs += "-InstallMsys2Deps:`$false" }
        if (-not [bool]$inst.add_msys_to_user_path) { $bootArgs += "-AddMsysToUserPath:`$false" }
        if (-not [bool]$inst.add_cargo_to_user_path) { $bootArgs += "-AddCargoToUserPath:`$false" }
        Invoke-HcPowerShellFile ".\scripts\setup\setup_boot_tools.ps1" $bootArgs
    }
    Invoke-Op -OpName "install.limine" -Action {
        $limineArgs = @()
        if ($InstallWslDeps) { $limineArgs += "-InstallWslDeps" }
        if ($Offline -or $UseCache) { $limineArgs += "-Offline" }
        Invoke-HcPowerShellFile ".\scripts\setup\setup_limine.ps1" $limineArgs
    }
    if ($Settings.shim_mode_enabled) {
        Invoke-Op -OpName "install.shim.validate" -Action {
            Invoke-HcPowerShellFile ".\scripts\setup\setup_shim.ps1" @("-ShimDir", $Settings.shim_bin_dir)
        }
    }

    $depReport = [ordered]@{
        timestamp_utc = [DateTime]::UtcNow.ToString("o")
        offline = [bool]$Offline
        use_cache = [bool]$UseCache
        profile = $Profile
        run_id = $script:RunId
        doctor = New-DoctorReport -Settings $Settings
    }
    Save-HcJsonFile -Object $depReport -Path "reports/tooling/dependency_report.json"
    Write-HcMsg "dependency_report_written" @("reports/tooling/dependency_report.json")
}

function Run-BuildIso {
    $py = Resolve-HcPython
    $args = @(
        "scripts/build_boot_image.py",
        "--profile","release",
        "--build-iso",
        "--auto-fetch-limine",
        "--allow-build-limine",
        "--allow-wsl-build-limine"
    )
    if ($Deterministic) { $args += @("--seed","$Seed") }
    if ($Offline -or $UseCache) { $args += @("--limine-version","latest") }
    if ($script:Settings.shim_mode_enabled) {
        $args += @("--shim-bin-dir", $script:Settings.shim_bin_dir)
        if ($script:Settings.shim_chainloader) { $args += @("--shim-chainloader", $script:Settings.shim_chainloader) }
        if ($script:Settings.grub_limine_target) { $args += @("--grub-limine-target", $script:Settings.grub_limine_target) }
        if ($script:Settings.write_grub_fallback) { $args += "--write-grub-fallback" }
    }
    if ($script:Settings.cargo_no_default_features) {
        $args += "--cargo-no-default-features"
    }
    $featureCsv = [string]$script:Settings.cargo_features
    if (-not [string]::IsNullOrWhiteSpace($featureCsv)) {
        $args += @("--cargo-features", $featureCsv.Trim())
    }
    Invoke-External $py $args
}

function Run-QemuSmoke {
    param($Settings)
    $py = Resolve-HcPython
    $effectiveOut = if ($OutDir) { $OutDir } else { $Settings.qemu_default_out_dir }
    $args = @(
        "scripts/qemu_soak_matrix.py",
        "--boot-mode","iso",
        "--build-iso",
        "--auto-fetch-limine",
        "--rounds","$($Settings.rounds)",
        "--memory-mb","$($Settings.memory_mb)",
        "--cores","$($Settings.cores)",
        "--round-timeout-sec","$($Settings.round_timeout_sec)",
        "--chaos-rate","$($Settings.chaos_rate)",
        "--out-dir","$effectiveOut"
    )
    if ($Deterministic) { $args += @("--seed","$Seed") }
    if ($Settings.allow_timeout_success) { $args += "--allow-timeout-success" }
    if ($Offline -or $UseCache) { $args += @("--limine-version","latest") }
    if ($Settings.shim_mode_enabled) {
        $args += @("--shim-bin-dir", $Settings.shim_bin_dir)
        if ($Settings.shim_chainloader) { $args += @("--shim-chainloader", $Settings.shim_chainloader) }
        if ($Settings.grub_limine_target) { $args += @("--grub-limine-target", $Settings.grub_limine_target) }
        if ($Settings.write_grub_fallback) { $args += "--write-grub-fallback" }
    }
    if ($Settings.cargo_no_default_features) {
        $args += "--cargo-no-default-features"
    }
    $featureCsv = [string]$Settings.cargo_features
    if (-not [string]::IsNullOrWhiteSpace($featureCsv)) {
        $args += @("--cargo-features", $featureCsv.Trim())
    }
    Invoke-External $py $args
}

function Run-QemuLive {
    $isoPath = "artifacts/boot_image/hypercore.iso"
    if (-not (Test-Path $isoPath)) {
        Write-HcMsg "iso_missing_building"
        Run-BuildIso
    }
    if (-not (Test-Path $isoPath)) {
        Fail-Hc -Code "dependency_missing" -Message "ISO missing after build: $isoPath"
    }

    $qemuBin = ""
    if (Test-HcCommand "qemu-system-x86_64") {
        $qemuBin = "qemu-system-x86_64"
    } elseif (Test-Path "$env:ProgramFiles\qemu\qemu-system-x86_64.exe") {
        $qemuBin = "$env:ProgramFiles\qemu\qemu-system-x86_64.exe"
    } else {
        Fail-Hc -Code "dependency_missing" -Message "qemu-system-x86_64 not found"
    }

    $args = @(
        "-m", "1024",
        "-smp", "2",
        "-cdrom", $isoPath,
        "-boot", "d"
    )
    Write-HcMsg "qemu_launch_iso" @($isoPath)
    Invoke-External $qemuBin $args
}

function Run-DashboardAgent {
    param($Settings, [switch]$UnsafeNoAuth)
    $scriptPath = ".\scripts\dashboard_agent.ps1"
    if (-not (Test-Path $scriptPath)) {
        Fail-Hc -Code "dependency_missing" -Message "dashboard agent script missing: $scriptPath"
    }
    $args = @("-Profile", $Profile, "-ConfigPath", $ConfigPath, "-Port", "$($Settings.agent_port)", "-MaxConcurrency", "$($Settings.agent_max_concurrency)", "-MaxQueue", "$($Settings.agent_max_queue)", "-LogRetentionDays", "$($Settings.agent_log_retention_days)")
    if ($UnsafeNoAuth) { $args += "-NoSafe" }
    if ($NoLock) { $args += "-NoLock" }
    Invoke-HcPowerShellFile $scriptPath $args
}

function Test-AgentReachable {
    param([int]$Port)
    try {
        $uri = "http://127.0.0.1:$Port/health"
        $resp = Invoke-WebRequest -Uri $uri -Method GET -TimeoutSec 2 -UseBasicParsing -ErrorAction Stop
        if ([int]$resp.StatusCode -ne 200) { return $false }
        return $true
    } catch {
        return $false
    }
}

function Run-DashboardAgentDetached {
    param($Settings, [switch]$UnsafeNoAuth)

    $scriptPath = ".\scripts\dashboard_agent.ps1"
    if (-not (Test-Path $scriptPath)) {
        Fail-Hc -Code "dependency_missing" -Message "dashboard agent script missing: $scriptPath"
    }

    $port = [int]$Settings.agent_port
    if (Test-AgentReachable -Port $port) {
        Write-Host ("[dashboard-agent-bg] already running on http://127.0.0.1:{0}" -f $port) -ForegroundColor DarkGray
        return
    }

    $runtimeDir = "reports/tooling/agent_runtime"
    New-Item -ItemType Directory -Path $runtimeDir -Force | Out-Null
    $stamp = (Get-Date).ToString("yyyyMMdd_HHmmss")
    $outLog = Join-Path $runtimeDir ("dashboard_agent_{0}.out.log" -f $stamp)
    $errLog = Join-Path $runtimeDir ("dashboard_agent_{0}.err.log" -f $stamp)
    $pidPath = Join-Path $runtimeDir "dashboard_agent_bg.json"

    $args = @(
        "-NoProfile", "-ExecutionPolicy", "Bypass",
        "-File", (Resolve-Path $scriptPath).Path,
        "-Profile", $Profile,
        "-ConfigPath", (Resolve-Path $ConfigPath).Path,
        "-Port", "$($Settings.agent_port)",
        "-MaxConcurrency", "$($Settings.agent_max_concurrency)",
        "-MaxQueue", "$($Settings.agent_max_queue)",
        "-LogRetentionDays", "$($Settings.agent_log_retention_days)"
    )
    if ($UnsafeNoAuth) { $args += "-NoSafe" }
    if ($NoLock) { $args += "-NoLock" }

    $proc = Start-Process -FilePath $script:ShellExe -ArgumentList $args -PassThru -WindowStyle Hidden -RedirectStandardOutput $outLog -RedirectStandardError $errLog
    $meta = [ordered]@{
        pid = [int]$proc.Id
        port = $port
        unsafe_no_auth = [bool]$UnsafeNoAuth
        started_utc = [DateTime]::UtcNow.ToString("o")
        out_log = $outLog
        err_log = $errLog
    }
    Save-HcJsonFile -Object $meta -Path $pidPath

    $healthy = $false
    for ($i = 0; $i -lt 30; $i++) {
        if ($proc.HasExited) { break }
        if (Test-AgentReachable -Port $port) { $healthy = $true; break }
        Start-Sleep -Milliseconds 250
    }

    if (-not $healthy) {
        $errTail = if (Test-Path $errLog) { (Get-Content -Path $errLog | Select-Object -Last 30) -join "`n" } else { "" }
        throw "dashboard-agent-bg failed to become healthy on port $port. stderr: $errTail"
    }

    Write-Host ("[dashboard-agent-bg] ready on http://127.0.0.1:{0} (pid={1})" -f $port, $proc.Id) -ForegroundColor Green
    Write-Host ("[dashboard-agent-bg] logs: {0}" -f $outLog) -ForegroundColor DarkGray
}

function Get-Tasks {
    param($Settings)
    $path = $Settings.tasks_path
    if (-not (Test-Path $path)) { throw "tasks file missing: $path" }
    return (Get-Content -Raw -Path $path -Encoding UTF8 | ConvertFrom-Json).tasks
}

function Run-Task {
    param($Settings, [string]$Name)
    $tasks = Get-Tasks -Settings $Settings
    $task = $tasks | Where-Object { $_.name -eq $Name } | Select-Object -First 1
    if ($null -eq $task) { throw "task not found: $Name" }
    Write-HcMsg "task_info" @($task.name, $task.description)
    $baseArgs = @($task.args)
    $profileArgs = @()
    if ($null -ne $task.profile_args) {
        $pp = $task.profile_args.PSObject.Properties[$Profile]
        if ($null -ne $pp) {
            $profileArgs = @($pp.Value)
        }
    }
    $effectiveArgs = @($baseArgs + $profileArgs)

    if ($task.runner -eq "powershell") {
        Invoke-HcPowerShellFile $task.script $effectiveArgs
    } elseif ($task.runner -eq "python") {
        $py = Resolve-HcPython
        $args = @($task.script) + $effectiveArgs
        Invoke-External $py $args
    } else {
        throw "unsupported task runner: $($task.runner)"
    }
}

function Write-TaskGraph {
    param($Settings)
    $tasks = Get-Tasks -Settings $Settings
    $nodes = @()
    $edges = @()
    foreach ($t in $tasks) {
        $nodes += [ordered]@{
            name = [string]$t.name
            description = [string]$t.description
            depends_on = @($t.depends_on)
        }
        foreach ($d in @($t.depends_on)) {
            $edges += [ordered]@{ from = [string]$d; to = [string]$t.name }
        }
    }
    $out = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        nodes = $nodes
        edges = $edges
    }
    Save-HcJsonFile -Object $out -Path "reports/tooling/task_graph.json"
    $md = @("# HyperCore Task Graph", "")
    foreach ($n in $nodes) {
        $deps = if ($n.depends_on.Count -gt 0) { $n.depends_on -join ", " } else { "(none)" }
        $md += "- $($n.name): depends_on=$deps"
    }
    Set-Content -Path "reports/tooling/task_graph.md" -Value ($md -join "`n") -Encoding UTF8
    Write-HcMsg "task_graph_written" @("reports/tooling/task_graph.json")
}

function Resolve-TaskClosure {
    param(
        [array]$Tasks,
        [string[]]$Seeds
    )
    $map = @{}
    foreach ($t in $Tasks) { $map[$t.name] = $t }
    $needed = New-Object System.Collections.Generic.HashSet[string]

    function Add-WithDeps([string]$name) {
        if (-not $map.ContainsKey($name)) { throw "task not found in registry: $name" }
        if ($needed.Contains($name)) { return }
        $needed.Add($name) | Out-Null
        $deps = @($map[$name].depends_on)
        foreach ($d in $deps) { Add-WithDeps $d }
    }

    foreach ($s in $Seeds) { Add-WithDeps $s }
    return @($needed)
}

function Get-TopologicalWaves {
    param(
        [array]$Tasks,
        [string[]]$Selected
    )
    $map = @{}
    foreach ($t in $Tasks) { $map[$t.name] = $t }
    $remaining = New-Object System.Collections.Generic.HashSet[string]
    foreach ($n in $Selected) { $remaining.Add($n) | Out-Null }
    $done = New-Object System.Collections.Generic.HashSet[string]
    $waves = @()

    while ($remaining.Count -gt 0) {
        $ready = @()
        foreach ($n in @($remaining)) {
            $deps = @($map[$n].depends_on)
            $allMet = $true
            foreach ($d in $deps) {
                if ($remaining.Contains($d) -and -not $done.Contains($d)) { $allMet = $false; break }
            }
            if ($allMet) { $ready += $n }
        }
        if ($ready.Count -eq 0) {
            throw "task dependency cycle or invalid graph in selected set: $($Selected -join ', ')"
        }
        $waves += ,($ready | Sort-Object)
        foreach ($r in $ready) {
            $done.Add($r) | Out-Null
            $remaining.Remove($r) | Out-Null
        }
    }

    return $waves
}

function Run-TaskJobWave {
    param(
        [string[]]$Wave,
        [string]$RepoRoot
    )
    $queue = @($Wave)
    $idx = 0
    while ($idx -lt $queue.Count) {
        $waveStart = [System.Diagnostics.Stopwatch]::StartNew()
        $jobs = @()
        $chunk = $queue[$idx..([Math]::Min($idx + $MaxParallel - 1, $queue.Count - 1))]
        Add-TelemetryEvent -Event "gate.wave.start" -Status "start" -Component "scheduler" -Data @{
            queue_total = $queue.Count
            wave_chunk_size = $chunk.Count
            max_parallel = $MaxParallel
            index = $idx
        }
        foreach ($name in $chunk) {
            $argList = @(
                "-ExecutionPolicy","Bypass",
                "-File",".\scripts\hypercore.ps1",
            "-Command","run-task",
            "-TaskName",$name,
            "-Profile",$Profile,
            "-ConfigPath",$ConfigPath
        )
        if ($Offline) { $argList += "-Offline" }
        if ($UseCache) { $argList += "-UseCache" }
        if ($DryRun) { $argList += "-DryRun" }
        $argList += "-NoLock"

            $job = Start-Job -ScriptBlock {
            param($wd, $argsIn, $shellExe)
            Set-Location $wd
            & $shellExe @argsIn
            exit $LASTEXITCODE
        } -ArgumentList $RepoRoot, $argList, $script:ShellExe
            $jobs += $job
        }

        Wait-Job -Job $jobs | Out-Null
        $failed = @()
        foreach ($j in $jobs) {
            Receive-Job -Job $j -Keep
            if ($j.State -ne "Completed") {
                $failed += "job_state:$($j.Name):$($j.State)"
            } elseif ($j.ChildJobs[0].JobStateInfo.Reason) {
                $failed += "job_error:$($j.Name):$($j.ChildJobs[0].JobStateInfo.Reason)"
            }
        }
        Remove-Job -Job $jobs -Force | Out-Null
        if ($failed.Count -gt 0) {
            throw "parallel wave failed: $($failed -join '; ')"
        }
        $waveStart.Stop()
        Add-TelemetryEvent -Event "gate.wave.end" -Status "ok" -Component "scheduler" -DurationMs $waveStart.Elapsed.TotalMilliseconds -Data @{
            wave_chunk_size = $chunk.Count
            queue_remaining = [math]::Max(0, ($queue.Count - ($idx + $chunk.Count)))
        }
        $idx += $MaxParallel
    }
}

function Run-Gate {
    param($Settings, [string]$Stage)
    $tasks = Get-Tasks -Settings $Settings
    $seeds = if ($Stage -eq "all") { @("rc") } else { @($Stage) }
    $selected = Resolve-TaskClosure -Tasks $tasks -Seeds $seeds
    $waves = Get-TopologicalWaves -Tasks $tasks -Selected $selected
    Write-HcMsg "gate_selected_tasks" @($selected -join ", ")

    foreach ($wave in $waves) {
        if ($ParallelGates -and $wave.Count -gt 1 -and -not $DryRun) {
            Write-HcMsg "gate_parallel_wave" @($wave -join ", ")
            Run-TaskJobWave -Wave $wave -RepoRoot $repoRoot
        } else {
            foreach ($name in $wave) {
                Invoke-Op -OpName "gate.$name" -Action { Run-Task -Settings $Settings -Name $name }
            }
        }
    }
}

function Run-Cleanup {
    param($Settings)
    $keep = [int]$Settings.cleanup_keep_latest
    $cutoff = (Get-Date).ToUniversalTime().AddDays(-1 * [int]$Settings.cleanup_keep_days)
    $aggressive = $false
    try {
        $drive = [System.IO.Path]::GetPathRoot((Resolve-Path ".").Path)
        $di = New-Object System.IO.DriveInfo($drive)
        if ($di.TotalSize -gt 0) {
            $usedPct = [int][math]::Round((100.0 * ($di.TotalSize - $di.AvailableFreeSpace) / $di.TotalSize), 0)
            if ($usedPct -ge [int]$Settings.disk_usage_aggressive_cleanup_pct) {
                $aggressive = $true
                Write-HcMsg "cleanup_aggressive" @($usedPct, [int]$Settings.disk_usage_aggressive_cleanup_pct)
            }
        }
    } catch {}
    foreach ($target in @($Settings.cleanup_targets)) {
        if (-not (Test-Path $target)) { continue }
        $children = Get-ChildItem -Path $target -Force | Sort-Object LastWriteTime -Descending
        $drop = @($children | Select-Object -Skip $keep)
        if (-not $aggressive) {
            $drop = @($drop | Where-Object { $_.LastWriteTimeUtc -lt $cutoff })
        }
        foreach ($item in $drop) {
            try {
                Remove-Item -Recurse -Force -Path $item.FullName
            } catch {}
        }
        Write-HcMsg "cleanup_target_stats" @($target, $drop.Count, $keep)
    }

    $totalBytes = 0
    foreach ($target in @($Settings.cleanup_targets)) {
        if (-not (Test-Path $target)) { continue }
        $totalBytes += (Get-ChildItem -Path $target -Recurse -File -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum
    }
    $totalGb = [double]($totalBytes / 1GB)
    if ($totalGb -gt [double]$Settings.cleanup_max_artifacts_gb) {
        Write-HcMsg "cleanup_artifacts_high" @([math]::Round($totalGb,2), [double]$Settings.cleanup_max_artifacts_gb)
    }
}

function Run-Health {
    param($Settings)
    $weights = $Settings.health_weights
    $doctor = if (Test-Path $Settings.doctor_report_path) { Get-HcJsonFile $Settings.doctor_report_path } else { New-DoctorReport -Settings $Settings }
    $qemuBaseDir = if ($OutDir) { $OutDir } else { $Settings.qemu_default_out_dir }
    $qemuSummaryPath = Join-Path $qemuBaseDir "summary.json"
    $qemuOk = $false
    if (Test-Path $qemuSummaryPath) {
        $qemuPayload = Get-HcJsonFile $qemuSummaryPath
        $q = $qemuPayload.summary
        if ($null -ne $q -and [bool]$q.ok) { $qemuOk = $true }
    }
    $p1Path = "reports/p1_ops_gate/summary.json"
    $p1Ok = $false
    if (Test-Path $p1Path) {
        $p1 = Get-HcJsonFile $p1Path
        if ($null -ne $p1.summary -and [bool]$p1.summary.ok) { $p1Ok = $true }
    }
    $rcPath = "reports/release_candidate/verdict.json"
    $rcOk = $false
    if (Test-Path $rcPath) {
        $rc = Get-HcJsonFile $rcPath
        if ([bool]$rc.ready) { $rcOk = $true }
    }

    $score = 0
    if ([bool]$doctor.ok) { $score += [int]$weights.doctor }
    if ($qemuOk) { $score += [int]$weights.qemu_smoke }
    if ($p1Ok) { $score += [int]$weights.p1_gate }
    if ($rcOk) { $score += [int]$weights.rc_verdict }

    $report = [ordered]@{
        timestamp_utc = [DateTime]::UtcNow.ToString("o")
        run_id = $script:RunId
        score = $score
        max_score = ([int]$weights.doctor + [int]$weights.qemu_smoke + [int]$weights.p1_gate + [int]$weights.rc_verdict)
        doctor_ok = [bool]$doctor.ok
        qemu_smoke_ok = $qemuOk
        p1_ok = $p1Ok
        rc_ok = $rcOk
        qemu_summary_path = $qemuSummaryPath
    }
    Save-HcJsonFile -Object $report -Path $Settings.health_report_path
    Write-HcMsg "health_score" @($report.score, $report.max_score)
    Write-HcMsg "health_report_path" @($Settings.health_report_path)
}

function Run-Verify {
    param($Settings)
    Invoke-HcPowerShellFile ".\scripts\release_preflight.ps1"
    Run-QemuSmoke -Settings $Settings
}

function Get-LastTelemetryFailures {
    param($Settings, [int]$Count = 5)
    $p = $Settings.telemetry_jsonl_path
    if (-not (Test-Path $p)) { return @() }
    $lines = Get-Content -Path $p -Tail 500
    $rows = @()
    foreach ($ln in $lines) {
        try { $rows += ($ln | ConvertFrom-Json) } catch {}
    }
    $fails = @($rows | Where-Object { $_.status -eq "fail" } | Select-Object -Last $Count)
    return $fails
}

function Get-RepoSnapshot {
    $status = ""
    $head = ""
    $branch = ""
    try { $status = [string](& git status --short 2>$null | Out-String) } catch {}
    try { $head = [string](& git rev-parse HEAD 2>$null | Out-String).Trim() } catch {}
    try { $branch = [string](& git rev-parse --abbrev-ref HEAD 2>$null | Out-String).Trim() } catch {}
    return [ordered]@{
        git_head = $head
        git_branch = $branch
        git_dirty = [bool]($status -and $status.Trim().Length -gt 0)
        git_status_short = $status
    }
}

function Run-ExplainLastFailure {
    param($Settings)
    $fails = Get-LastTelemetryFailures -Settings $Settings -Count 1
    if ($fails.Count -eq 0) {
        Write-HcMsg "no_failure_found"
        return
    }
    $f = $fails[0]
    $msg = [string]$f.data.message
    $classified = Classify-Failure -Message $msg
    $out = [ordered]@{
        timestamp_utc = [DateTime]::UtcNow.ToString("o")
        source_event = $f.event
        source_status = $f.status
        category = $classified.category
        severity = $classified.severity
        hint = $classified.hint
        message = $msg
    }
    Save-HcJsonFile -Object $out -Path "reports/tooling/last_failure_explained.json"
    Write-HcMsg "failure_category_severity" @($out.category, $out.severity)
    Write-HcMsg "failure_hint" @($out.hint)
}

function Run-Dashboard {
    param($Settings)
    function Resolve-RcVerdictPath {
        param([string]$CurrentProfile)
        $ordered = @()
        if ($CurrentProfile -eq "strict") {
            $ordered += "reports/release_candidate_smoke_strict/verdict.json"
            $ordered += "reports/release_candidate/verdict.json"
            $ordered += "reports/release_candidate_smoke/verdict.json"
        } else {
            $ordered += "reports/release_candidate_smoke/verdict.json"
            $ordered += "reports/release_candidate/verdict.json"
            $ordered += "reports/release_candidate_smoke_strict/verdict.json"
        }
        foreach ($p in $ordered) {
            if (Test-Path $p) { return $p }
        }
        return ""
    }

    $doctor = if (Test-Path $Settings.doctor_report_path) { Get-HcJsonFile $Settings.doctor_report_path } else { @{} }
    $health = if (Test-Path $Settings.health_report_path) { Get-HcJsonFile $Settings.health_report_path } else { @{} }
    $qemuBaseDir = if ($OutDir) { $OutDir } else { $Settings.qemu_default_out_dir }
    $qpath = Join-Path $qemuBaseDir "summary.json"
    $qsum = if (Test-Path $qpath) { Get-HcJsonFile $qpath } else { @{} }
    $p1path = "reports/p1_ops_gate/summary.json"
    $p1 = if (Test-Path $p1path) { Get-HcJsonFile $p1path } else { @{} }
    $rcpath = Resolve-RcVerdictPath -CurrentProfile $Profile
    $rc = if ($rcpath -and (Test-Path $rcpath)) { Get-HcJsonFile $rcpath } else { @{} }
    $fails = Get-LastTelemetryFailures -Settings $Settings -Count $FailureTail

    $payload = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        profile = $Profile
        doctor_ok = [bool]$doctor.ok
        health_score = if ($health.score) { [int]$health.score } else { $null }
        qemu_ok = if ($qsum.summary.ok) { [bool]$qsum.summary.ok } else { $false }
        p1_ok = if ($p1.summary.ok) { [bool]$p1.summary.ok } else { $false }
        rc_ready = if ($rc.ready) { [bool]$rc.ready } else { $false }
        rc_path = $rcpath
        rc_generated_utc = if ($rc.generated_utc) { [string]$rc.generated_utc } else { "" }
        recent_failures = @($fails)
    }
    Save-HcJsonFile -Object $payload -Path "reports/tooling/dashboard.json"

    $md = @(
        "# HyperCore Dashboard",
        "",
        "- generated_utc: $($payload.generated_utc)",
        "- profile: $($payload.profile)",
        "- doctor_ok: $($payload.doctor_ok)",
        "- health_score: $($payload.health_score)",
        "- qemu_ok: $($payload.qemu_ok)",
        "- p1_ok: $($payload.p1_ok)",
        "- rc_ready: $($payload.rc_ready)",
        ""
    )
    if ($fails.Count -gt 0) {
        $md += "## Recent Failures"
        $md += ""
        foreach ($f in $fails) {
            $md += "- event=$($f.event) status=$($f.status) msg=$([string]$f.data.message)"
        }
    }
    $dashDir = Split-Path -Parent $DashboardPath
    if ($dashDir -and -not (Test-Path $dashDir)) { New-Item -ItemType Directory -Force -Path $dashDir | Out-Null }
    Set-Content -Path $DashboardPath -Value ($md -join "`n") -Encoding UTF8
    Write-HcMsg "dashboard_path" @($DashboardPath)
}

function Run-Trends {
    param($Settings)
    $telemetry = $Settings.telemetry_jsonl_path
    if (-not (Test-Path $telemetry)) {
        throw "telemetry file missing: $telemetry"
    }
    $lines = Get-Content -Path $telemetry -Tail ([Math]::Max($TrendWindow, 20) * 20)
    $rows = @()
    foreach ($ln in $lines) { try { $rows += ($ln | ConvertFrom-Json) } catch {} }
    $windowRows = @($rows | Select-Object -Last $TrendWindow)
    $failRows = @($windowRows | Where-Object { $_.status -eq "fail" })
    $failRate = if ($windowRows.Count -gt 0) { [math]::Round((100.0 * $failRows.Count / $windowRows.Count), 2) } else { 0.0 }
    $byEvent = @{}
    foreach ($r in $failRows) {
        $e = [string]$r.event
        if (-not $byEvent.ContainsKey($e)) { $byEvent[$e] = 0 }
        $byEvent[$e] += 1
    }

    $summary = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        trend_window = $TrendWindow
        samples = $windowRows.Count
        failures = $failRows.Count
        failure_rate_pct = $failRate
        failure_by_event = $byEvent
        alert = ($failRate -gt 15.0)
        alert_reason = if ($failRate -gt 15.0) { "failure_rate_pct > 15" } else { "" }
    }
    Save-HcJsonFile -Object $summary -Path "reports/tooling/trends.json"
    $md = @(
        "# HyperCore Trends",
        "",
        "- samples: $($summary.samples)",
        "- failures: $($summary.failures)",
        "- failure_rate_pct: $($summary.failure_rate_pct)",
        "- alert: $($summary.alert)",
        "- alert_reason: $($summary.alert_reason)",
        ""
    )
    Set-Content -Path "reports/tooling/trends.md" -Value ($md -join "`n") -Encoding UTF8
    Write-HcMsg "trends_report_path" @("reports/tooling/trends.json")
}

function Run-CollectDiagnostics {
    param($Settings)
    $envSnapshot = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        profile = $Profile
        command = $Command
        tool_versions = [ordered]@{
            rustc = Get-ToolVersion -Cmd "rustc" -CmdArgs @("-V")
            cargo = Get-ToolVersion -Cmd "cargo" -CmdArgs @("-V")
            python = if (Test-HcCommand "python") { Get-ToolVersion -Cmd "python" -CmdArgs @("--version") } else { "" }
            pwsh = Get-ToolVersionOrPath -Cmd "pwsh" -FallbackPath "" -CmdArgs @("--version")
            qemu = Get-ToolVersionOrPath -Cmd "qemu-system-x86_64" -FallbackPath "$env:ProgramFiles\qemu\qemu-system-x86_64.exe" -CmdArgs @("--version")
            xorriso = Get-ToolVersionOrPath -Cmd "xorriso" -FallbackPath "C:\msys64\usr\bin\xorriso.exe" -CmdArgs @("-version")
        }
        repo = Get-RepoSnapshot
    }
    Save-HcJsonFile -Object $envSnapshot -Path "reports/tooling/env_snapshot.json"

    $items = @(
        "reports/tooling",
        "reports/p1_ops_gate",
        "reports/p2_gap",
        "reports/release_candidate",
        "artifacts/qemu_smoke_easy",
        "artifacts/qemu_smoke_after_full",
        "artifacts/qemu_smoke_continued"
    ) | Where-Object { Test-Path $_ }
    $zipDir = Split-Path -Parent $DiagnosticsZipPath
    if ($zipDir -and -not (Test-Path $zipDir)) { New-Item -ItemType Directory -Force -Path $zipDir | Out-Null }
    if (Test-Path $DiagnosticsZipPath) { Remove-Item -Force $DiagnosticsZipPath }
    if ($items.Count -eq 0) {
        throw "no diagnostic artifacts found to package"
    }
    Compress-Archive -Path $items -DestinationPath $DiagnosticsZipPath -Force
    Write-HcMsg "diagnostics_zip_path" @($DiagnosticsZipPath)
}

function Run-SupportBundle {
    param($Settings)
    Run-CollectDiagnostics -Settings $Settings
    $outDir = "reports/tooling"
    $san = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        machine = [string]$env:COMPUTERNAME
        user = "<redacted>"
        run_manifest = $script:RunManifestPath
    }
    Save-HcJsonFile -Object $san -Path (Join-Path $outDir "support_bundle_metadata.json")
    Write-HcMsg "support_bundle_prepared"
}

function Run-ScriptTests {
    if (-not (Test-HcCommand "pwsh")) {
        Write-HcMsg "pwsh_not_found_fallback"
    }
    if (-not (Get-Module -ListAvailable -Name Pester)) {
        if ($Offline) {
            throw "Pester missing and offline mode enabled"
        }
        Write-HcMsg "installing_pester"
        Invoke-HcPowerShellCommand "Install-Module Pester -Scope CurrentUser -Force -SkipPublisherCheck"
    }
    $testPath = "tests/powershell/Hypercore.Tests.ps1"
    if (-not (Test-Path $testPath)) {
        throw "test file missing: $testPath"
    }
    Invoke-HcPowerShellCommand "`$r = Invoke-Pester -Path '$testPath' -PassThru; if (`$r.FailedCount -gt 0) { exit 1 }"
}

function Run-Triage {
    param($Settings)
    $doctor = New-DoctorReport -Settings $Settings
    Save-HcJsonFile -Object $doctor -Path $Settings.doctor_report_path

    Run-Health -Settings $Settings
    $health = Get-HcJsonFile -Path $Settings.health_report_path

    $trendError = ""
    try { Run-Trends -Settings $Settings } catch { $trendError = [string]$_ }
    $lastFailureError = ""
    try { Run-ExplainLastFailure -Settings $Settings } catch { $lastFailureError = [string]$_ }
    Run-Dashboard -Settings $Settings

    $missing = @()
    foreach ($kv in $doctor.checks.PSObject.Properties) {
        if (-not [bool]$kv.Value) { $missing += $kv.Name }
    }
    $summary = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        profile = $Profile
        doctor_ok = [bool]$doctor.ok
        health_score = [int]$health.score
        health_max_score = [int]$health.max_score
        missing_checks = @($missing)
        trends_error = $trendError
        last_failure_error = $lastFailureError
        dashboard_path = $DashboardPath
    }
    Save-HcJsonFile -Object $summary -Path "reports/tooling/triage.json"

    $md = @(
        "# HyperCore Triage",
        "",
        "- generated_utc: $($summary.generated_utc)",
        "- profile: $($summary.profile)",
        "- doctor_ok: $($summary.doctor_ok)",
        "- health_score: $($summary.health_score)/$($summary.health_max_score)",
        "- missing_checks: $((@($summary.missing_checks) -join ', '))",
        "- dashboard_path: $($summary.dashboard_path)",
        "",
        "## Suggested Next Commands",
        "",
        "1. powershell -ExecutionPolicy Bypass -File .\scripts\hypercore.ps1 -Command install",
        "2. powershell -ExecutionPolicy Bypass -File .\scripts\hypercore.ps1 -Command ci-smoke -Profile quick",
        "3. powershell -ExecutionPolicy Bypass -File .\scripts\hypercore.ps1 -Command health",
        "4. powershell -ExecutionPolicy Bypass -File .\scripts\hypercore.ps1 -Command collect-diagnostics"
    )
    $triageDir = Split-Path -Parent $TriagePath
    if ($triageDir -and -not (Test-Path $triageDir)) { New-Item -ItemType Directory -Force -Path $triageDir | Out-Null }
    Set-Content -Path $TriagePath -Value ($md -join "`n") -Encoding UTF8
    Write-HcMsg "triage_report_path" @($TriagePath)
}

function Run-PreRelease {
    param($Cfg, $Settings)
    Run-Doctor -Settings $Settings
    Run-Gate -Settings $Settings -Stage "all"
    Run-Health -Settings $Settings
    Run-Triage -Settings $Settings
    Run-CollectDiagnostics -Settings $Settings
}

function Escape-Html {
    param([string]$Text)
    if ($null -eq $Text) { return "" }
    return [System.Net.WebUtility]::HtmlEncode($Text)
}

function Run-HtmlDashboard {
    param($Settings)
    function Get-JsonMaybe([string]$Path) {
        if (Test-Path $Path) { return (Get-HcJsonFile -Path $Path) }
        return $null
    }
    function Get-ReportAgeHours([string]$Path) {
        if (-not (Test-Path $Path)) { return 999999.0 }
        try {
            return [math]::Round(([DateTime]::UtcNow - (Get-Item $Path).LastWriteTimeUtc).TotalHours, 2)
        } catch {
            return 999999.0
        }
    }
    # Always refresh dashboard snapshot so RC/health states are current.
    Run-Dashboard -Settings $Settings
    if (-not (Test-Path "reports/tooling/trends.json")) { Run-Trends -Settings $Settings }
    if (-not (Test-Path "reports/tooling/health_report.json")) { Run-Health -Settings $Settings }
    if (-not (Test-Path "reports/tooling/anomaly_report.json") -or ((Get-ReportAgeHours -Path "reports/tooling/anomaly_report.json") -gt 8.0)) {
        Run-AnomalyReport -Settings $Settings
    }

    $dash = Get-HcJsonFile -Path "reports/tooling/dashboard.json"
    $tr = Get-HcJsonFile -Path "reports/tooling/trends.json"
    $health = Get-JsonMaybe "reports/tooling/health_report.json"
    $doctor = Get-JsonMaybe "reports/tooling/doctor_report.json"
    $anomaly = Get-JsonMaybe "reports/tooling/anomaly_report.json"
    $policy = Get-JsonMaybe "reports/tooling/policy_gate.json"
    $drift = Get-JsonMaybe "reports/tooling/dependency_drift.json"
    $flaky = Get-JsonMaybe "reports/tooling/flaky_report.json"
    $schema = Get-JsonMaybe "reports/tooling/schema_validation.json"
    $artifactVerify = Get-JsonMaybe "reports/tooling/artifact_verify.json"
    $i18nLint = Get-JsonMaybe "reports/tooling/i18n_lint.json"
    $updateCheck = Get-JsonMaybe "reports/tooling/update_check.json"

    $telemetryRows = @()
    if (Test-Path $Settings.telemetry_jsonl_path) {
        foreach ($ln in (Get-Content -Path $Settings.telemetry_jsonl_path -Tail 1200)) {
            try { $telemetryRows += ($ln | ConvertFrom-Json) } catch {}
        }
    }
    $opDur = @($telemetryRows | Where-Object { $_.event -like "*.end" -and $_.duration_ms -gt 0 } | Sort-Object duration_ms -Descending | Select-Object -First 12)
    $recentRunEnds = @($telemetryRows | Where-Object { $_.event -eq "run.end" } | Select-Object -Last 40)
    $fails = @($dash.recent_failures)

    $statusClass = @{
        good = "ok"
        warn = "warn"
        bad = "bad"
    }
    $doctorOk = [bool]$dash.doctor_ok
    $qemuOk = [bool]$dash.qemu_ok
    $rcReady = [bool]$dash.rc_ready
    $policyOk = if ($policy) { [bool]$policy.ok } else { $false }
    $schemaOk = if ($schema) { [bool]$schema.ok } else { $false }
    $artifactOk = if ($artifactVerify) { [bool]$artifactVerify.ok } else { $false }
    $anomalyAgeHours = Get-ReportAgeHours -Path "reports/tooling/anomaly_report.json"
    $anomalyFresh = ($anomalyAgeHours -le 8.0)
    $anomalyAlert = if ($anomaly -and $anomalyFresh) { [bool]$anomaly.alert } else { $false }
    $driftCount = if ($drift) { [int]$drift.drift_count } else { 0 }
    $flakyCommands = 0
    if ($flaky -and $flaky.rows) {
        $flakyCommands = @($flaky.rows | Where-Object { [bool]$_.flaky }).Count
    }
    $healthScore = if ($health) { [int]$health.score } elseif ($dash.health_score) { [int]$dash.health_score } else { 0 }
    $healthMax = if ($health) { [int]$health.max_score } else { 100 }
    $healthPct = if ($healthMax -gt 0) { [math]::Round((100.0 * $healthScore / $healthMax), 1) } else { 0.0 }

    $actions = New-Object System.Collections.Generic.List[string]
    if (-not $doctorOk) { $actions.Add("[HIGH] Run: powershell -ExecutionPolicy Bypass -File scripts/hypercore.ps1 -Command doctor-fix -AutoApprove") }
    if (-not $qemuOk) { $actions.Add("[HIGH] Run: powershell -ExecutionPolicy Bypass -File scripts/hypercore.ps1 -Command qemu-smoke -NoLock") }
    if ($anomalyAlert) { $actions.Add("[HIGH] Run: powershell -ExecutionPolicy Bypass -File scripts/hypercore.ps1 -Command anomaly-report -NoLock and inspect top failing commands") }
    if ($driftCount -gt 0) { $actions.Add("[MED] Run: powershell -ExecutionPolicy Bypass -File scripts/hypercore.ps1 -Command dependency-drift -NoLock") }
    if (-not $schemaOk) { $actions.Add("[HIGH] Run: powershell -ExecutionPolicy Bypass -File scripts/hypercore.ps1 -Command validate-schemas -NoLock") }
    if (-not $policyOk) { $actions.Add("[HIGH] Run: powershell -ExecutionPolicy Bypass -File scripts/hypercore.ps1 -Command policy-gate -NoLock") }
    if ($flakyCommands -gt 0) { $actions.Add("[MED] Run: powershell -ExecutionPolicy Bypass -File scripts/hypercore.ps1 -Command detect-flaky -NoLock") }
    if ($actions.Count -eq 0) { $actions.Add("[OK] No critical blockers. Suggested: run pre-release gate for final confidence.") }

    $rows = @()
    foreach ($f in $fails) {
        $rows += "<tr><td>$(Escape-Html ([string]$f.event))</td><td>$(Escape-Html ([string]$f.status))</td><td>$(Escape-Html ([string]$f.data.message))</td></tr>"
    }
    if ($rows.Count -eq 0) { $rows += "<tr><td colspan='3' class='muted'>No recent failures in telemetry window.</td></tr>" }

    $durRows = @()
    foreach ($d in $opDur) {
        $durRows += "<tr><td>$(Escape-Html ([string]$d.event))</td><td>$(Escape-Html ([string]([math]::Round([double]$d.duration_ms,2))))</td><td>$(Escape-Html ([string]$d.status))</td></tr>"
    }
    if ($durRows.Count -eq 0) { $durRows += "<tr><td colspan='3' class='muted'>No duration telemetry found.</td></tr>" }

    $timelineRows = @()
    foreach ($r in $recentRunEnds) {
        $timelineRows += "<tr><td>$(Escape-Html ([string]$r.timestamp_utc))</td><td>$(Escape-Html ([string]$r.command))</td><td>$(Escape-Html ([string]$r.status))</td><td>$(Escape-Html ([string]$r.code))</td></tr>"
    }
    if ($timelineRows.Count -eq 0) { $timelineRows += "<tr><td colspan='4' class='muted'>No run timeline records.</td></tr>" }

    $actionRows = @()
    foreach ($a in $actions) { $actionRows += "<li><code>$(Escape-Html $a)</code></li>" }

    $summaryRows = @(
        @{ name = "Doctor"; value = $doctorOk; detail = "Host dependency readiness" },
        @{ name = "QEMU Smoke"; value = $qemuOk; detail = "Boot smoke verification" },
        @{ name = "RC Ready"; value = $rcReady; detail = "Release candidate readiness" },
        @{ name = "Policy Gate"; value = $policyOk; detail = "Policy-as-code pass/fail" },
        @{ name = "Schema Validation"; value = $schemaOk; detail = "Report JSON integrity" },
        @{ name = "Artifact Verify"; value = $artifactOk; detail = "Artifact checksum validation" },
        @{ name = "Anomaly Alert"; value = (-not $anomalyAlert); detail = "No active anomaly alert" }
    )
    $summaryTableRows = @()
    foreach ($s in $summaryRows) {
        $class = if ([bool]$s.value) { "ok" } else { "bad" }
        $txt = if ([bool]$s.value) { "PASS" } else { "FAIL" }
        $summaryTableRows += "<tr><td>$(Escape-Html $s.name)</td><td class='$class'>$txt</td><td>$(Escape-Html $s.detail)</td></tr>"
    }

    $failByEventLabels = @()
    $failByEventValues = @()
    if ($tr.failure_by_event) {
        foreach ($p in $tr.failure_by_event.PSObject.Properties) {
            $failByEventLabels += [string]$p.Name
            $failByEventValues += [int]$p.Value
        }
    }
    if ($failByEventLabels.Count -eq 0) {
        $byEvt = @{}
        foreach ($f in @($fails)) {
            $ev = [string]$f.event
            if (-not $ev) { $ev = "unknown" }
            if (-not $byEvt.ContainsKey($ev)) { $byEvt[$ev] = 0 }
            $byEvt[$ev] += 1
        }
        if ($byEvt.Count -gt 0) {
            foreach ($k in ($byEvt.Keys | Sort-Object)) {
                $failByEventLabels += [string]$k
                $failByEventValues += [int]$byEvt[$k]
            }
        }
    }
    if ($failByEventLabels.Count -eq 0) {
        $failByEventLabels = @("none")
        $failByEventValues = @(0)
    }

    $failLabelJson = "[" + ((@($failByEventLabels) | ForEach-Object {
        '"' + (([string]$_).Replace('\', '\\').Replace('"', '\"')) + '"'
    }) -join ",") + "]"
    $failValueJson = "[" + ((@($failByEventValues) | ForEach-Object { [string]([int]$_) }) -join ",") + "]"

    $detailDash = Escape-Html (($dash | ConvertTo-Json -Depth 12))
    $detailTrend = Escape-Html (($tr | ConvertTo-Json -Depth 12))
    $detailHealth = Escape-Html $(if ($health) { $health | ConvertTo-Json -Depth 12 } else { "{}" })
    $detailAnomaly = Escape-Html $(if ($anomaly) { $anomaly | ConvertTo-Json -Depth 12 } else { "{}" })
    $detailPolicy = Escape-Html $(if ($policy) { $policy | ConvertTo-Json -Depth 12 } else { "{}" })

    $html = @"
<!doctype html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>HyperCore Dashboard</title>
<style>
:root{
  --bg1:#0b1220;--bg2:#18243a;--card:#0f172acc;--text:#e5e7eb;--muted:#94a3b8;
  --ok:#10b981;--warn:#f59e0b;--bad:#ef4444;--line:#334155;--blue:#38bdf8;
}
*{box-sizing:border-box}
body{
  margin:0;padding:24px;font-family:"Segoe UI",system-ui,sans-serif;color:var(--text);
  background:radial-gradient(circle at 15% 20%, #1d4ed8 0%, transparent 40%),
             radial-gradient(circle at 85% 15%, #0891b2 0%, transparent 35%),
             linear-gradient(160deg,var(--bg1),var(--bg2));
}
.wrap{max-width:1400px;margin:0 auto}
.head{display:flex;justify-content:space-between;align-items:end;gap:16px;margin-bottom:18px}
.title{font-size:32px;font-weight:700;letter-spacing:.3px}
.meta{color:var(--muted);font-size:13px}
.grid{display:grid;grid-template-columns:repeat(5,minmax(0,1fr));gap:12px}
.card{
  background:var(--card);backdrop-filter:blur(8px);border:1px solid #1f2a44;border-radius:14px;padding:14px;
  box-shadow:0 8px 24px #02061766;
}
.k{font-size:12px;color:var(--muted)}
.v{font-size:24px;font-weight:700;margin-top:4px}
.ok{color:var(--ok)} .warn{color:var(--warn)} .bad{color:var(--bad)}
.row{display:grid;grid-template-columns:1.4fr 1fr;gap:12px;margin-top:12px}
table{width:100%;border-collapse:collapse}
th,td{padding:8px 10px;border-bottom:1px solid var(--line);font-size:13px;text-align:left;vertical-align:top}
th{color:var(--muted);font-weight:600}
canvas{width:100%;height:220px;background:#02061766;border:1px solid var(--line);border-radius:10px}
.badge{padding:3px 8px;border-radius:999px;font-size:11px;font-weight:700;display:inline-block}
.b-ok{background:#065f46;color:#a7f3d0}.b-warn{background:#78350f;color:#fde68a}.b-bad{background:#7f1d1d;color:#fecaca}
.muted{color:var(--muted)}
details{margin-top:10px;border:1px solid var(--line);border-radius:10px;padding:8px;background:#02061755}
summary{cursor:pointer;color:#cbd5e1}
pre{white-space:pre-wrap;word-break:break-word;color:#cbd5e1;font-size:12px}
ul{margin:8px 0 0 18px}
.legend{display:flex;gap:10px;flex-wrap:wrap;margin-top:6px}
.subgrid{display:grid;grid-template-columns:1fr 1fr;gap:12px}
@media (max-width:1200px){.grid{grid-template-columns:repeat(3,minmax(0,1fr))}}
@media (max-width:980px){.grid{grid-template-columns:repeat(2,minmax(0,1fr))}.row,.subgrid{grid-template-columns:1fr}}
@media (max-width:560px){.grid{grid-template-columns:1fr}.title{font-size:24px}}
</style></head><body><div class="wrap">
<div class="head">
  <div>
    <div class="title">HyperCore Dashboard</div>
    <div class="meta">Generated: $(Escape-Html ([string]$dash.generated_utc)) | Profile: $(Escape-Html ([string]$dash.profile)) | Telemetry Rows: $(Escape-Html ([string]$telemetryRows.Count))</div>
  </div>
</div>
<div class="grid">
  <div class="card"><div class="k">Health Score</div><div class="v">$(Escape-Html ([string]$healthScore))/$(Escape-Html ([string]$healthMax))</div><div class="muted">$(Escape-Html ([string]$healthPct))%</div></div>
  <div class="card"><div class="k">Doctor</div><div class="v $(if([bool]$dash.doctor_ok){'ok'}else{'bad'})">$(Escape-Html ([string]$dash.doctor_ok))</div></div>
  <div class="card"><div class="k">QEMU</div><div class="v $(if([bool]$dash.qemu_ok){'ok'}else{'bad'})">$(Escape-Html ([string]$dash.qemu_ok))</div></div>
  <div class="card"><div class="k">RC Ready</div><div class="v $(if([bool]$dash.rc_ready){'ok'}else{'warn'})">$(Escape-Html ([string]$dash.rc_ready))</div></div>
  <div class="card"><div class="k">Drift / Flaky</div><div class="v $(if($driftCount -eq 0 -and $flakyCommands -eq 0){'ok'}else{'warn'})">$(Escape-Html ([string]$driftCount)) / $(Escape-Html ([string]$flakyCommands))</div><div class="muted">drift tools / flaky cmds</div></div>
</div>
<div class="row">
  <div class="card">
    <div class="k">Failure Analytics</div>
    <canvas id="trend"></canvas>
    <div class="legend">
      <span class='badge b-$(if([bool]$tr.alert){'bad'}else{'ok'})'>Trend Alert: $(Escape-Html ([string]$tr.alert))</span>
      <span class='badge b-$(if($anomalyAlert){'bad'}else{'ok'})'>Anomaly: $(Escape-Html ([string]$anomalyAlert))</span>
      <span class='badge b-$(if($anomalyFresh){'ok'}else{'warn'})'>Anomaly freshness(h): $(Escape-Html ([string]$anomalyAgeHours))</span>
      <span class='badge b-$(if($policyOk){'ok'}else{'warn'})'>Policy: $(Escape-Html ([string]$policyOk))</span>
      <span class='badge b-$(if($schemaOk){'ok'}else{'warn'})'>Schema: $(Escape-Html ([string]$schemaOk))</span>
    </div>
    <div class="meta" style="margin-top:8px">Samples=$(Escape-Html ([string]$tr.samples)), Failures=$(Escape-Html ([string]$tr.failures)), FailureRate=$(Escape-Html ([string]$tr.failure_rate_pct))%, AlertReason=$(Escape-Html ([string]$tr.alert_reason))</div>
  </div>
  <div class="card">
    <div class="k">What Should I Do Next?</div>
    <ul>
      $($actionRows -join "`n")
    </ul>
    <div class="muted" style="margin-top:10px">This list is generated from doctor/qemu/policy/anomaly/flaky/drift signals.</div>
  </div>
</div>
<div class="row">
  <div class="card">
    <div class="k">Execution Status Matrix</div>
    <table><thead><tr><th>Check</th><th>Status</th><th>Meaning</th></tr></thead><tbody>
    $($summaryTableRows -join "`n")
    </tbody></table>
  </div>
  <div class="card">
    <div class="k">Slowest Operations (Top 12)</div>
    <table><thead><tr><th>Event</th><th>Duration(ms)</th><th>Status</th></tr></thead><tbody>
    $($durRows -join "`n")
    </tbody></table>
  </div>
</div>
<div class="subgrid" style="margin-top:12px">
  <div class="card">
    <div class="k">Recent Failures</div>
    <table><thead><tr><th>Event</th><th>Status</th><th>Message</th></tr></thead><tbody>
    $($rows -join "`n")
    </tbody></table>
  </div>
  <div class="card">
    <div class="k">Run Timeline (Last 40)</div>
    <table><thead><tr><th>Timestamp</th><th>Command</th><th>Status</th><th>Code</th></tr></thead><tbody>
    $($timelineRows -join "`n")
    </tbody></table>
  </div>
</div>
<div class="card" style="margin-top:12px">
  <div class="k">Deep Diagnostics (Raw JSON)</div>
  <details><summary>dashboard.json</summary><pre>$detailDash</pre></details>
  <details><summary>trends.json</summary><pre>$detailTrend</pre></details>
  <details><summary>health_report.json</summary><pre>$detailHealth</pre></details>
  <details><summary>anomaly_report.json</summary><pre>$detailAnomaly</pre></details>
  <details><summary>policy_gate.json</summary><pre>$detailPolicy</pre></details>
</div>
</div>
<script>
(()=> {
  const c=document.getElementById('trend'),x=c.getContext('2d');
  const labels=$failLabelJson;
  const values=$failValueJson;
  c.width=c.clientWidth*2;c.height=220*2;x.scale(2,2);
  x.clearRect(0,0,c.width,c.height);
  const w=c.clientWidth,h=220,p=22,max=Math.max(...values,1),bw=Math.max(16,Math.floor((w-2*p)/Math.max(values.length,1))-8);
  x.strokeStyle="#334155";x.lineWidth=1;
  for(let gy=0;gy<5;gy++){const y=p+gy*(h-2*p)/4;x.beginPath();x.moveTo(p,y);x.lineTo(w-p,y);x.stroke();}
  values.forEach((v,i)=>{
    const x0=p+i*((w-2*p)/Math.max(values.length,1))+4;
    const bh=(v/max)*(h-2*p); const y0=h-p-bh;
    x.fillStyle="#0ea5e9"; x.fillRect(x0,y0,bw,bh);
    x.fillStyle="#cbd5e1"; x.font="11px Segoe UI"; x.fillText(String(v),x0,y0-4);
    const lbl=(labels[i]||"").toString().slice(0,18);
    x.fillStyle="#94a3b8"; x.fillText(lbl,x0,h-p+14);
  });
})();
</script>
</body></html>
"@
    Set-Content -Path "reports/tooling/dashboard.html" -Value $html -Encoding UTF8
    Write-HcMsg "html_dashboard_path" @("reports/tooling/dashboard.html")
}

function Get-HcJsonOrDefault {
    param(
        [string]$Path,
        $Default = @{}
    )
    if (-not (Test-Path $Path)) { return $Default }
    try { return (Get-HcJsonFile -Path $Path) } catch { return $Default }
}

function Run-ExportDashboardData {
    param($Settings)
    if (-not (Test-Path "reports/tooling/dashboard.json")) {
        Run-Dashboard -Settings $Settings
    }
    if (-not (Test-Path "reports/tooling/trends.json")) {
        Run-Trends -Settings $Settings
    }
    if (-not (Test-Path "reports/tooling/health_report.json")) {
        Run-Health -Settings $Settings
    }
    if (-not (Test-Path "reports/tooling/anomaly_report.json")) {
        Run-AnomalyReport -Settings $Settings
    }
    if (-not (Test-Path "reports/tooling/policy_gate.json")) {
        Run-PolicyGate
    }
    if (-not (Test-Path "reports/tooling/flaky_report.json")) {
        Run-DetectFlaky -Settings $Settings
    }
    if (-not (Test-Path "reports/tooling/dependency_drift.json")) {
        Run-DependencyDrift -Settings $Settings
    }
    if (-not (Test-Path "reports/tooling/artifact_verify.json")) {
        Run-VerifyArtifacts
    }
    if (-not (Test-Path "reports/tooling/schema_validation.json")) {
        Run-ValidateSchemas
    }
    if (-not (Test-Path "reports/tooling/p_tier_status.json")) {
        Run-TierStatus
    }

    $dash = Get-HcJsonOrDefault -Path "reports/tooling/dashboard.json" -Default @{}
    $trend = Get-HcJsonOrDefault -Path "reports/tooling/trends.json" -Default @{}
    $health = Get-HcJsonOrDefault -Path "reports/tooling/health_report.json" -Default @{}
    $anomaly = Get-HcJsonOrDefault -Path "reports/tooling/anomaly_report.json" -Default @{}
    $policy = Get-HcJsonOrDefault -Path "reports/tooling/policy_gate.json" -Default @{}
    $flaky = Get-HcJsonOrDefault -Path "reports/tooling/flaky_report.json" -Default @{}
    $drift = Get-HcJsonOrDefault -Path "reports/tooling/dependency_drift.json" -Default @{}
    $artifactVerify = Get-HcJsonOrDefault -Path "reports/tooling/artifact_verify.json" -Default @{}
    $schema = Get-HcJsonOrDefault -Path "reports/tooling/schema_validation.json" -Default @{}
    $tierStatus = Get-HcJsonOrDefault -Path "reports/tooling/p_tier_status.json" -Default @{}
    $doctor = Get-HcJsonOrDefault -Path "reports/tooling/doctor_report.json" -Default @{}
    $update = Get-HcJsonOrDefault -Path "reports/tooling/update_check.json" -Default @{}
    $playbook = Get-HcJsonOrDefault -Path "scripts/config/hc_error_playbook.json" -Default @{}

    $telemetryRows = @()
    if (Test-Path $Settings.telemetry_jsonl_path) {
        foreach ($ln in (Get-Content -Path $Settings.telemetry_jsonl_path -Tail 3000)) {
            try { $telemetryRows += ($ln | ConvertFrom-Json) } catch {}
        }
    }
    $recent = @($telemetryRows | Select-Object -Last 150)
    $failureGroups = @($telemetryRows |
        Where-Object { ([string]$_.status -eq "fail") -or ([string]$_.level -eq "error") } |
        Group-Object -Property event |
        Sort-Object Count -Descending |
        Select-Object -First 20)

    $failByEvent = [ordered]@{}
    foreach ($g in $failureGroups) {
        $failByEvent[[string]$g.Name] = [int]$g.Count
    }

    $artifactPaths = @(
        "reports/tooling/dashboard.html",
        "reports/tooling/dashboard.md",
        "reports/tooling/trends.md",
        "reports/tooling/triage.md",
        "reports/tooling/dashboard_data.json",
        "reports/tooling/health_report.json",
        "reports/tooling/hypercore_telemetry.jsonl"
    )
    $artifactInventory = @()
    foreach ($p in $artifactPaths) {
        if (Test-Path $p -PathType Leaf) {
            $fi = Get-Item $p
            $artifactInventory += [ordered]@{
                path = $p
                size_bytes = [int64]$fi.Length
                size_kib = [math]::Round(([double]$fi.Length / 1KB), 2)
                modified_utc = $fi.LastWriteTimeUtc.ToString("o")
            }
        }
    }

    $runHistory = @()
    $manifests = @(Get-ChildItem -Path "reports/tooling" -Filter "run_manifest_*.json" -File -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 30)
    foreach ($m in $manifests) {
        try {
            $obj = Get-HcJsonFile -Path $m.FullName
            $runHistory += [ordered]@{
                run_id = [string]$obj.run_id
                command = [string]$obj.command
                profile = [string]$obj.profile
                status = [string]$obj.status
                exit_code = if ($obj.exit_code) { [int]$obj.exit_code } else { 0 }
                start_utc = [string]$obj.start_utc
                end_utc = [string]$obj.end_utc
                script_version = [string]$obj.script_version
                manifest_path = $m.FullName
            }
        } catch {}
    }

    $plugins = @()
    $pluginWidgets = @()
    $pluginManifests = @(Get-ChildItem -Path $Settings.plugin_dir -Filter "*.plugin.json" -File -ErrorAction SilentlyContinue | Sort-Object Name)
    foreach ($pm in $pluginManifests) {
        try {
            $pj = Get-HcJsonFile -Path $pm.FullName
            $plugins += [ordered]@{
                name = [string]$pj.name
                version = [string]$pj.version
                min_api_version = [string]$pj.min_api_version
                path = $pm.FullName
            }
            if ($pj.dashboard_widgets -and ($pj.dashboard_widgets -is [System.Collections.IEnumerable])) {
                foreach ($w in $pj.dashboard_widgets) {
                    try {
                        $pluginWidgets += [ordered]@{
                            plugin = [string]$pj.name
                            id = [string]$w.id
                            title = [string]$w.title
                            kind = if ($w.kind) { [string]$w.kind } else { "stat" }
                            value = if ($w.PSObject.Properties.Name -contains "value") { $w.value } else { $null }
                            unit = if ($w.unit) { [string]$w.unit } else { "" }
                            status = if ($w.status) { [string]$w.status } else { "info" }
                            detail = if ($w.detail) { [string]$w.detail } else { "" }
                            columns = if ($w.columns -and ($w.columns -is [System.Collections.IEnumerable])) { @($w.columns) } else { @() }
                            rows = if ($w.rows -and ($w.rows -is [System.Collections.IEnumerable])) { @($w.rows) } else { @() }
                        }
                    } catch {}
                }
            }
        } catch {}
    }

    $export = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        profile = $Profile
        run_id = $script:RunId
        summary = [ordered]@{
            health_score = if ($health.score) { [int]$health.score } else { 0 }
            health_max = if ($health.max_score) { [int]$health.max_score } else { 100 }
            doctor_ok = [bool]$dash.doctor_ok
            qemu_ok = [bool]$dash.qemu_ok
            rc_ready = [bool]$dash.rc_ready
            policy_ok = [bool]$policy.ok
            anomaly_alert = [bool]$anomaly.alert
            drift_count = if ($drift.drift_count) { [int]$drift.drift_count } else { 0 }
            flaky_count = if ($flaky.rows) { @($flaky.rows | Where-Object { [bool]$_.flaky }).Count } else { 0 }
            telemetry_rows = $telemetryRows.Count
        }
        trend = $trend
        failure_by_event = $failByEvent
        recent_failures = if ($dash.recent_failures) { @($dash.recent_failures) } else { @() }
        telemetry_recent = $recent
        run_history = $runHistory
        host = [ordered]@{
            machine = [string]$env:COMPUTERNAME
            os = [string]$env:OS
            script_version = [string]$Settings.script_version
            tool_versions = if ($doctor.versions) { $doctor.versions } else { @{} }
            update_tools = if ($update.tools) { $update.tools } else { @{} }
        }
        ui = [ordered]@{
            default_language = if ($Settings.language) { [string]$Settings.language } else { "en" }
            feature_flags = if ($Settings.ui_feature_flags) { $Settings.ui_feature_flags } else { @{} }
            plugin_widgets = $pluginWidgets
        }
        plugins = $plugins
        tier_status = $tierStatus
        artifacts = [ordered]@{
            dashboard_html = "reports/tooling/dashboard.html"
            dashboard_markdown = "reports/tooling/dashboard.md"
            trends_markdown = "reports/tooling/trends.md"
            triage_markdown = "reports/tooling/triage.md"
            inventory = $artifactInventory
        }
        playbook = $playbook
        sources = [ordered]@{
            dashboard = $dash
            health = $health
            anomaly = $anomaly
            policy = $policy
            drift = $drift
            flaky = $flaky
            schema_validation = $schema
            artifact_verify = $artifactVerify
        }
    }

    $jsonText = $export | ConvertTo-Json -Depth 16
    $reportOut = "reports/tooling/dashboard_data.json"
    $uiOut = "dashboard-ui/src/generated/dashboard_data.json"
    $uiOutDir = Split-Path -Parent $uiOut
    if (-not (Test-Path $uiOutDir)) {
        New-Item -ItemType Directory -Force -Path $uiOutDir | Out-Null
    }
    Set-Content -Path $reportOut -Value $jsonText -Encoding UTF8
    Set-Content -Path $uiOut -Value $jsonText -Encoding UTF8
    Write-HcMsg "dashboard_data_exported" @($reportOut)
}

function Run-DashboardUiBuild {
    param($Settings)
    if (-not (Test-HcCommand "npm")) {
        Fail-Hc -Code "dependency_missing" -Message "npm is required for dashboard-ui-build"
    }
    if (-not (Test-Path "dashboard-ui/package.json")) {
        Fail-Hc -Code "dependency_missing" -Message "dashboard-ui/package.json not found"
    }
    Write-HcMsg "dashboard_ui_build_start"
    Run-ExportDashboardData -Settings $Settings
    if (-not (Test-Path "dashboard-ui/node_modules")) {
        Write-HcMsg "dashboard_ui_installing_deps"
        Invoke-HcPowerShellCommand "Push-Location 'dashboard-ui'; try { npm install --no-audit --no-fund; if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE } } finally { Pop-Location }"
    }
    Invoke-HcPowerShellCommand "Push-Location 'dashboard-ui'; try { npm run build; if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE } } finally { Pop-Location }"
    if (Test-Path "reports/tooling/dashboard_data.json") {
        Copy-Item -Force -Path "reports/tooling/dashboard_data.json" -Destination "reports/tooling/dashboard_ui/dashboard_data.json"
    }
    Write-HcMsg "dashboard_ui_build_done" @("reports/tooling/dashboard_ui/index.html")
}

function Run-DashboardUiDev {
    param($Settings)
    if (-not (Test-HcCommand "npm")) {
        Fail-Hc -Code "dependency_missing" -Message "npm is required for dashboard-ui-dev"
    }
    if (-not (Test-Path "dashboard-ui/package.json")) {
        Fail-Hc -Code "dependency_missing" -Message "dashboard-ui/package.json not found"
    }
    Write-HcMsg "dashboard_ui_dev_start"
    Run-ExportDashboardData -Settings $Settings
    if (-not (Test-Path "dashboard-ui/node_modules")) {
        Write-HcMsg "dashboard_ui_installing_deps"
        Invoke-HcPowerShellCommand "Push-Location 'dashboard-ui'; try { npm install --no-audit --no-fund; if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE } } finally { Pop-Location }"
    }
    Invoke-HcPowerShellCommand "Push-Location 'dashboard-ui'; try { npm run dev -- --host; if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE } } finally { Pop-Location }"
}

function Run-DashboardUiTest {
    if (-not (Test-HcCommand "npm")) {
        Fail-Hc -Code "dependency_missing" -Message "npm is required for dashboard-ui-test"
    }
    if (-not (Test-Path "dashboard-ui/package.json")) {
        Fail-Hc -Code "dependency_missing" -Message "dashboard-ui/package.json not found"
    }
    Write-HcMsg "dashboard_ui_test_start"
    if (-not (Test-Path "dashboard-ui/node_modules")) {
        Write-HcMsg "dashboard_ui_installing_deps"
        Invoke-HcPowerShellCommand "Push-Location 'dashboard-ui'; try { npm install --no-audit --no-fund; if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE } } finally { Pop-Location }"
    }
    Invoke-HcPowerShellCommand "Push-Location 'dashboard-ui'; try { npm run test:run; if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE } } finally { Pop-Location }"
    Write-HcMsg "dashboard_ui_test_done"
}

function Run-DashboardUiE2E {
    if (-not (Test-HcCommand "npm")) {
        Fail-Hc -Code "dependency_missing" -Message "npm is required for dashboard-ui-e2e"
    }
    if (-not (Test-Path "dashboard-ui/package.json")) {
        Fail-Hc -Code "dependency_missing" -Message "dashboard-ui/package.json not found"
    }
    Write-HcMsg "dashboard_ui_e2e_start"
    if (-not (Test-Path "dashboard-ui/node_modules")) {
        Write-HcMsg "dashboard_ui_installing_deps"
        Invoke-HcPowerShellCommand "Push-Location 'dashboard-ui'; try { npm install --no-audit --no-fund; if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE } } finally { Pop-Location }"
    }
    Invoke-HcPowerShellCommand "Push-Location 'dashboard-ui'; try { npm run test:e2e; if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE } } finally { Pop-Location }"
    Write-HcMsg "dashboard_ui_e2e_done"
}

function Run-DashboardUiE2ESetup {
    if (-not (Test-HcCommand "npm")) {
        Fail-Hc -Code "dependency_missing" -Message "npm is required for dashboard-ui-e2e-setup"
    }
    if (-not (Test-Path "dashboard-ui/package.json")) {
        Fail-Hc -Code "dependency_missing" -Message "dashboard-ui/package.json not found"
    }
    Write-HcMsg "dashboard_ui_e2e_setup_start"
    if (-not (Test-Path "dashboard-ui/node_modules")) {
        Write-HcMsg "dashboard_ui_installing_deps"
        Invoke-HcPowerShellCommand "Push-Location 'dashboard-ui'; try { npm install --no-audit --no-fund; if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE } } finally { Pop-Location }"
    }
    Invoke-HcPowerShellCommand "Push-Location 'dashboard-ui'; try { npx playwright install chromium; if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE } } finally { Pop-Location }"
    Write-HcMsg "dashboard_ui_e2e_setup_done"
}

function Run-CheckUpdates {
    param($Settings)
    if ($UseCache) {
        $cached = Get-HcCache -Key "update_check" -TtlSec $Settings.cache_ttl_sec
        if ($cached) {
            Save-HcJsonFile -Object $cached -Path "reports/tooling/update_check.json"
            Write-HcMsg "update_check_cache_hit"
            return
        }
    }
    $result = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        script_version = $Settings.script_version
        git_available = (Test-HcCommand "git")
        update_available = $false
        details = ""
        tools = [ordered]@{
            rustc = Get-ToolVersion -Cmd "rustc" -CmdArgs @("-V")
            cargo = Get-ToolVersion -Cmd "cargo" -CmdArgs @("-V")
            qemu = Get-ToolVersionOrPath -Cmd "qemu-system-x86_64" -FallbackPath "$env:ProgramFiles\qemu\qemu-system-x86_64.exe" -CmdArgs @("--version")
            xorriso = Get-ToolVersionOrPath -Cmd "xorriso" -FallbackPath "C:\msys64\usr\bin\xorriso.exe" -CmdArgs @("-version")
        }
    }
    if ($result.git_available) {
        if ($Offline) {
            $result.details = "offline mode: git fetch skipped"
            Save-HcJsonFile -Object $result -Path "reports/tooling/update_check.json"
            Write-HcMsg "update_check_path" @("reports/tooling/update_check.json")
            return
        }
        try {
            & git fetch --quiet 2>$null
            $aheadBehind = [string](& git rev-list --left-right --count HEAD...@'{u}' 2>$null)
            if ($aheadBehind) {
                $parts = $aheadBehind.Trim().Split()
                if ($parts.Count -ge 2 -and [int]$parts[1] -gt 0) {
                    $result.update_available = $true
                    $result.details = "remote has newer commits"
                } else {
                    $result.details = "up-to-date"
                }
            } else {
                $result.details = "no upstream tracking branch"
            }
        } catch {
            $result.details = "git update check failed: $([string]$_)"
        }
    }
    Set-HcCache -Key "update_check" -Payload $result
    Save-HcJsonFile -Object $result -Path "reports/tooling/update_check.json"
    Write-HcMsg "update_check_path" @("reports/tooling/update_check.json")
}

function Run-DetectFlaky {
    param($Settings)
    $p = $Settings.telemetry_jsonl_path
    if (-not (Test-Path $p)) { throw "telemetry file missing: $p" }
    $rows = @()
    foreach ($ln in (Get-Content -Path $p -Tail ([Math]::Max($FlakyWindow, 20) * 10))) {
        try { $rows += ($ln | ConvertFrom-Json) } catch {}
    }
    $windowRows = @($rows | Where-Object { $_.event -eq "run.end" } | Select-Object -Last $FlakyWindow)
    $byCmd = @{}
    foreach ($r in $windowRows) {
        $cmd = [string]$r.command
        if (-not $byCmd.ContainsKey($cmd)) {
            $byCmd[$cmd] = [ordered]@{ total = 0; fails = 0 }
        }
        $byCmd[$cmd].total += 1
        if ([string]$r.status -eq "fail") { $byCmd[$cmd].fails += 1 }
    }
    $reportRows = @()
    foreach ($k in ($byCmd.Keys | Sort-Object)) {
        $t = [int]$byCmd[$k].total
        $f = [int]$byCmd[$k].fails
        $rate = if ($t -gt 0) { [math]::Round(100.0 * $f / $t, 2) } else { 0.0 }
        $reportRows += [ordered]@{
            command = $k
            total = $t
            fails = $f
            failure_rate_pct = $rate
            flaky = ($rate -gt 0.0 -and $rate -lt 100.0)
        }
    }
    $out = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        window = $FlakyWindow
        rows = $reportRows
    }
    Save-HcJsonFile -Object $out -Path "reports/tooling/flaky_report.json"
    Write-HcMsg "flaky_report_path" @("reports/tooling/flaky_report.json")
}

function Get-CommandPrereqs {
    $map = @{
        "qemu-smoke" = @("doctor", "build-iso")
        "ci-smoke" = @("doctor", "install")
        "pre-release" = @("doctor", "gate", "health")
        "support-bundle" = @("collect-diagnostics")
        "run-plugin" = @("plugins")
    }
    return $map
}

function Run-PrereqCheck {
    param($Settings)
    $pr = Get-CommandPrereqs
    if (-not $pr.ContainsKey($Command)) {
        Write-HcMsg "prereq_none"
        return
    }
    $missing = @()
    foreach ($p in @($pr[$Command])) {
        switch ($p) {
            "doctor" {
                if (-not (Test-Path $Settings.doctor_report_path)) { $missing += "doctor_report" }
            }
            "build-iso" {
                if (-not (Test-Path "artifacts/boot_image/hypercore.iso")) { $missing += "hypercore.iso" }
            }
            "health" {
                if (-not (Test-Path $Settings.health_report_path)) { $missing += "health_report" }
            }
            "collect-diagnostics" {
                if (-not (Test-Path $DiagnosticsZipPath)) { $missing += "diagnostics_zip" }
            }
        }
    }
    $out = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        command = $Command
        missing = $missing
        ok = ($missing.Count -eq 0)
    }
    Save-HcJsonFile -Object $out -Path "reports/tooling/prereq_check.json"
    Write-HcMsg "prereq_check_path" @("reports/tooling/prereq_check.json")
}

function Run-DryRunDiff {
    $tracked = @(
        "reports/tooling/dashboard.json",
        "reports/tooling/trends.json",
        "reports/tooling/health_report.json",
        "reports/tooling/triage.json",
        $DiagnosticsZipPath
    )
    $existing = @($tracked | Where-Object { Test-Path $_ })
    $missing = @($tracked | Where-Object { -not (Test-Path $_) })
    $out = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        command = $Command
        dry_run = [bool]$DryRun
        would_update = $tracked
        exists_now = $existing
        missing_now = $missing
    }
    Save-HcJsonFile -Object $out -Path "reports/tooling/dry_run_diff.json"
    Write-HcMsg "dry_run_diff_path" @("reports/tooling/dry_run_diff.json")
}

function Run-ApplyFix {
    param($Cfg, $Settings)
    if (-not (Test-Path "reports/tooling/last_failure_explained.json")) {
        Run-ExplainLastFailure -Settings $Settings
    }
    $f = if (Test-Path "reports/tooling/last_failure_explained.json") { Get-HcJsonFile -Path "reports/tooling/last_failure_explained.json" } else { $null }
    if ($null -eq $f) { Write-HcMsg "no_failure_explanation"; return }
    $cat = [string]$f.category
    if ($cat -in @("dependency", "tool_missing", "unknown")) {
        Run-DoctorFix -Cfg $Cfg -Settings $Settings
    } else {
        Write-HcMsg "no_auto_fix_for_category" @($cat)
    }
}

function Run-ArtifactManifest {
    $items = @(
        "artifacts/boot_image/hypercore.iso",
        "reports/tooling/dashboard.json",
        "reports/tooling/trends.json",
        "reports/tooling/health_report.json",
        "reports/tooling/triage.json",
        $DiagnosticsZipPath
    ) | Where-Object { Test-Path $_ -PathType Leaf }
    $rows = @()
    foreach ($i in $items) {
        $h = Get-FileHash -Algorithm SHA256 -Path $i
        $rows += [ordered]@{
            path = $i
            sha256 = $h.Hash.ToLowerInvariant()
            bytes = (Get-Item $i).Length
            mtime_utc = (Get-Item $i).LastWriteTimeUtc.ToString("o")
        }
    }
    $out = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        rows = $rows
    }
    Save-HcJsonFile -Object $out -Path $ArtifactManifestPath
    Write-HcMsg "artifact_manifest_path" @($ArtifactManifestPath)
}

function Run-VerifyArtifacts {
    if (-not (Test-Path $ArtifactManifestPath)) {
        Fail-Hc -Code "dependency_missing" -Message ("artifact manifest missing: {0}" -f $ArtifactManifestPath)
    }
    $m = Get-HcJsonFile -Path $ArtifactManifestPath
    $bad = @()
    foreach ($r in @($m.rows)) {
        $p = [string]$r.path
        if (-not (Test-Path $p -PathType Leaf)) { $bad += "missing:$p"; continue }
        $h = (Get-FileHash -Algorithm SHA256 -Path $p).Hash.ToLowerInvariant()
        if ($h -ne [string]$r.sha256) { $bad += "hash:$p" }
    }
    $out = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        ok = ($bad.Count -eq 0)
        issues = $bad
    }
    Save-HcJsonFile -Object $out -Path "reports/tooling/artifact_verify.json"
    if ($bad.Count -gt 0) { Fail-Hc -Code "command_failed" -Message "artifact verification failed" }
    Write-HcMsg "artifact_verification_ok"
}

function Run-AnomalyReport {
    param($Settings)
    $failRateAlertThresholdPct = 20.0
    $durationSpikeRatio = 3.0
    $durationSpikeMinP95Ms = 60000.0

    if (-not (Test-Path $Settings.telemetry_jsonl_path)) { Fail-Hc -Code "dependency_missing" -Message "telemetry missing" }
    $rows = @()
    foreach ($ln in (Get-Content -Path $Settings.telemetry_jsonl_path -Tail 1000)) {
        try { $rows += ($ln | ConvertFrom-Json) } catch {}
    }
    $ends = @($rows | Where-Object { $_.event -eq "run.end" } | Select-Object -Last 100)
    $failRate = if ($ends.Count -gt 0) { [math]::Round((100.0 * (@($ends | Where-Object {$_.status -eq "fail"}).Count) / $ends.Count),2) } else { 0.0 }
    $dur = @($rows | Where-Object { $_.duration_ms -gt 0 } | Select-Object -Last 200)
    $avgDur = if ($dur.Count -gt 0) { [math]::Round((($dur | Measure-Object -Property duration_ms -Average).Average),2) } else { 0.0 }
    $p95 = 0.0
    if ($dur.Count -gt 0) {
        $sorted = @($dur | Sort-Object duration_ms)
        $idx = [math]::Min($sorted.Count - 1, [math]::Floor(0.95 * $sorted.Count))
        $p95 = [double]$sorted[$idx].duration_ms
    }
    $durSpike = ($avgDur -gt 0 -and $p95 -gt ($avgDur * $durationSpikeRatio) -and $p95 -gt $durationSpikeMinP95Ms)
    $alert = ($failRate -gt $failRateAlertThresholdPct) -or $durSpike
    $out = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        fail_rate_pct = $failRate
        avg_duration_ms = $avgDur
        p95_duration_ms = [math]::Round($p95,2)
        fail_rate_threshold_pct = $failRateAlertThresholdPct
        duration_spike_ratio = $durationSpikeRatio
        duration_spike_min_p95_ms = $durationSpikeMinP95Ms
        alert = [bool]$alert
    }
    Save-HcJsonFile -Object $out -Path "reports/tooling/anomaly_report.json"
    Write-HcMsg "anomaly_report_path" @("reports/tooling/anomaly_report.json")
}

function Run-BisectHelper {
    $head = ""
    $prev = ""
    try { $head = [string](& git rev-parse HEAD).Trim() } catch {}
    try { $prev = [string](& git rev-parse HEAD~20).Trim() } catch {}
    $out = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        suggested_bad = $head
        suggested_good = $prev
        command = if ($head -and $prev) { "git bisect start $head $prev" } else { "" }
    }
    Save-HcJsonFile -Object $out -Path "reports/tooling/bisect_helper.json"
    Write-HcMsg "bisect_helper_path" @("reports/tooling/bisect_helper.json")
}

function Run-DependencyDrift {
    param($Settings)
    $basePath = "reports/tooling/dependency_baseline.json"
    $now = [ordered]@{
        rustc = Get-ToolVersion -Cmd "rustc" -CmdArgs @("-V")
        cargo = Get-ToolVersion -Cmd "cargo" -CmdArgs @("-V")
        qemu = Get-ToolVersionOrPath -Cmd "qemu-system-x86_64" -FallbackPath "$env:ProgramFiles\qemu\qemu-system-x86_64.exe" -CmdArgs @("--version")
        xorriso = Get-ToolVersionOrPath -Cmd "xorriso" -FallbackPath "C:\msys64\usr\bin\xorriso.exe" -CmdArgs @("-version")
    }
    if (-not (Test-Path $basePath)) {
        Save-HcJsonFile -Object $now -Path $basePath
    }
    $base = Get-HcJsonFile -Path $basePath
    $changes = @()
    foreach ($k in @("rustc","cargo","qemu","xorriso")) {
        if ([string]$base.$k -ne [string]$now.$k) {
            $changes += [ordered]@{ tool = $k; baseline = [string]$base.$k; current = [string]$now.$k }
        }
    }
    $out = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        drift_count = $changes.Count
        drift = $changes
    }
    Save-HcJsonFile -Object $out -Path "reports/tooling/dependency_drift.json"
    Write-HcMsg "dependency_drift_path" @("reports/tooling/dependency_drift.json")
}

function Run-PolicyGate {
    $policyPath = "scripts/config/hypercore.policy.json"
    if (-not (Test-Path $policyPath)) {
        Fail-Hc -Code "dependency_missing" -Message "policy file missing: scripts/config/hypercore.policy.json"
    }
    $policy = Get-HcJsonFile -Path $policyPath
    if (-not (Test-Path "reports/tooling/health_report.json")) { Fail-Hc -Code "dependency_missing" -Message "health report missing" }
    $health = Get-HcJsonFile -Path "reports/tooling/health_report.json"
    $ok = ($health.score -ge [int]$policy.min_health_score)
    $reasons = @()
    if (-not $ok) { $reasons += "health score below threshold" }
    if (Test-Path "reports/tooling/anomaly_report.json") {
        $an = Get-HcJsonFile -Path "reports/tooling/anomaly_report.json"
        if ([bool]$an.alert -and [bool]$policy.block_on_anomaly) { $ok = $false; $reasons += "anomaly alert" }
    }
    $out = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        ok = $ok
        reasons = $reasons
    }
    Save-HcJsonFile -Object $out -Path "reports/tooling/policy_gate.json"
    if (-not $ok) { Fail-Hc -Code "command_failed" -Message "policy gate failed" }
    Write-HcMsg "policy_gate_passed"
}

function Run-LinuxAbiGate {
    $py = Resolve-HcPython
    Invoke-External $py @("scripts/linux_abi_gate.py")
    Write-HcMsg "step_done"
}

function Run-LinuxPlatformReadiness {
    $py = Resolve-HcPython
    Invoke-External $py @("scripts/linux_platform_readiness.py")
    Write-HcMsg "step_done"
}

function Run-MergeTelemetry {
    param($Settings)
    $dir = $MergeTelemetryDir
    if (-not (Test-Path $dir)) { Fail-Hc -Code "dependency_missing" -Message ("merge dir missing: {0}" -f $dir) }
    $all = @()
    $files = Get-ChildItem -Path $dir -Filter "*.jsonl" -File -Recurse
    foreach ($f in $files) {
        foreach ($ln in (Get-Content -Path $f.FullName -ErrorAction SilentlyContinue)) {
            try { $all += ($ln | ConvertFrom-Json) } catch {}
        }
    }
    $all = @($all | Sort-Object timestamp_utc)
    $outPath = "reports/tooling/merged_telemetry.jsonl"
    $outDir = Split-Path -Parent $outPath
    if (-not (Test-Path $outDir)) { New-Item -ItemType Directory -Force -Path $outDir | Out-Null }
    if (Test-Path $outPath) { Remove-Item -Force $outPath }
    foreach ($r in $all) {
        ($r | ConvertTo-Json -Compress -Depth 12) | Out-File -FilePath $outPath -Append -Encoding utf8
    }
    Write-HcMsg "merged_telemetry_path" @($outPath)
}

function Run-ValidateSchemas {
    $checks = @(
        @{ path = "reports/tooling/dashboard.json"; keys = @("generated_utc","profile","doctor_ok") },
        @{ path = "reports/tooling/trends.json"; keys = @("generated_utc","samples","failures") },
        @{ path = "reports/tooling/health_report.json"; keys = @("score","max_score","doctor_ok") },
        @{ path = "reports/tooling/triage.json"; keys = @("generated_utc","doctor_ok","health_score") }
    )
    $issues = @()
    foreach ($c in $checks) {
        if (-not (Test-Path $c.path)) { $issues += "missing:$($c.path)"; continue }
        $obj = Get-HcJsonFile -Path $c.path
        foreach ($k in $c.keys) {
            if (-not $obj.PSObject.Properties.Name.Contains($k)) { $issues += "missing_key:$($c.path):$k" }
        }
    }
    try {
        $py = Resolve-HcPython
        Invoke-External $py @("scripts/tools/check_command_catalog.py")
    } catch {
        $issues += "command_catalog_validation_failed"
    }
    $out = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        ok = ($issues.Count -eq 0)
        issues = $issues
    }
    Save-HcJsonFile -Object $out -Path "reports/tooling/schema_validation.json"
    if ($issues.Count -gt 0) { Fail-Hc -Code "command_failed" -Message "schema validation failed" }
    Write-HcMsg "schema_validation_ok"
}

function Run-Canary {
    param($Cfg, $Settings)
    Run-Doctor -Settings $Settings
    $oldRounds = $Settings.rounds
    $oldChaos = $Settings.chaos_rate
    $Settings.rounds = 1
    $Settings.chaos_rate = 0.0
    Run-QemuSmoke -Settings $Settings
    $Settings.rounds = $oldRounds
    $Settings.chaos_rate = $oldChaos
    Run-Health -Settings $Settings
}

function Run-ReplayRun {
    if (-not $ReplayManifestPath) {
        $latest = Get-ChildItem -Path "reports/tooling" -Filter "run_manifest_*.json" -File -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
        if ($latest) { $ReplayManifestPath = $latest.FullName }
    }
    if (-not $ReplayManifestPath -or -not (Test-Path $ReplayManifestPath)) {
        Fail-Hc -Code "dependency_missing" -Message "replay manifest not found"
    }
    $m = Get-HcJsonFile -Path $ReplayManifestPath
    $cmd = [string]$m.command
    $prof = [string]$m.profile
    Write-HcMsg "replaying_command" @($cmd, $prof)
    $args = @("-ExecutionPolicy","Bypass","-File",".\scripts\hypercore.ps1","-Command",$cmd,"-Profile",$prof,"-NoLock")
    if ($Deterministic) { $args += @("-Deterministic","-Seed",$Seed) }
    if ($DryRun) { $args += "-DryRun" }
    Invoke-External $script:ShellExe $args
}

function Run-LintI18n {
    $files = @(
        "scripts/hypercore.ps1",
        "scripts/hypercore/novice.ps1",
        "scripts/hypercore/plugins.ps1"
    ) | Where-Object { Test-Path $_ }
    $issues = @()
    foreach ($f in $files) {
        $lines = Get-Content -Path $f
        for ($i=0; $i -lt $lines.Count; $i++) {
            $ln = $lines[$i]
            if ($ln -match 'Write-HcStep .+"[^"]+"') {
                if ($ln -notmatch "Get-Msg" -and $ln -notmatch "Write-HcMsg" -and $ln -notmatch "Write-HcStep .+playbook" -and $ln -notmatch "-match 'Write-HcStep") {
                    $issues += ("{0}:{1}" -f $f, ($i+1))
                }
            }
        }
    }
    $dashboardOk = $true
    if (Test-Path "dashboard-ui/package.json") {
        try {
            if (-not (Test-HcCommand "npm")) {
                throw "npm is required for dashboard i18n lint"
            }
            if (-not (Test-Path "dashboard-ui/node_modules")) {
                Invoke-HcPowerShellCommand "Push-Location 'dashboard-ui'; try { npm install --no-audit --no-fund; if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE } } finally { Pop-Location }"
            }
            Invoke-HcPowerShellCommand "Push-Location 'dashboard-ui'; try { npm run check:i18n; if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE } } finally { Pop-Location }"
        } catch {
            $dashboardOk = $false
            $issues += "dashboard-ui:check-i18n"
        }
    }

    $scriptCatalogOk = $true
    try {
        $py = Resolve-HcPython
        Invoke-External $py @("scripts/tools/check_locale_catalog.py")
    } catch {
        $scriptCatalogOk = $false
        $issues += "scripts:check-locale-catalog"
    }

    $out = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        issue_count = $issues.Count
        issues = $issues
        dashboard_i18n_ok = $dashboardOk
        scripts_i18n_ok = $scriptCatalogOk
        ok = ($issues.Count -eq 0 -and $dashboardOk -and $scriptCatalogOk)
    }
    Save-HcJsonFile -Object $out -Path "reports/tooling/i18n_lint.json"
    Write-HcMsg "i18n_lint_path" @("reports/tooling/i18n_lint.json")
    if (-not [bool]$out.ok) {
        Fail-Hc -Code "command_failed" -Message "i18n lint failed"
    }
}

function Run-ReleaseNotes {
    $health = if (Test-Path "reports/tooling/health_report.json") { Get-HcJsonFile -Path "reports/tooling/health_report.json" } else { @{} }
    $trend = if (Test-Path "reports/tooling/trends.json") { Get-HcJsonFile -Path "reports/tooling/trends.json" } else { @{} }
    $an = if (Test-Path "reports/tooling/anomaly_report.json") { Get-HcJsonFile -Path "reports/tooling/anomaly_report.json" } else { @{} }
    $md = @(
        "# HyperCore Release Notes (Auto)",
        "",
        "- generated_utc: $([DateTime]::UtcNow.ToString('o'))",
        "- health_score: $($health.score)",
        "- failures: $($trend.failures)",
        "- failure_rate_pct: $($trend.failure_rate_pct)",
        "- anomaly_alert: $($an.alert)",
        ""
    )
    Set-Content -Path "reports/tooling/release_notes.md" -Value ($md -join "`n") -Encoding UTF8
    Write-HcMsg "release_notes_path" @("reports/tooling/release_notes.md")
}

function Run-TierStatus {
    $py = Resolve-HcPython
    $args = @(
        "scripts/p_tier_status.py",
        "--root", ".",
        "--out-json", "reports/tooling/p_tier_status.json",
        "--out-md", "reports/tooling/p_tier_status.md"
    )
    Invoke-External $py $args
    Write-HcMsg "tier_status_path" @("reports/tooling/p_tier_status.json")
}

function Run-IdempotencyCheck {
    $py = Resolve-HcPython
    Invoke-External $py @("scripts/tools/check_idempotency.py")
    Write-HcMsg "idempotency_check_path" @("reports/tooling/idempotency_check.json")
}

function Run-SecureBootSign {
    $py = Resolve-HcPython
    $args = @("scripts/secureboot_sign.py")
    if ($DryRun) { $args += "--dry-run" }
    Invoke-External $py $args
}

function Run-SecureBootSbat {
    $py = Resolve-HcPython
    Invoke-External $py @("scripts/secureboot_sbat_validate.py")
}

function Run-SecureBootMokPlan {
    $py = Resolve-HcPython
    Invoke-External $py @("scripts/secureboot_mok_plan.py")
}

function Run-SecureBootOvmfMatrix {
    $py = Resolve-HcPython
    $args = @("scripts/secureboot_ovmf_matrix.py")
    if ($DryRun) { $args += "--dry-run" }
    Invoke-External $py $args
}

function Run-SecureBootPcrReport {
    $py = Resolve-HcPython
    Invoke-External $py @("scripts/secureboot_pcr_report.py")
}

function Run-DashboardAgentContract {
    Invoke-HcPowerShellFile "tests/powershell/Run-DashboardAgentContract.ps1"
    Write-HcMsg "step_done"
}

function Run-ToolingQualityGate {
    Write-HcMsg "tooling_quality_gate_start"
    Run-LinuxAbiGate
    Run-LinuxPlatformReadiness
    Run-LintI18n
    Run-ValidateSchemas
    Run-DashboardAgentContract
    Run-DashboardUiTest
    Run-DashboardUiE2E
    Run-IdempotencyCheck
    Write-HcMsg "tooling_quality_gate_done"
}

function Run-OsSmokeDashboard {
    param($Cfg, $Settings)
    Invoke-Op -OpName "os_smoke.doctor_fix" -Action { Run-DoctorFix -Cfg $Cfg -Settings $Settings }
    Invoke-Op -OpName "os_smoke.ci_smoke" -Action { Run-BuildIso; Run-QemuSmoke -Settings $Settings }
    Invoke-Op -OpName "os_smoke.dashboard_data" -Action { Run-ExportDashboardData -Settings $Settings }
    Invoke-Op -OpName "os_smoke.dashboard_html" -Action { Run-HtmlDashboard -Settings $Settings }
    Invoke-Op -OpName "os_smoke.dashboard_ui_build" -Action { Run-DashboardUiBuild -Settings $Settings }
    Write-HcMsg "os_smoke_dashboard_done" @("reports/tooling/dashboard.html")
}

function Run-OsFullDashboard {
    param($Cfg, $Settings)
    Invoke-Op -OpName "os_full.doctor_fix" -Action { Run-DoctorFix -Cfg $Cfg -Settings $Settings }
    Invoke-Op -OpName "os_full.pre_release" -Action { Run-PreRelease -Cfg $Cfg -Settings $Settings }
    Invoke-Op -OpName "os_full.dashboard_data" -Action { Run-ExportDashboardData -Settings $Settings }
    Invoke-Op -OpName "os_full.dashboard_html" -Action { Run-HtmlDashboard -Settings $Settings }
    Invoke-Op -OpName "os_full.dashboard_ui_build" -Action { Run-DashboardUiBuild -Settings $Settings }
    Invoke-Op -OpName "os_full.dashboard_ui_test" -Action { Run-DashboardUiTest }
    Write-HcMsg "os_full_dashboard_done" @("reports/tooling/dashboard.html")
}

function Build-CommandHandlers {
    param($Cfg, $Settings)
    $h = @{}
    $h["doctor"] = { Invoke-Op -OpName "doctor" -Action { Run-Doctor -Settings $Settings } }
    $h["report"] = { Invoke-Op -OpName "report" -Action { Run-Doctor -Settings $Settings } }
    $h["install"] = {
        Invoke-Op -OpName "install" -Action { Run-Install -Cfg $Cfg -Settings $Settings }
        Invoke-Op -OpName "doctor.post_install" -Action { Run-Doctor -Settings $Settings }
    }
    $h["install-deno"] = { Invoke-Op -OpName "install_deno" -Action { Run-InstallDeno } }
    $h["build-iso"] = { Invoke-Op -OpName "build_iso" -Action { Run-BuildIso } }
    $h["qemu-smoke"] = { Invoke-Op -OpName "qemu_smoke" -Action { Run-QemuSmoke -Settings $Settings } }
    $h["qemu-live"] = { Invoke-Op -OpName "qemu_live" -Action { Run-QemuLive } }
    $h["dashboard-agent"] = { Invoke-Op -OpName "dashboard_agent" -Action { Run-DashboardAgent -Settings $settings } }
    $h["dashboard-agent-nosafe"] = { Invoke-Op -OpName "dashboard_agent_nosafe" -Action { Run-DashboardAgent -Settings $settings -UnsafeNoAuth } }
    $h["dashboard-agent-bg"] = { Invoke-Op -OpName "dashboard_agent_bg" -Action { Run-DashboardAgentDetached -Settings $settings } }
    $h["dashboard-agent-nosafe-bg"] = { Invoke-Op -OpName "dashboard_agent_nosafe_bg" -Action { Run-DashboardAgentDetached -Settings $settings -UnsafeNoAuth } }
    $h["dashboard-agent-contract"] = { Invoke-Op -OpName "dashboard_agent_contract" -Action { Run-DashboardAgentContract } }
    $h["ci-smoke"] = {
        Invoke-Op -OpName "ci.build_iso" -Action { Run-BuildIso }
        Invoke-Op -OpName "ci.qemu_smoke" -Action { Run-QemuSmoke -Settings $Settings }
    }
    $h["verify"] = { Invoke-Op -OpName "verify" -Action { Run-Verify -Settings $Settings } }
    $h["list-tasks"] = {
        $tasks = Get-Tasks -Settings $Settings
        foreach ($t in $tasks) { Write-Host ("- {0}: {1}" -f $t.name, $t.description) }
        if ($ListTaskGraph) { Write-TaskGraph -Settings $Settings }
    }
    $h["run-task"] = {
        if (-not $TaskName) { Fail-Hc -Code "config_invalid" -Message "run-task requires -TaskName" }
        Invoke-Op -OpName "run_task.$TaskName" -Action { Run-Task -Settings $Settings -Name $TaskName }
    }
    $h["gate"] = { Invoke-Op -OpName "gate.$GateStage" -Action { Run-Gate -Settings $Settings -Stage $GateStage } }
    $h["cleanup"] = { Invoke-Op -OpName "cleanup" -Action { Run-Cleanup -Settings $Settings } }
    $h["health"] = { Invoke-Op -OpName "health" -Action { Run-Health -Settings $Settings } }
    $h["test-scripts"] = { Invoke-Op -OpName "test_scripts" -Action { Run-ScriptTests } }
    $h["dashboard"] = { Invoke-Op -OpName "dashboard" -Action { Run-Dashboard -Settings $Settings } }
    $h["trends"] = { Invoke-Op -OpName "trends" -Action { Run-Trends -Settings $Settings } }
    $h["collect-diagnostics"] = { Invoke-Op -OpName "collect_diagnostics" -Action { Run-CollectDiagnostics -Settings $Settings } }
    $h["support-bundle"] = { Invoke-Op -OpName "support_bundle" -Action { Run-SupportBundle -Settings $Settings } }
    $h["explain-last-failure"] = { Invoke-Op -OpName "explain_last_failure" -Action { Run-ExplainLastFailure -Settings $Settings } }
    $h["triage"] = { Invoke-Op -OpName "triage" -Action { Run-Triage -Settings $Settings } }
    $h["pre-release"] = { Invoke-Op -OpName "pre_release" -Action { Run-PreRelease -Cfg $Cfg -Settings $Settings } }
    $h["html-dashboard"] = { Invoke-Op -OpName "html_dashboard" -Action { Run-HtmlDashboard -Settings $Settings } }
    $h["check-updates"] = { Invoke-Op -OpName "check_updates" -Action { Run-CheckUpdates -Settings $Settings } }
    $h["plugins"] = { Invoke-Op -OpName "plugins.list" -Action { Run-Plugins -Settings $Settings } }
    $h["run-plugin"] = { Invoke-Op -OpName "plugins.run" -Action { Run-PluginByName -Settings $Settings -Name $PluginName } }
    $h["detect-flaky"] = { Invoke-Op -OpName "detect_flaky" -Action { Run-DetectFlaky -Settings $Settings } }
    $h["doctor-fix"] = { Invoke-Op -OpName "doctor_fix" -Action { Run-DoctorFix -Cfg $Cfg -Settings $Settings } }
    $h["bootstrap"] = { Invoke-Op -OpName "bootstrap" -Action { Run-Bootstrap -Cfg $Cfg -Settings $Settings } }
    $h["first-run"] = { Invoke-Op -OpName "first_run" -Action { Run-FirstRun -Cfg $Cfg -Settings $Settings } }
    $h["help"] = { Invoke-Op -OpName "help" -Action { Run-Help } }
    $h["open-report"] = { Invoke-Op -OpName "open_report" -Action { Run-OpenReport } }
    $h["dry-run-diff"] = { Invoke-Op -OpName "dry_run_diff" -Action { Run-DryRunDiff } }
    $h["prereq-check"] = { Invoke-Op -OpName "prereq_check" -Action { Run-PrereqCheck -Settings $Settings } }
    $h["apply-fix"] = { Invoke-Op -OpName "apply_fix" -Action { Run-ApplyFix -Cfg $Cfg -Settings $Settings } }
    $h["artifact-manifest"] = { Invoke-Op -OpName "artifact_manifest" -Action { Run-ArtifactManifest } }
    $h["verify-artifacts"] = { Invoke-Op -OpName "verify_artifacts" -Action { Run-VerifyArtifacts } }
    $h["anomaly-report"] = { Invoke-Op -OpName "anomaly_report" -Action { Run-AnomalyReport -Settings $Settings } }
    $h["bisect-helper"] = { Invoke-Op -OpName "bisect_helper" -Action { Run-BisectHelper } }
    $h["dependency-drift"] = { Invoke-Op -OpName "dependency_drift" -Action { Run-DependencyDrift -Settings $Settings } }
    $h["policy-gate"] = { Invoke-Op -OpName "policy_gate" -Action { Run-PolicyGate } }
    $h["linux-abi-gate"] = { Invoke-Op -OpName "linux_abi_gate" -Action { Run-LinuxAbiGate } }
    $h["linux-platform-readiness"] = { Invoke-Op -OpName "linux_platform_readiness" -Action { Run-LinuxPlatformReadiness } }
    $h["merge-telemetry"] = { Invoke-Op -OpName "merge_telemetry" -Action { Run-MergeTelemetry -Settings $Settings } }
    $h["validate-schemas"] = { Invoke-Op -OpName "validate_schemas" -Action { Run-ValidateSchemas } }
    $h["canary"] = { Invoke-Op -OpName "canary" -Action { Run-Canary -Cfg $Cfg -Settings $Settings } }
    $h["replay-run"] = { Invoke-Op -OpName "replay_run" -Action { Run-ReplayRun } }
    $h["lint-i18n"] = { Invoke-Op -OpName "lint_i18n" -Action { Run-LintI18n } }
    $h["release-notes"] = { Invoke-Op -OpName "release_notes" -Action { Run-ReleaseNotes } }
    $h["tier-status"] = { Invoke-Op -OpName "tier_status" -Action { Run-TierStatus } }
    $h["idempotency-check"] = { Invoke-Op -OpName "idempotency_check" -Action { Run-IdempotencyCheck } }
    $h["tooling-quality-gate"] = { Invoke-Op -OpName "tooling_quality_gate" -Action { Run-ToolingQualityGate } }
    $h["dashboard-ui-build"] = { Invoke-Op -OpName "dashboard_ui_build" -Action { Run-DashboardUiBuild -Settings $Settings } }
    $h["dashboard-ui-dev"] = { Invoke-Op -OpName "dashboard_ui_dev" -Action { Run-DashboardUiDev -Settings $Settings } }
    $h["dashboard-ui-test"] = { Invoke-Op -OpName "dashboard_ui_test" -Action { Run-DashboardUiTest } }
    $h["dashboard-ui-e2e"] = { Invoke-Op -OpName "dashboard_ui_e2e" -Action { Run-DashboardUiE2E } }
    $h["dashboard-ui-e2e-setup"] = { Invoke-Op -OpName "dashboard_ui_e2e_setup" -Action { Run-DashboardUiE2ESetup } }
    $h["os-smoke-dashboard"] = { Invoke-Op -OpName "os_smoke_dashboard" -Action { Run-OsSmokeDashboard -Cfg $Cfg -Settings $Settings } }
    $h["os-full-dashboard"] = { Invoke-Op -OpName "os_full_dashboard" -Action { Run-OsFullDashboard -Cfg $Cfg -Settings $Settings } }
    $h["secureboot-sign"] = { Invoke-Op -OpName "secureboot_sign" -Action { Run-SecureBootSign } }
    $h["secureboot-sbat"] = { Invoke-Op -OpName "secureboot_sbat" -Action { Run-SecureBootSbat } }
    $h["secureboot-mok-plan"] = { Invoke-Op -OpName "secureboot_mok_plan" -Action { Run-SecureBootMokPlan } }
    $h["secureboot-ovmf-matrix"] = { Invoke-Op -OpName "secureboot_ovmf_matrix" -Action { Run-SecureBootOvmfMatrix } }
    $h["secureboot-pcr-report"] = { Invoke-Op -OpName "secureboot_pcr_report" -Action { Run-SecureBootPcrReport } }
    return $h
}

$repoRoot = Get-HcRepoRoot -ScriptRoot $PSScriptRoot
Set-Location $repoRoot
$cfgRaw = Get-HcJsonFile -Path $ConfigPath
$cfg = Migrate-ConfigIfNeeded -Cfg $cfgRaw
Validate-ConfigSchema -Cfg $cfg
Initialize-LocaleCatalog -Cfg $cfg
$script:ShellExe = Resolve-HostShell
Initialize-UiMode
$script:Settings = Resolve-EffectiveSettings -Cfg $cfg -ProfileName $Profile
$script:CommandHandlers = Build-CommandHandlers -Cfg $cfg -Settings $script:Settings
Initialize-RunManifest

try {
    Acquire-RunLock -Settings $script:Settings
    Add-TelemetryEvent -Event "run.start" -Status "start" -Component "run" -Data @{ offline = [bool]$Offline; use_cache = [bool]$UseCache; dry_run = [bool]$DryRun }
    if (-not $script:CommandHandlers.ContainsKey($Command)) {
        Fail-Hc -Code "config_invalid" -Message ("Unknown command: {0}" -f $Command)
    }
    & $script:CommandHandlers[$Command]
    Add-TelemetryEvent -Event "run.end" -Status "ok" -Component "run"
    $script:RunStatus = "ok"
    Complete-RunManifest -Status "ok" -ExitCode 0
} catch {
    $code = Resolve-ExitCode -ErrorText ([string]$_)
    $codeKey = ($script:ExitCodeMap.Keys | Where-Object { [int]$script:ExitCodeMap[$_] -eq $code } | Select-Object -First 1)
    Add-TelemetryEvent -Event "run.end" -Status "fail" -Level "error" -Code ([string]$codeKey) -Component "run" -Data @{ message = [string]$_ }
    $hint = Get-ErrorPlaybookHint -Code ([string]$codeKey)
    if ($hint) { Write-HcMsg "playbook_hint" @($hint) }
    $script:RunStatus = "fail"
    Complete-RunManifest -Status "fail" -ExitCode $code
    Release-RunLock
    exit $code
}

Release-RunLock
Write-HcMsg "done"

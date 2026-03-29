param(
    [int]$Port = 7401,
    [ValidateSet("quick", "strict")]
    [string]$Profile = "quick",
    [string]$ConfigPath = "",
    [string]$AuthToken = "",
    [string[]]$AllowedOrigins = @(),
    [int]$MaxConcurrency = 1,
    [int]$MaxQueue = 100,
    [int]$LogRetentionDays = 14,
    [switch]$NoSafe,
    [switch]$NoLock
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Web
try {
    [Console]::InputEncoding = [System.Text.UTF8Encoding]::new($false)
    [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
    & chcp 65001 *> $null
} catch {}

$hypercorePath = Join-Path $PSScriptRoot "hypercore.ps1"
if (-not (Test-Path $hypercorePath)) {
    throw "hypercore.ps1 not found: $hypercorePath"
}
$configEnginePath = Join-Path $PSScriptRoot "hypercore/config_engine.ps1"
if (-not (Test-Path $configEnginePath)) {
    throw "config engine script not found: $configEnginePath"
}
. $configEnginePath

if (-not $ConfigPath) {
    $ConfigPath = Join-Path $PSScriptRoot "config/hypercore.defaults.json"
}

$cfgAgent = $null
if (Test-Path $ConfigPath) {
    try {
        $cfgRaw = Get-Content -Raw -Path $ConfigPath | ConvertFrom-Json
        if ($cfgRaw -and $cfgRaw.PSObject.Properties.Name.Contains("agent")) {
            $cfgAgent = $cfgRaw.agent
        }
    } catch {
        Write-Host "[dashboard-agent] warning: failed to parse config at $ConfigPath" -ForegroundColor Yellow
    }
}

$unsafeNoAuth = [bool]$NoSafe

if ($cfgAgent) {
    if (-not $PSBoundParameters.ContainsKey("Port") -and $cfgAgent.PSObject.Properties.Name.Contains("port")) {
        $Port = [int]$cfgAgent.port
    }
    if (-not $AuthToken -and $cfgAgent.PSObject.Properties.Name.Contains("auth_token")) {
        $AuthToken = [string]$cfgAgent.auth_token
    }
    if ((-not $AllowedOrigins -or $AllowedOrigins.Count -eq 0) -and $cfgAgent.PSObject.Properties.Name.Contains("allowed_origins")) {
        $AllowedOrigins = @($cfgAgent.allowed_origins | ForEach-Object { [string]$_ })
    }
    if (-not $PSBoundParameters.ContainsKey("MaxConcurrency") -and $cfgAgent.PSObject.Properties.Name.Contains("max_concurrency")) {
        $MaxConcurrency = [Math]::Max(1, [int]$cfgAgent.max_concurrency)
    }
    if (-not $PSBoundParameters.ContainsKey("MaxQueue") -and $cfgAgent.PSObject.Properties.Name.Contains("max_queue")) {
        $MaxQueue = [Math]::Max(10, [int]$cfgAgent.max_queue)
    }
    if (-not $PSBoundParameters.ContainsKey("LogRetentionDays") -and $cfgAgent.PSObject.Properties.Name.Contains("log_retention_days")) {
        $LogRetentionDays = [Math]::Max(1, [int]$cfgAgent.log_retention_days)
    }
    if (-not $PSBoundParameters.ContainsKey("NoSafe") -and $cfgAgent.PSObject.Properties.Name.Contains("auth_mode")) {
        $unsafeNoAuth = ([string]$cfgAgent.auth_mode).ToLowerInvariant() -eq "unsafe"
    }
}

if (-not $AllowedOrigins -or $AllowedOrigins.Count -eq 0) {
    $AllowedOrigins = @("http://127.0.0.1", "http://localhost", "http://127.0.0.1:5173", "http://localhost:5173")
}

if (-not $AuthToken -or $AuthToken -eq "change-me-hypercore-agent-token") {
    $AuthToken = "hypercore-local-dev-token"
    Write-Host "[dashboard-agent] info: using stable local dev auth token (set agent.auth_token in config to override)." -ForegroundColor Yellow
}

$script:UnsafeNoAuth = [bool]$unsafeNoAuth
if ($script:UnsafeNoAuth) {
    Write-Host "[dashboard-agent] WARNING: NO-SAFE mode enabled. Auth token checks are bypassed." -ForegroundColor Red
}

$script:Recent = New-Object System.Collections.Generic.List[object]
$script:StartedUtc = [DateTime]::UtcNow.ToString("o")
$script:Jobs = @{}
$script:Queue = New-Object System.Collections.Generic.List[string]
$script:AllowedOrigins = $AllowedOrigins
$script:AuthToken = $AuthToken
$script:TokenRoles = @{
    viewer = ""
    operator = ""
    admin = $AuthToken
}
$script:TokenMeta = @{
    viewer = [ordered]@{ updated_utc = "" }
    operator = [ordered]@{ updated_utc = "" }
    admin = [ordered]@{ updated_utc = [DateTime]::UtcNow.ToString("o") }
}
$script:MaxConcurrency = [Math]::Max(1, $MaxConcurrency)
$script:MaxQueue = [Math]::Max(10, $MaxQueue)
$script:AuditDir = "reports/tooling/agent_runs"
$script:LogRetentionDays = [Math]::Max(1, $LogRetentionDays)
$script:PriorityOrder = @{ high = 0; normal = 1; low = 2 }
$script:SchedulerEnabled = $true
$script:Schedules = @{}
$script:Confirmations = @{}
$script:ConfirmationTtlSec = 120
$script:Hosts = @()
$script:RolePolicies = @{}
$script:PolicyTrace = New-Object System.Collections.Generic.List[object]
Initialize-ConfigEngine -ConfigPath $ConfigPath -ScriptRoot $PSScriptRoot

$actions = @(
    @{ id = "doctor"; title = "Check Dependencies"; desc = "Readiness + version checks"; cmd = "doctor"; args = @("-WriteDoctorReport"); risk = "INFO"; category = "diagnostics"; impact = "Read-only host/tool checks." },
    @{ id = "doctor_fix"; title = "Install/Fix Dependencies"; desc = "Auto install missing tools"; cmd = "doctor-fix"; args = @("-AutoApprove"); risk = "HIGH"; category = "install"; impact = "Installs/changes host dependencies." },
    @{ id = "install_deno"; title = "Install Deno Runtime"; desc = "Install or repair Deno runtime on host"; cmd = "install-deno"; args = @(); risk = "MED"; category = "install"; impact = "Installs Deno runtime via package manager." },
    @{ id = "build_iso"; title = "Build ISO"; desc = "Compile kernel and generate ISO"; cmd = "build-iso"; args = @(); risk = "HIGH"; category = "build"; impact = "Writes boot artifacts and ISO outputs." },
    @{ id = "qemu_smoke"; title = "QEMU Smoke"; desc = "Automated boot smoke test"; cmd = "qemu-smoke"; args = @(); risk = "HIGH"; category = "test"; impact = "Runs emulator smoke tests and writes reports." },
    @{ id = "qemu_live"; title = "QEMU Live"; desc = "Open interactive QEMU boot window"; cmd = "qemu-live"; args = @(); risk = "MED"; category = "test"; impact = "Starts interactive emulator window." },
    @{ id = "dashboard_build"; title = "Build Dashboard UI"; desc = "Generate telemetry + build Svelte UI"; cmd = "os-smoke-dashboard"; args = @(); risk = "MED"; category = "dashboard"; impact = "Rebuilds reports and dashboard assets." },
    @{ id = "dashboard_tests"; title = "Dashboard Tests"; desc = "Run unit + e2e tests"; cmd = "dashboard-ui-test"; args = @(); risk = "MED"; category = "test"; impact = "Executes dashboard unit tests." },
    @{ id = "dashboard_e2e"; title = "Dashboard E2E"; desc = "Run browser end-to-end tests"; cmd = "dashboard-ui-e2e"; args = @(); risk = "MED"; category = "test"; impact = "Executes browser E2E automation." },
    @{ id = "quality_gate"; title = "Tooling Quality Gate"; desc = "Run full gate checks"; cmd = "tooling-quality-gate"; args = @(); risk = "HIGH"; category = "gate"; impact = "Runs full quality gate and acceptance checks." },
    @{ id = "open_report"; title = "Open Report"; desc = "Open modern dashboard report"; cmd = "open-report"; args = @("-ReportTarget", "ui"); risk = "INFO"; category = "dashboard"; impact = "Opens local report UI in browser." },
    @{ id = "crash_diagnostics"; title = "Crash Diagnostics Bundle"; desc = "Collect diagnostics artifact bundle"; cmd = "collect-diagnostics"; args = @(); risk = "MED"; category = "recovery"; impact = "Collects diagnostics zip and metadata for triage." },
    @{ id = "crash_triage"; title = "Crash Triage"; desc = "Generate triage report from recent failures"; cmd = "triage"; args = @(); risk = "MED"; category = "recovery"; impact = "Builds actionable failure triage report." }
)

$actionMap = @{}
foreach ($a in $actions) { $actionMap[$a.id] = $a }

if ($cfgAgent -and $cfgAgent.PSObject.Properties.Name.Contains("tokens")) {
    $tk = $cfgAgent.tokens
    foreach ($rk in @("viewer","operator","admin")) {
        if ($tk.PSObject.Properties.Name.Contains($rk)) {
            $script:TokenRoles[$rk] = [string]$tk.$rk
        }
    }
}
if (-not $script:TokenRoles["admin"]) {
    $script:TokenRoles["admin"] = $AuthToken
}
if (-not $script:TokenMeta["admin"].updated_utc) {
    $script:TokenMeta["admin"].updated_utc = [DateTime]::UtcNow.ToString("o")
}

function Initialize-Scheduler {
    $tasks = @(
        [ordered]@{ id = "nightly_smoke"; action = "qemu_smoke"; interval_sec = 86400; priority = "low"; enabled = $true },
        [ordered]@{ id = "weekly_quality_gate"; action = "quality_gate"; interval_sec = 604800; priority = "low"; enabled = $true },
        [ordered]@{ id = "dashboard_refresh"; action = "dashboard_build"; interval_sec = 21600; priority = "low"; enabled = $true }
    )

    if ($cfgAgent -and $cfgAgent.PSObject.Properties.Name.Contains("scheduler")) {
        $sc = $cfgAgent.scheduler
        if ($sc.PSObject.Properties.Name.Contains("enabled")) { $script:SchedulerEnabled = [bool]$sc.enabled }
        if ($sc.PSObject.Properties.Name.Contains("tasks") -and $sc.tasks) {
            $tasks = @($sc.tasks)
        }
    }

    foreach ($t in $tasks) {
        $id = [string]$t.id
        $action = [string]$t.action
        if (-not $id -or -not $action -or -not $actionMap.ContainsKey($action)) { continue }
        $interval = [Math]::Max(60, [int]$t.interval_sec)
        $priority = [string]$t.priority
        if (-not $script:PriorityOrder.ContainsKey($priority)) { $priority = "low" }
        $enabled = $true
        if ($t.PSObject.Properties.Name.Contains("enabled")) { $enabled = [bool]$t.enabled }
        $now = [DateTime]::UtcNow
        $script:Schedules[$id] = [ordered]@{
            id = $id
            action = $action
            interval_sec = $interval
            priority = $priority
            enabled = $enabled
            last_run_utc = ""
            next_run_utc = $now.AddSeconds($interval).ToString("o")
            source = "scheduler:$id"
        }
    }
}

function Get-SchedulerTemplates {
    return @(
        [ordered]@{
            id = "balanced_default"
            title = "Balanced Default"
            description = "Daily smoke, weekly quality gate, and periodic dashboard refresh."
            scheduler_enabled = $true
            tasks = @(
                [ordered]@{ id = "nightly_smoke"; action = "qemu_smoke"; interval_sec = 86400; priority = "low"; enabled = $true },
                [ordered]@{ id = "weekly_quality_gate"; action = "quality_gate"; interval_sec = 604800; priority = "low"; enabled = $true },
                [ordered]@{ id = "dashboard_refresh"; action = "dashboard_build"; interval_sec = 21600; priority = "low"; enabled = $true }
            )
        },
        [ordered]@{
            id = "release_hardening"
            title = "Release Hardening"
            description = "More frequent smoke and gate cadence for RC periods."
            scheduler_enabled = $true
            tasks = @(
                [ordered]@{ id = "release_smoke_6h"; action = "qemu_smoke"; interval_sec = 21600; priority = "normal"; enabled = $true },
                [ordered]@{ id = "release_gate_daily"; action = "quality_gate"; interval_sec = 86400; priority = "normal"; enabled = $true },
                [ordered]@{ id = "release_dashboard_2h"; action = "dashboard_build"; interval_sec = 7200; priority = "low"; enabled = $true }
            )
        },
        [ordered]@{
            id = "diagnostics_focus"
            title = "Diagnostics Focus"
            description = "Frequent diagnostics for unstable periods."
            scheduler_enabled = $true
            tasks = @(
                [ordered]@{ id = "diag_crash_bundle_4h"; action = "crash_diagnostics"; interval_sec = 14400; priority = "normal"; enabled = $true },
                [ordered]@{ id = "diag_triage_4h"; action = "crash_triage"; interval_sec = 14400; priority = "normal"; enabled = $true },
                [ordered]@{ id = "diag_smoke_12h"; action = "qemu_smoke"; interval_sec = 43200; priority = "low"; enabled = $true }
            )
        }
    )
}

function Apply-SchedulerTemplate([string]$templateId) {
    $templates = Get-SchedulerTemplates
    $tpl = $null
    foreach ($t in $templates) {
        if ([string]$t.id -eq $templateId) {
            $tpl = $t
            break
        }
    }
    if ($null -eq $tpl) { return $false }

    $script:Schedules = @{}
    $script:SchedulerEnabled = [bool]$tpl.scheduler_enabled
    foreach ($task in @($tpl.tasks)) {
        $id = [string]$task.id
        $action = [string]$task.action
        if (-not $id -or -not $action -or -not $actionMap.ContainsKey($action)) { continue }
        $priority = [string]$task.priority
        if (-not $script:PriorityOrder.ContainsKey($priority)) { $priority = "low" }
        $interval = [Math]::Max(60, [int]$task.interval_sec)
        $enabled = if ($task.PSObject.Properties.Name.Contains("enabled")) { [bool]$task.enabled } else { $true }
        $now = [DateTime]::UtcNow
        $script:Schedules[$id] = [ordered]@{
            id = $id
            action = $action
            interval_sec = $interval
            priority = $priority
            enabled = $enabled
            last_run_utc = ""
            next_run_utc = $now.AddSeconds($interval).ToString("o")
            source = "scheduler:$id"
        }
    }
    return $true
}

function Initialize-Hosts {
    $script:Hosts = @()
    $script:Hosts += [ordered]@{
        id = "local"
        name = "Localhost"
        url = ("http://127.0.0.1:{0}" -f $Port)
        enabled = $true
        role_hint = "admin"
    }
    if ($cfgAgent -and $cfgAgent.PSObject.Properties.Name.Contains("hosts")) {
        foreach ($h in @($cfgAgent.hosts)) {
            $id = [string]$h.id
            $url = [string]$h.url
            if (-not $id -or -not $url) { continue }
            if ($id -eq "local") { continue }
            $script:Hosts += [ordered]@{
                id = $id
                name = if ($h.name) { [string]$h.name } else { $id }
                url = $url
                enabled = if ($h.PSObject.Properties.Name.Contains("enabled")) { [bool]$h.enabled } else { $true }
                role_hint = if ($h.role_hint) { [string]$h.role_hint } else { "operator" }
                token = if ($h.PSObject.Properties.Name.Contains("token")) { [string]$h.token } else { "" }
            }
        }
    }
}

function Normalize-HostEntry($h) {
    if ($null -eq $h) { return $null }
    $id = [string]$h.id
    $url = [string]$h.url
    if (-not $id -or -not $url) { return $null }
    return [ordered]@{
        id = $id
        name = if ($h.name) { [string]$h.name } else { $id }
        url = $url
        enabled = if ($h.PSObject.Properties.Name.Contains("enabled")) { [bool]$h.enabled } else { $true }
        role_hint = if ($h.role_hint) { [string]$h.role_hint } else { "operator" }
        token = if ($h.PSObject.Properties.Name.Contains("token")) { [string]$h.token } else { "" }
    }
}

function Upsert-HostEntry($inputHost) {
    $normalized = Normalize-HostEntry $inputHost
    if ($null -eq $normalized) { return $false }
    if ([string]$normalized.id -eq "local") { return $false }
    $next = @()
    $found = $false
    foreach ($h in @($script:Hosts)) {
        if ([string]$h.id -eq [string]$normalized.id) {
            $next += $normalized
            $found = $true
        } else {
            $next += $h
        }
    }
    if (-not $found) { $next += $normalized }
    $script:Hosts = $next
    return $true
}

function Remove-HostEntry([string]$hostId) {
    if (-not $hostId -or $hostId -eq "local") { return $false }
    $before = @($script:Hosts).Count
    $script:Hosts = @($script:Hosts | Where-Object { [string]$_.id -ne $hostId })
    return (@($script:Hosts).Count -lt $before)
}

function Initialize-RolePolicies {
    $script:RolePolicies = @{
        viewer = [ordered]@{
            max_risk = "INFO"
            denied_actions = @("*")
            denied_categories = @()
        }
        operator = [ordered]@{
            max_risk = "HIGH"
            denied_actions = @("doctor_fix", "quality_gate")
            denied_categories = @()
        }
        admin = [ordered]@{
            max_risk = "HIGH"
            denied_actions = @()
            denied_categories = @()
        }
    }

    if ($cfgAgent -and $cfgAgent.PSObject.Properties.Name.Contains("policy")) {
        $pol = $cfgAgent.policy
        if ($pol -and $pol.PSObject.Properties.Name.Contains("roles")) {
            foreach ($rk in @("viewer","operator","admin")) {
                if (-not $pol.roles.PSObject.Properties.Name.Contains($rk)) { continue }
                $rp = $pol.roles.$rk
                $entry = $script:RolePolicies[$rk]
                if ($rp.PSObject.Properties.Name.Contains("max_risk")) { $entry.max_risk = [string]$rp.max_risk }
                if ($rp.PSObject.Properties.Name.Contains("denied_actions")) { $entry.denied_actions = @($rp.denied_actions | ForEach-Object { [string]$_ }) }
                if ($rp.PSObject.Properties.Name.Contains("denied_categories")) { $entry.denied_categories = @($rp.denied_categories | ForEach-Object { [string]$_ }) }
            }
        }
    }
}

function Get-DefaultRolePolicies {
    return @{
        viewer = [ordered]@{
            max_risk = "INFO"
            denied_actions = @("*")
            denied_categories = @()
        }
        operator = [ordered]@{
            max_risk = "HIGH"
            denied_actions = @("doctor_fix", "quality_gate")
            denied_categories = @()
        }
        admin = [ordered]@{
            max_risk = "HIGH"
            denied_actions = @()
            denied_categories = @()
        }
    }
}

function ConvertTo-SafeStringArray($value) {
    if ($null -eq $value) { return @() }
    $out = @()
    foreach ($it in @($value)) {
        $s = [string]$it
        if ($s) { $out += $s }
    }
    return @($out | Select-Object -Unique)
}

function Normalize-Risk([string]$risk, [string]$fallback = "INFO") {
    $val = [string]$risk
    if (-not $val) { return $fallback }
    $up = $val.ToUpperInvariant()
    if ($up -in @("INFO", "MED", "HIGH")) { return $up }
    return $fallback
}

function New-NormalizedRolePolicy([string]$roleName, $inputRolePolicy) {
    $defaults = Get-DefaultRolePolicies
    $fallback = $defaults[$roleName]
    $entry = [ordered]@{
        max_risk = Normalize-Risk -risk ([string]$fallback.max_risk) -fallback "INFO"
        denied_actions = ConvertTo-SafeStringArray $fallback.denied_actions
        denied_categories = ConvertTo-SafeStringArray $fallback.denied_categories
    }
    if ($null -eq $inputRolePolicy) { return $entry }
    if ($inputRolePolicy.PSObject.Properties.Name.Contains("max_risk")) {
        $entry.max_risk = Normalize-Risk -risk ([string]$inputRolePolicy.max_risk) -fallback $entry.max_risk
    }
    if ($inputRolePolicy.PSObject.Properties.Name.Contains("denied_actions")) {
        $entry.denied_actions = ConvertTo-SafeStringArray $inputRolePolicy.denied_actions
    }
    if ($inputRolePolicy.PSObject.Properties.Name.Contains("denied_categories")) {
        $entry.denied_categories = ConvertTo-SafeStringArray $inputRolePolicy.denied_categories
    }
    return $entry
}

function New-NormalizedRolePolicies($rolesObjectOrArray) {
    $result = @{}
    foreach ($rk in @("viewer","operator","admin")) {
        $result[$rk] = New-NormalizedRolePolicy -roleName $rk -inputRolePolicy $null
    }
    if ($null -eq $rolesObjectOrArray) { return $result }

    if ($rolesObjectOrArray -is [System.Array] -or $rolesObjectOrArray -is [System.Collections.IEnumerable]) {
        foreach ($row in @($rolesObjectOrArray)) {
            if ($null -eq $row) { continue }
            $rk = [string]$row.role
            if (-not $rk -or -not ($rk -in @("viewer","operator","admin"))) { continue }
            $result[$rk] = New-NormalizedRolePolicy -roleName $rk -inputRolePolicy $row
        }
        return $result
    }

    foreach ($rk in @("viewer","operator","admin")) {
        if ($rolesObjectOrArray.PSObject.Properties.Name.Contains($rk)) {
            $result[$rk] = New-NormalizedRolePolicy -roleName $rk -inputRolePolicy $rolesObjectOrArray.$rk
        }
    }
    return $result
}

function Set-RolePoliciesFromInput($rolesObjectOrArray, [string]$source = "api") {
    $next = New-NormalizedRolePolicies $rolesObjectOrArray
    $script:RolePolicies = $next
    Push-PolicyTrace ([ordered]@{
        ts_utc = [DateTime]::UtcNow.ToString("o")
        source = $source
        role = "admin"
        action = "policy_apply"
        category = "policy"
        risk = "INFO"
        allowed = $true
        reason = "policy_updated"
    })
}

function Get-PolicySnapshotPayload {
    $policyRows = @()
    foreach ($rk in @("viewer","operator","admin")) {
        $policyRows += [ordered]@{
            role = $rk
            max_risk = [string]$script:RolePolicies[$rk].max_risk
            denied_actions = @($script:RolePolicies[$rk].denied_actions)
            denied_categories = @($script:RolePolicies[$rk].denied_categories)
        }
    }
    $actionRows = @()
    foreach ($a in $actions) {
        $actionRows += [ordered]@{
            id = [string]$a.id
            category = [string]$a.category
            risk = [string]$a.risk
        }
    }
    return [ordered]@{
        roles = $policyRows
        actions = $actionRows
    }
}

function Is-OriginAllowed([string]$origin) {
    if (-not $origin) { return $true }
    foreach ($allowed in $script:AllowedOrigins) {
        $rule = [string]$allowed
        if ($rule -eq "*") { return $true }
        if ($rule -eq "null" -and $origin -eq "null") { return $true }
        if ($origin.StartsWith($rule, [System.StringComparison]::OrdinalIgnoreCase)) { return $true }
    }
    return $false
}

function Add-CorsHeaders($req, $resp) {
    $origin = [string]$req.Headers["Origin"]
    if (Is-OriginAllowed $origin) {
        if ($origin) {
            $resp.Headers["Access-Control-Allow-Origin"] = $origin
        } else {
            $resp.Headers["Access-Control-Allow-Origin"] = "http://127.0.0.1"
        }
        $resp.Headers["Vary"] = "Origin"
        $resp.Headers["Access-Control-Allow-Methods"] = "GET,POST,OPTIONS"
        $resp.Headers["Access-Control-Allow-Headers"] = "Content-Type,X-HyperCore-Token"
    }
}

function Write-ApiResponse($ctx, [int]$StatusCode, [bool]$Ok, [string]$Code, [string]$Message, [string]$Error, $Details, $Data) {
    $resp = $ctx.Response
    Add-CorsHeaders $ctx.Request $resp
    $payload = [ordered]@{
        ok = $Ok
        error = if ($Ok) { $null } else { $Error }
        code = $Code
        message = $Message
    }
    if ($Details -ne $null) { $payload["details"] = $Details }
    if ($Data -is [hashtable] -or $Data -is [System.Collections.Specialized.OrderedDictionary]) {
        foreach ($k in $Data.Keys) { $payload[$k] = $Data[$k] }
    }

    $resp.StatusCode = $StatusCode
    $resp.ContentType = "application/json; charset=utf-8"
    $json = ($payload | ConvertTo-Json -Depth 12)
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($json)
    try {
        $resp.OutputStream.Write($bytes, 0, $bytes.Length)
        try { $resp.OutputStream.Flush() } catch {}
    } catch {
        # Client may have disconnected; avoid crashing the agent loop.
    } finally {
        try { $resp.Close() } catch {
            try { $resp.OutputStream.Close() } catch {}
        }
    }
}

function Write-Ok($ctx, [int]$StatusCode, [string]$Message = "ok", $Data = @{}) {
    Write-ApiResponse $ctx $StatusCode $true "ok" $Message $null $null $Data
}

function Write-Err($ctx, [int]$StatusCode, [string]$Code, [string]$Message, [string]$Error = "request_failed", $Details = $null) {
    Write-ApiResponse $ctx $StatusCode $false $Code $Message $Error $Details @{}
}

function Read-RequestJson($req) {
    $reader = New-Object System.IO.StreamReader($req.InputStream, $req.ContentEncoding)
    try { $raw = $reader.ReadToEnd() } finally { $reader.Dispose() }
    if (-not $raw) { return @{} }
    return ($raw | ConvertFrom-Json)
}

function Get-QueryMap($req) {
    $out = @{}
    $q = [string]$req.Url.Query
    if (-not $q) { return $out }
    $q = $q.TrimStart('?')
    foreach ($pair in ($q -split '&')) {
        if (-not $pair) { continue }
        $kv = $pair -split '=', 2
        $k = [System.Web.HttpUtility]::UrlDecode($kv[0])
        $v = if ($kv.Count -gt 1) { [System.Web.HttpUtility]::UrlDecode($kv[1]) } else { "" }
        $out[$k] = $v
    }
    return $out
}

function Test-BodyAllowedKeys($body, [string[]]$allowedKeys, [ref]$unknownKeys) {
    $unknown = @()
    if ($null -eq $body) {
        $unknownKeys.Value = @()
        return $true
    }
    foreach ($p in $body.PSObject.Properties) {
        if (-not ($allowedKeys -contains [string]$p.Name)) {
            $unknown += [string]$p.Name
        }
    }
    $unknownKeys.Value = $unknown
    return ($unknown.Count -eq 0)
}

function Validate-RunPayload($ctx, $body) {
    $unknown = @()
    if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("action","priority","confirmation_id") -unknownKeys ([ref]$unknown))) {
        Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
        return $false
    }
    $priority = [string]$body.priority
    if ($priority -and -not $script:PriorityOrder.ContainsKey($priority)) {
        Write-Err $ctx 400 "invalid_priority" "priority must be one of high|normal|low." "invalid_payload" @{ priority = $priority }
        return $false
    }
    return $true
}

function Validate-CancelPayload($ctx, $body) {
    $unknown = @()
    if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("id") -unknownKeys ([ref]$unknown))) {
        Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
        return $false
    }
    return $true
}

function Validate-SchedulerRunPayload($ctx, $body) {
    $unknown = @()
    if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("id") -unknownKeys ([ref]$unknown))) {
        Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
        return $false
    }
    return $true
}

function Validate-SchedulerUpdatePayload($ctx, $body) {
    $unknown = @()
    if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("id","enabled","interval_sec","priority") -unknownKeys ([ref]$unknown))) {
        Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
        return $false
    }
    if ($body.PSObject.Properties.Name.Contains("priority")) {
        $priority = [string]$body.priority
        if (-not $script:PriorityOrder.ContainsKey($priority)) {
            Write-Err $ctx 400 "invalid_priority" "priority must be one of high|normal|low." "invalid_payload" @{ priority = $priority }
            return $false
        }
    }
    if ($body.PSObject.Properties.Name.Contains("interval_sec")) {
        $interval = [int]$body.interval_sec
        if ($interval -lt 60) {
            Write-Err $ctx 400 "invalid_interval" "interval_sec must be >= 60." "invalid_payload" @{ interval_sec = $interval }
            return $false
        }
    }
    return $true
}

function Validate-DispatchPayload($ctx, $body) {
    $unknown = @()
    if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("host_id","action","priority","confirmation_id") -unknownKeys ([ref]$unknown))) {
        Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
        return $false
    }
    $priority = [string]$body.priority
    if ($priority -and -not $script:PriorityOrder.ContainsKey($priority)) {
        Write-Err $ctx 400 "invalid_priority" "priority must be one of high|normal|low." "invalid_payload" @{ priority = $priority }
        return $false
    }
    return $true
}

function Is-MutatingRequest($req, [string]$path) {
    if ($req.HttpMethod -ne "POST") { return $false }
    return @("run", "run_async", "dispatch/run_async", "dispatch/fanout", "dispatch/job/cancel", "job/cancel", "scheduler/run_now", "scheduler/update", "scheduler/apply_template", "confirm/request", "confirm/revoke", "config/update", "config/auto", "config/import", "config/compose/apply", "config/drift/apply", "policy/apply", "policy/reset", "policy/validate", "auth/rotate", "hosts/register", "hosts/update", "hosts/remove", "roadmap/master/update", "roadmap/batch/record") -contains $path
}

function Test-AuthToken($req) {
    if ($script:UnsafeNoAuth) { return $true }
    $provided = [string]$req.Headers["X-HyperCore-Token"]
    if (-not $provided) { return $false }
    foreach ($rk in @("viewer","operator","admin")) {
        $token = [string]$script:TokenRoles[$rk]
        if ($token -and $provided -eq $token) { return $true }
    }
    return $false
}

function New-SecureToken {
    $a = [Guid]::NewGuid().ToString("N")
    $b = [Guid]::NewGuid().ToString("N")
    return ("{0}{1}" -f $a, $b)
}

function Mark-TokenUpdated([string]$role) {
    if (-not $script:TokenMeta.ContainsKey($role)) {
        $script:TokenMeta[$role] = [ordered]@{ updated_utc = "" }
    }
    $script:TokenMeta[$role].updated_utc = [DateTime]::UtcNow.ToString("o")
}

function Get-AuthStatusPayload {
    $rows = @()
    foreach ($rk in @("viewer","operator","admin")) {
        $token = [string]$script:TokenRoles[$rk]
        $meta = if ($script:TokenMeta.ContainsKey($rk)) { $script:TokenMeta[$rk] } else { [ordered]@{ updated_utc = "" } }
        $rows += [ordered]@{
            role = $rk
            has_token = [bool]$token
            updated_utc = [string]$meta.updated_utc
        }
    }
    return [ordered]@{
        auth_mode = if ($script:UnsafeNoAuth) { "unsafe" } else { "strict" }
        roles = $rows
    }
}

function Get-RoadmapStatusPayload {
    $repoRoot = Split-Path -Parent $PSScriptRoot
    $path = Join-Path $repoRoot "dashboard-ui/src/generated/roadmap_status.json"
    $fallbackTier = {
        param([string]$scope)
        return [ordered]@{
            done = 0
            total = 1
            remaining = 1
            scope = $scope
            done_items = @()
            remaining_items = @()
        }
    }
    $fallback = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        p0 = (& $fallbackTier "core")
        p1 = (& $fallbackTier "feature_depth")
        p2 = (& $fallbackTier "hardening")
        source = "fallback"
        path = $path
    }
    if (-not (Test-Path $path)) { return $fallback }
    try {
        $raw = Get-Content -Raw -Path $path | ConvertFrom-Json
        $normalizeTier = {
            param($r, [string]$fallbackScope)
            $done = [int]$r.done
            $total = [Math]::Max(1, [int]$r.total)
            $remaining = if ($r.PSObject.Properties.Name.Contains("remaining")) {
                [Math]::Max(0, [int]$r.remaining)
            } else {
                [Math]::Max(0, $total - $done)
            }
            return [ordered]@{
                done = $done
                total = $total
                remaining = $remaining
                scope = if ($r.PSObject.Properties.Name.Contains("scope")) { [string]$r.scope } else { $fallbackScope }
                done_items = if ($r.PSObject.Properties.Name.Contains("done_items")) { @($r.done_items | ForEach-Object { [string]$_ }) } else { @() }
                remaining_items = if ($r.PSObject.Properties.Name.Contains("remaining_items")) { @($r.remaining_items | ForEach-Object { [string]$_ }) } else { @() }
            }
        }
        return [ordered]@{
            generated_utc = [string]$raw.generated_utc
            p0 = (& $normalizeTier $raw.p0 "core")
            p1 = (& $normalizeTier $raw.p1 "feature_depth")
            p2 = (& $normalizeTier $raw.p2 "hardening")
            source = "dashboard-ui/src/generated/roadmap_status.json"
            path = $path
        }
    } catch {
        return $fallback
    }
}

function Get-RoadmapStatePath {
    $repoRoot = Split-Path -Parent $PSScriptRoot
    $dir = Join-Path $repoRoot "reports/tooling"
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
    return (Join-Path $dir "dashboard_roadmap_state.json")
}

function Read-RoadmapState {
    $path = Get-RoadmapStatePath
    $fallback = [ordered]@{
        item_meta = @{}
        batch_history = @()
        updated_utc = ""
        path = $path
    }
    if (-not (Test-Path $path)) { return $fallback }
    try {
        $raw = Get-Content -Raw -Path $path | ConvertFrom-Json
        $meta = @{}
        if ($raw -and $raw.PSObject.Properties.Name.Contains("item_meta") -and $raw.item_meta) {
            foreach ($p in $raw.item_meta.PSObject.Properties) {
                $id = [string]$p.Name
                $v = $p.Value
                $meta[$id] = [ordered]@{
                    owner = if ($v.PSObject.Properties.Name.Contains("owner")) { [string]$v.owner } else { "" }
                    eta_utc = if ($v.PSObject.Properties.Name.Contains("eta_utc")) { [string]$v.eta_utc } else { "" }
                    updated_utc = if ($v.PSObject.Properties.Name.Contains("updated_utc")) { [string]$v.updated_utc } else { "" }
                }
            }
        }
        $history = @()
        if ($raw -and $raw.PSObject.Properties.Name.Contains("batch_history")) {
            foreach ($h in @($raw.batch_history)) {
                $history += [ordered]@{
                    phase = [string]$h.phase
                    actions = @($h.actions | ForEach-Object { [string]$_ })
                    ok = [int]$h.ok
                    fail = [int]$h.fail
                    started_utc = [string]$h.started_utc
                    ended_utc = [string]$h.ended_utc
                    duration_ms = [int]$h.duration_ms
                }
            }
        }
        return [ordered]@{
            item_meta = $meta
            batch_history = $history
            updated_utc = if ($raw -and $raw.PSObject.Properties.Name.Contains("updated_utc")) { [string]$raw.updated_utc } else { "" }
            path = $path
        }
    } catch {
        return $fallback
    }
}

function Write-RoadmapState($state) {
    $path = Get-RoadmapStatePath
    $payload = [ordered]@{
        item_meta = if ($state -and $state.item_meta) { $state.item_meta } else { @{} }
        batch_history = if ($state -and $state.batch_history) { @($state.batch_history) } else { @() }
        updated_utc = [DateTime]::UtcNow.ToString("o")
    }
    $json = ($payload | ConvertTo-Json -Depth 12)
    Set-Content -Path $path -Value $json -Encoding UTF8
    return $path
}

function Normalize-RoadmapMetaText([string]$text) {
    return [string]([string]$text).Trim()
}

function Normalize-RoadmapEta([string]$etaText) {
    $raw = [string]$etaText
    if (-not $raw) { return "" }
    $trim = $raw.Trim()
    if (-not $trim) { return "" }
    $dt = $null
    if ([DateTime]::TryParse($trim, [ref]$dt)) {
        return ([DateTime]$dt).ToUniversalTime().ToString("o")
    }
    throw "invalid_eta_utc"
}

function Update-RoadmapItemMeta([string]$itemId, [string]$owner, [string]$etaUtc) {
    $state = Read-RoadmapState
    $id = [string]$itemId
    if (-not ($id -match "^P[0-2]-\d{3}$")) {
        return [ordered]@{ ok = $false; code = "invalid_item_id"; message = "item_id must match P0-001 style."; item_id = $itemId }
    }
    $normalizedOwner = Normalize-RoadmapMetaText $owner
    try {
        $normalizedEta = Normalize-RoadmapEta $etaUtc
    } catch {
        return [ordered]@{ ok = $false; code = "invalid_eta_utc"; message = "eta_utc must be parseable datetime."; eta_utc = $etaUtc }
    }
    if ($normalizedOwner -or $normalizedEta) {
        $state.item_meta[$id] = [ordered]@{
            owner = $normalizedOwner
            eta_utc = $normalizedEta
            updated_utc = [DateTime]::UtcNow.ToString("o")
        }
    } elseif ($state.item_meta.ContainsKey($id)) {
        $state.item_meta.Remove($id)
    }
    $path = Write-RoadmapState $state
    return [ordered]@{
        ok = $true
        item_id = $id
        owner = $normalizedOwner
        eta_utc = $normalizedEta
        path = $path
    }
}

function Record-RoadmapBatchHistory([string]$phase, $actions, [int]$ok, [int]$fail, [string]$startedUtc, [int]$durationMs = 0) {
    $state = Read-RoadmapState
    $phaseText = [string]$phase
    if (-not ($phaseText -in @("P0","P1","P2","p0","p1","p2"))) {
        return [ordered]@{ ok = $false; code = "invalid_phase"; message = "phase must be P0|P1|P2."; phase = $phase }
    }
    $actionsSafe = @()
    foreach ($a in @($actions)) {
        $x = [string]$a
        if ($x) { $actionsSafe += $x }
    }
    $now = [DateTime]::UtcNow.ToString("o")
    $record = [ordered]@{
        phase = ([string]$phaseText).ToUpperInvariant()
        actions = @($actionsSafe | Select-Object -Unique)
        ok = [Math]::Max(0, [int]$ok)
        fail = [Math]::Max(0, [int]$fail)
        started_utc = if ($startedUtc) { [string]$startedUtc } else { $now }
        ended_utc = $now
        duration_ms = [Math]::Max(0, [int]$durationMs)
    }
    $history = @($state.batch_history)
    $history = @($record) + $history
    if ($history.Count -gt 80) {
        $history = $history[0..79]
    }
    $state.batch_history = $history
    $path = Write-RoadmapState $state
    return [ordered]@{
        ok = $true
        record = $record
        count = @($history).Count
        path = $path
    }
}

function Get-MasterBacklogStatusPayload {
    $repoRoot = Split-Path -Parent $PSScriptRoot
    $path = Join-Path $repoRoot "dashboard-ui/docs/MASTER_BACKLOG.md"
    $state = Read-RoadmapState
    $fallback = [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        path = $path
        exists = $false
        total = 0
        done = 0
        remaining = 0
        batch_history = @($state.batch_history)
        tiers = [ordered]@{
            p0 = [ordered]@{ total = 0; done = 0; remaining = 0; remaining_items = @(); remaining_rows = @() }
            p1 = [ordered]@{ total = 0; done = 0; remaining = 0; remaining_items = @(); remaining_rows = @() }
            p2 = [ordered]@{ total = 0; done = 0; remaining = 0; remaining_items = @(); remaining_rows = @() }
        }
    }
    if (-not (Test-Path $path)) { return $fallback }
    try {
        $lines = Get-Content -Path $path
        $tier = ""
        $bucket = @{
            p0 = [ordered]@{ total = 0; done = 0; remaining = 0; remaining_items = @(); remaining_rows = @() }
            p1 = [ordered]@{ total = 0; done = 0; remaining = 0; remaining_items = @(); remaining_rows = @() }
            p2 = [ordered]@{ total = 0; done = 0; remaining = 0; remaining_items = @(); remaining_rows = @() }
        }
        foreach ($lineRaw in $lines) {
            $line = [string]$lineRaw
            $trim = $line.Trim()
            if ($trim -eq "## P0") { $tier = "p0"; continue }
            if ($trim -eq "## P1") { $tier = "p1"; continue }
            if ($trim -eq "## P2") { $tier = "p2"; continue }
            if (-not $tier) { continue }
            if ($trim -match "^- \[([xX ])\] (.+)$") {
                $state = [string]$Matches[1]
                $item = [string]$Matches[2]
                $bucket[$tier].total += 1
                if ($state -eq "x" -or $state -eq "X") {
                    $bucket[$tier].done += 1
                } else {
                    $bucket[$tier].remaining += 1
                    $bucket[$tier].remaining_items += $item
                    $idMatch = [System.Text.RegularExpressions.Regex]::Match([string]$item, '^(P[0-2]-\d{3})\b')
                    $itemId = if ($idMatch.Success) { [string]$idMatch.Groups[1].Value } else { "" }
                    $meta = $null
                    if ($itemId -and $state.item_meta.ContainsKey($itemId)) {
                        $meta = $state.item_meta[$itemId]
                    }
                    $bucket[$tier].remaining_rows += [ordered]@{
                        id = $itemId
                        text = [string]$item
                        owner = if ($meta) { [string]$meta.owner } else { "" }
                        eta_utc = if ($meta) { [string]$meta.eta_utc } else { "" }
                    }
                }
            }
        }
        $total = $bucket.p0.total + $bucket.p1.total + $bucket.p2.total
        $done = $bucket.p0.done + $bucket.p1.done + $bucket.p2.done
        $remaining = $bucket.p0.remaining + $bucket.p1.remaining + $bucket.p2.remaining
        return [ordered]@{
            generated_utc = [DateTime]::UtcNow.ToString("o")
            path = $path
            exists = $true
            total = $total
            done = $done
            remaining = $remaining
            batch_history = @($state.batch_history)
            tiers = [ordered]@{
                p0 = $bucket.p0
                p1 = $bucket.p1
                p2 = $bucket.p2
            }
        }
    } catch {
        return $fallback
    }
}

function Update-MasterBacklogItemStatus([string]$itemId, [bool]$done) {
    $repoRoot = Split-Path -Parent $PSScriptRoot
    $path = Join-Path $repoRoot "dashboard-ui/docs/MASTER_BACKLOG.md"
    if (-not (Test-Path $path)) {
        return [ordered]@{ ok = $false; code = "missing_backlog"; message = "MASTER_BACKLOG.md not found."; path = $path }
    }
    $id = [string]$itemId
    if (-not $id -or -not ($id -match "^P[0-2]-\d{3}$")) {
        return [ordered]@{ ok = $false; code = "invalid_item_id"; message = "item_id must match P0-001 style."; item_id = $itemId }
    }
    $lines = @((Get-Content -Path $path))
    $changed = $false
    $targetState = if ($done) { "x" } else { " " }
    for ($i = 0; $i -lt $lines.Count; $i++) {
        $line = [string]$lines[$i]
        $m = [System.Text.RegularExpressions.Regex]::Match($line, '^(\s*-\s*\[)([xX ])(\]\s+)(P[0-2]-\d{3}\b.*)$')
        if (-not $m.Success) { continue }
        $tail = [string]$m.Groups[4].Value
        if (-not $tail.StartsWith($id)) { continue }
        $next = ("{0}{1}{2}{3}" -f [string]$m.Groups[1].Value, $targetState, [string]$m.Groups[3].Value, $tail)
        if ($next -ne $line) {
            $lines[$i] = $next
            $changed = $true
        }
        break
    }
    if (-not $changed) {
        return [ordered]@{ ok = $false; code = "item_not_found_or_unchanged"; message = "Item not found or already in requested state."; item_id = $id; done = $done }
    }
    $content = [string]::Join([Environment]::NewLine, $lines)
    Set-Content -Path $path -Value $content -Encoding UTF8
    return [ordered]@{ ok = $true; item_id = $id; done = $done; path = $path }
}

function Update-MasterBacklogEntry([string]$itemId, $doneRaw, [bool]$hasDone, [string]$owner, [string]$etaUtc) {
    $id = [string]$itemId
    if (-not $id -or -not ($id -match "^P[0-2]-\d{3}$")) {
        return [ordered]@{ ok = $false; code = "invalid_item_id"; message = "item_id must match P0-001 style."; item_id = $itemId }
    }
    $updated = [ordered]@{
        done = $null
        meta = $null
    }
    if ($hasDone) {
        $done = [bool]$doneRaw
        $resDone = Update-MasterBacklogItemStatus -itemId $id -done $done
        if (-not $resDone.ok) { return $resDone }
        $updated.done = $resDone
    }
    $ownerSet = $null -ne $owner
    $etaSet = $null -ne $etaUtc
    if ($ownerSet -or $etaSet) {
        $resMeta = Update-RoadmapItemMeta -itemId $id -owner ([string]$owner) -etaUtc ([string]$etaUtc)
        if (-not $resMeta.ok) { return $resMeta }
        $updated.meta = $resMeta
    }
    if (-not $hasDone -and -not ($ownerSet -or $etaSet)) {
        return [ordered]@{ ok = $false; code = "invalid_payload"; message = "At least one of done/owner/eta_utc is required."; item_id = $id }
    }
    return [ordered]@{
        ok = $true
        item_id = $id
        updated = $updated
    }
}

function Rotate-RoleToken([string]$role) {
    if (-not ($role -in @("viewer","operator","admin"))) { return $null }
    $next = New-SecureToken
    $script:TokenRoles[$role] = $next
    if ($role -eq "admin") {
        $script:AuthToken = $next
    }
    Mark-TokenUpdated $role
    return $next
}

function Get-RoleRank([string]$role) {
    switch ($role) {
        "admin" { return 3 }
        "operator" { return 2 }
        "viewer" { return 1 }
        default { return 0 }
    }
}

function Get-RiskRank([string]$risk) {
    switch ([string]$risk) {
        "HIGH" { return 3 }
        "MED" { return 2 }
        "INFO" { return 1 }
        default { return 0 }
    }
}

function Resolve-RequestRole($req) {
    if ($script:UnsafeNoAuth) { return "admin" }
    $provided = [string]$req.Headers["X-HyperCore-Token"]
    if (-not $provided) { return "" }
    foreach ($rk in @("admin","operator","viewer")) {
        $token = [string]$script:TokenRoles[$rk]
        if ($token -and $provided -eq $token) { return $rk }
    }
    return ""
}

function Ensure-RoleAtLeast($ctx, [string]$requiredRole) {
    $resolved = Resolve-RequestRole $ctx.Request
    if (-not $resolved) {
        Write-Err $ctx 401 "unauthorized" "Missing or invalid X-HyperCore-Token." "unauthorized" $null
        return $false
    }
    if ((Get-RoleRank $resolved) -lt (Get-RoleRank $requiredRole)) {
        Write-Err $ctx 403 "forbidden_role" ("Role '{0}' is not allowed for this operation." -f $resolved) "forbidden" @{ required_role = $requiredRole; resolved_role = $resolved }
        return $false
    }
    return $true
}

function Is-CriticalAction($actionDef) {
    if ($null -eq $actionDef) { return $false }
    return ([string]$actionDef.risk -eq "HIGH")
}

function Ensure-ActionAllowedByPolicy($ctx, [string]$role, $actionDef, [string]$actionId) {
    if (-not $role) {
        Push-PolicyTrace ([ordered]@{
            ts_utc = [DateTime]::UtcNow.ToString("o")
            source = "runtime"
            role = $role
            action = $actionId
            category = if ($actionDef) { [string]$actionDef.category } else { "" }
            risk = if ($actionDef) { [string]$actionDef.risk } else { "" }
            allowed = $false
            reason = "missing_role"
        })
        Write-Err $ctx 401 "unauthorized" "Missing or invalid X-HyperCore-Token." "unauthorized" $null
        return $false
    }
    if ($null -eq $actionDef) {
        Push-PolicyTrace ([ordered]@{
            ts_utc = [DateTime]::UtcNow.ToString("o")
            source = "runtime"
            role = $role
            action = $actionId
            category = ""
            risk = ""
            allowed = $false
            reason = "unknown_action"
        })
        Write-Err $ctx 400 "unknown_action" "Action is not in catalog." "unknown_action" @{ action = $actionId }
        return $false
    }

    $pol = $script:RolePolicies[$role]
    if ($null -eq $pol) { return $true }

    $deniedActions = @($pol.denied_actions)
    if ($deniedActions -contains "*" -or $deniedActions -contains [string]$actionId) {
        Push-PolicyTrace ([ordered]@{
            ts_utc = [DateTime]::UtcNow.ToString("o")
            source = "runtime"
            role = $role
            action = $actionId
            category = [string]$actionDef.category
            risk = [string]$actionDef.risk
            allowed = $false
            reason = "forbidden_action_policy"
        })
        Write-Err $ctx 403 "forbidden_action_policy" "Action is blocked by role policy." "forbidden" @{ action = $actionId; role = $role }
        return $false
    }

    $cat = [string]$actionDef.category
    $deniedCats = @($pol.denied_categories)
    if ($deniedCats -contains $cat) {
        Push-PolicyTrace ([ordered]@{
            ts_utc = [DateTime]::UtcNow.ToString("o")
            source = "runtime"
            role = $role
            action = $actionId
            category = [string]$actionDef.category
            risk = [string]$actionDef.risk
            allowed = $false
            reason = "forbidden_category_policy"
        })
        Write-Err $ctx 403 "forbidden_category_policy" "Action category is blocked by role policy." "forbidden" @{ category = $cat; role = $role }
        return $false
    }

    $maxRisk = [string]$pol.max_risk
    if ($maxRisk) {
        if ((Get-RiskRank ([string]$actionDef.risk)) -gt (Get-RiskRank $maxRisk)) {
            Push-PolicyTrace ([ordered]@{
                ts_utc = [DateTime]::UtcNow.ToString("o")
                source = "runtime"
                role = $role
                action = $actionId
                category = [string]$actionDef.category
                risk = [string]$actionDef.risk
                allowed = $false
                reason = "forbidden_risk_policy"
            })
            Write-Err $ctx 403 "forbidden_risk_policy" "Action risk exceeds role policy." "forbidden" @{ action = $actionId; role = $role; max_risk = $maxRisk; risk = [string]$actionDef.risk }
            return $false
        }
    }
    Push-PolicyTrace ([ordered]@{
        ts_utc = [DateTime]::UtcNow.ToString("o")
        source = "runtime"
        role = $role
        action = $actionId
        category = [string]$actionDef.category
        risk = [string]$actionDef.risk
        allowed = $true
        reason = "allowed"
    })
    return $true
}

function Evaluate-ActionPolicy([string]$role, $actionDef, [string]$actionId) {
    $result = [ordered]@{
        role = [string]$role
        action = [string]$actionId
        category = if ($actionDef) { [string]$actionDef.category } else { "" }
        risk = if ($actionDef) { [string]$actionDef.risk } else { "" }
        allowed = $true
        reason = "allowed"
    }
    if (-not $role) {
        $result.allowed = $false
        $result.reason = "missing_role"
        return $result
    }
    if ($null -eq $actionDef) {
        $result.allowed = $false
        $result.reason = "unknown_action"
        return $result
    }
    $pol = $script:RolePolicies[$role]
    if ($null -eq $pol) { return $result }

    $deniedActions = @($pol.denied_actions)
    if ($deniedActions -contains "*" -or $deniedActions -contains [string]$actionId) {
        $result.allowed = $false
        $result.reason = "forbidden_action_policy"
        return $result
    }

    $cat = [string]$actionDef.category
    $deniedCats = @($pol.denied_categories)
    if ($deniedCats -contains $cat) {
        $result.allowed = $false
        $result.reason = "forbidden_category_policy"
        return $result
    }

    $maxRisk = [string]$pol.max_risk
    if ($maxRisk) {
        if ((Get-RiskRank ([string]$actionDef.risk)) -gt (Get-RiskRank $maxRisk)) {
            $result.allowed = $false
            $result.reason = "forbidden_risk_policy"
            return $result
        }
    }
    return $result
}

function Push-PolicyTrace($item) {
    if ($null -eq $item) { return }
    $script:PolicyTrace.Insert(0, $item)
    while ($script:PolicyTrace.Count -gt 200) { $script:PolicyTrace.RemoveAt($script:PolicyTrace.Count - 1) }
}

function Get-HostById([string]$hostId) {
    foreach ($h in @($script:Hosts)) {
        if ([string]$h.id -eq [string]$hostId) { return $h }
    }
    return $null
}

function Invoke-RemoteJson([string]$Method, [string]$Url, [string]$Token, $Body = $null) {
    $headers = @{}
    if ($Token) { $headers["X-HyperCore-Token"] = $Token }
    $args = @{
        Method = $Method
        Uri = $Url
        Headers = $headers
        TimeoutSec = 8
    }
    if ($null -ne $Body) {
        $args["ContentType"] = "application/json"
        $args["Body"] = ($Body | ConvertTo-Json -Depth 10 -Compress)
    }
    $resp = Invoke-WebRequest @args
    $json = $null
    if ($resp.Content) {
        try { $json = $resp.Content | ConvertFrom-Json } catch {}
    }
    return [ordered]@{
        status = [int]$resp.StatusCode
        json = $json
        raw = [string]$resp.Content
    }
}

function New-Confirmation([string]$actionId, [string]$role) {
    Prune-ExpiredConfirmations
    $cid = [guid]::NewGuid().ToString("N")
    $now = [DateTime]::UtcNow
    $entry = [ordered]@{
        id = $cid
        action = $actionId
        role = $role
        created_utc = $now.ToString("o")
        expires_utc = $now.AddSeconds($script:ConfirmationTtlSec).ToString("o")
        used = $false
    }
    $script:Confirmations[$cid] = $entry
    return $entry
}

function Validate-Confirmation([string]$confirmationId, [string]$actionId, [string]$role) {
    Prune-ExpiredConfirmations
    if (-not $confirmationId) { return $false }
    if (-not $script:Confirmations.ContainsKey($confirmationId)) { return $false }
    $c = $script:Confirmations[$confirmationId]
    if ($c.used) { return $false }
    if ([string]$c.action -ne $actionId) { return $false }
    if ([string]$c.role -ne $role) { return $false }
    $exp = [DateTime]::Parse([string]$c.expires_utc)
    if ([DateTime]::UtcNow -gt $exp) { return $false }
    $c.used = $true
    return $true
}

function Get-ConfirmationState($entry) {
    if ($null -eq $entry) { return "unknown" }
    if ($entry.used) { return "used" }
    $exp = [DateTime]::Parse([string]$entry.expires_utc)
    if ([DateTime]::UtcNow -gt $exp) { return "expired" }
    return "pending"
}

function Prune-ExpiredConfirmations([int]$hardLimit = 400) {
    $keys = @($script:Confirmations.Keys)
    foreach ($k in $keys) {
        $entry = $script:Confirmations[$k]
        if ($null -eq $entry) {
            $script:Confirmations.Remove($k) | Out-Null
            continue
        }
        $state = Get-ConfirmationState $entry
        if ($state -eq "used" -or $state -eq "expired") {
            $exp = [DateTime]::Parse([string]$entry.expires_utc)
            if ([DateTime]::UtcNow -gt $exp.AddMinutes(5)) {
                $script:Confirmations.Remove($k) | Out-Null
            }
        }
    }

    if ($script:Confirmations.Count -le $hardLimit) { return }
    $rows = @()
    foreach ($k in @($script:Confirmations.Keys)) {
        $e = $script:Confirmations[$k]
        $rows += [ordered]@{
            id = [string]$k
            created_utc = [string]$e.created_utc
        }
    }
    $drop = @($rows | Sort-Object -Property @{ Expression = { [string]$_.created_utc }; Ascending = $true } | Select-Object -First ($script:Confirmations.Count - $hardLimit))
    foreach ($d in $drop) {
        $script:Confirmations.Remove([string]$d.id) | Out-Null
    }
}

function Get-ConfirmationRows {
    $rows = @()
    foreach ($k in @($script:Confirmations.Keys)) {
        $e = $script:Confirmations[$k]
        if ($null -eq $e) { continue }
        $rows += [ordered]@{
            id = [string]$k
            action = [string]$e.action
            role = [string]$e.role
            created_utc = [string]$e.created_utc
            expires_utc = [string]$e.expires_utc
            used = [bool]$e.used
            state = (Get-ConfirmationState $e)
        }
    }
    return @($rows | Sort-Object -Property @{ Expression = { [string]$_.created_utc }; Ascending = $false })
}

function Write-SseEvent($resp, [string]$eventName, $dataObj) {
    $json = $dataObj | ConvertTo-Json -Depth 10 -Compress
    $payload = "event: {0}`ndata: {1}`n`n" -f $eventName, $json
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($payload)
    $resp.OutputStream.Write($bytes, 0, $bytes.Length)
    $resp.OutputStream.Flush()
}

function Read-JsonSafe([string]$path) {
    if (-not (Test-Path $path)) { return $null }
    try {
        return Get-Content -Raw -Path $path | ConvertFrom-Json
    } catch {
        return $null
    }
}

function Get-Sha256Hex([string]$path) {
    $stream = $null
    try {
        $stream = [System.IO.File]::OpenRead($path)
        $sha = [System.Security.Cryptography.SHA256]::Create()
        try {
            $bytes = $sha.ComputeHash($stream)
            return ([System.BitConverter]::ToString($bytes).Replace("-", "").ToLowerInvariant())
        } finally {
            $sha.Dispose()
        }
    } finally {
        if ($stream) { $stream.Dispose() }
    }
}

function Get-CrashSummaryPayload {
    $targets = [ordered]@{
        crash_pipeline = "reports/crash_pipeline_smoke/summary.json"
        reboot_recovery = "reports/reboot_recovery_gate_pass/summary.json"
        reboot_recovery_smoke = "reports/reboot_recovery_gate_smoke/summary.json"
        diagnostics_manifest = "reports/tooling/artifact_manifest.json"
    }
    $entries = @()
    foreach ($k in $targets.Keys) {
        $p = [string]$targets[$k]
        $obj = Read-JsonSafe $p
        $entries += [ordered]@{
            id = $k
            path = $p
            exists = [bool](Test-Path $p)
            ok = if ($null -ne $obj -and $obj.PSObject.Properties.Name.Contains("ok")) { [bool]$obj.ok } else { $null }
            summary = $obj
            modified_utc = if (Test-Path $p) { (Get-Item $p).LastWriteTimeUtc.ToString("o") } else { "" }
        }
    }
    return [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        entries = $entries
    }
}

function Get-ComplianceReportPayload {
    $manifestPath = "reports/tooling/artifact_manifest.json"
    $verifyPath = "reports/tooling/artifact_verify.json"
    $manifest = Read-JsonSafe $manifestPath
    $verify = Read-JsonSafe $verifyPath
    $rows = @()
    $missing = 0
    $checksumMismatch = 0
    foreach ($r in @($manifest.rows)) {
        if ($null -eq $r) { continue }
        $p = [string]$r.path
        $exists = $false
        $actual = ""
        $expected = [string]$r.sha256
        if ($p -and (Test-Path $p)) {
            $exists = $true
            try { $actual = Get-Sha256Hex $p } catch {}
        } else {
            $missing += 1
        }
        $match = $false
        if ($exists -and $expected -and $actual) {
            $match = ($expected.ToLowerInvariant() -eq $actual.ToLowerInvariant())
            if (-not $match) { $checksumMismatch += 1 }
        }
        $rows += [ordered]@{
            path = $p
            exists = $exists
            expected_sha256 = $expected
            actual_sha256 = $actual
            checksum_match = $match
            bytes = [int64]$r.bytes
            mtime_utc = [string]$r.mtime_utc
        }
    }

    $securityChecklist = @(
        [ordered]@{ id = "artifact_verify_ok"; pass = [bool]$verify.ok; detail = "reports/tooling/artifact_verify.json" },
        [ordered]@{ id = "manifest_present"; pass = [bool](Test-Path $manifestPath); detail = $manifestPath },
        [ordered]@{ id = "manifest_rows_present"; pass = (@($rows).Count -gt 0); detail = ("rows={0}" -f @($rows).Count) },
        [ordered]@{ id = "manifest_missing_files"; pass = ($missing -eq 0); detail = ("missing={0}" -f $missing) },
        [ordered]@{ id = "manifest_checksum_mismatch"; pass = ($checksumMismatch -eq 0); detail = ("mismatch={0}" -f $checksumMismatch) }
    )

    $securityTemplate = [ordered]@{
        id = "security_regression_v1"
        title = "Security Regression Template"
        checks = @(
            [ordered]@{ step = "Run tooling quality gate"; command = "tooling-quality-gate" },
            [ordered]@{ step = "Verify artifacts"; command = "verify-artifacts" },
            [ordered]@{ step = "Run policy gate"; command = "policy-gate" },
            [ordered]@{ step = "Run qemu smoke"; command = "qemu-smoke" }
        )
    }

    $passCount = @($securityChecklist | Where-Object { $_.pass }).Count
    return [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        ok = ($passCount -eq @($securityChecklist).Count)
        pass_count = $passCount
        total_checks = @($securityChecklist).Count
        artifact_manifest_path = $manifestPath
        artifact_verify_path = $verifyPath
        artifact_rows = $rows
        checks = $securityChecklist
        security_regression_template = $securityTemplate
    }
}

function Get-PluginHealthPayload {
    $pluginDir = "scripts/plugins"
    $apiVersion = [version]"1.0"
    $items = @()
    $okCount = 0
    $failCount = 0

    if (-not (Test-Path $pluginDir)) {
        return [ordered]@{
            ok = $false
            reason = "plugin_dir_missing"
            plugin_dir = $pluginDir
            items = @()
            summary = [ordered]@{ total = 0; ok = 0; fail = 0 }
        }
    }

    $manifests = @(Get-ChildItem -Path $pluginDir -Filter "*.plugin.json" -File -ErrorAction SilentlyContinue | Sort-Object Name)
    foreach ($m in $manifests) {
        $obj = Read-JsonSafe $m.FullName
        $statusOk = $true
        $issues = @()
        $pluginScript = [System.IO.Path]::ChangeExtension($m.FullName, ".ps1")
        if (-not (Test-Path $pluginScript)) {
            $statusOk = $false
            $issues += "script_missing"
        }
        foreach ($req in @("name","version","min_api_version","checksum_sha256")) {
            if ($null -eq $obj -or -not $obj.PSObject.Properties.Name.Contains($req) -or -not [string]$obj.$req) {
                $statusOk = $false
                $issues += ("manifest_missing_{0}" -f $req)
            }
        }
        if ((Test-Path $pluginScript) -and $obj -and $obj.PSObject.Properties.Name.Contains("checksum_sha256")) {
            $actual = Get-Sha256Hex -path $pluginScript
            $expected = ([string]$obj.checksum_sha256).ToLowerInvariant()
            if ($actual -ne $expected) {
                $statusOk = $false
                $issues += "checksum_mismatch"
            }
        }
        if ($obj -and $obj.PSObject.Properties.Name.Contains("min_api_version")) {
            try {
                $minApi = [version][string]$obj.min_api_version
                if ($minApi -gt $apiVersion) {
                    $statusOk = $false
                    $issues += "api_incompatible"
                }
            } catch {
                $statusOk = $false
                $issues += "api_version_parse_failed"
            }
        }

        if ($statusOk) { $okCount += 1 } else { $failCount += 1 }
        $items += [ordered]@{
            manifest = $m.FullName
            script = $pluginScript
            name = if ($obj) { [string]$obj.name } else { $m.BaseName }
            version = if ($obj) { [string]$obj.version } else { "" }
            min_api_version = if ($obj) { [string]$obj.min_api_version } else { "" }
            ok = $statusOk
            issues = $issues
        }
    }

    return [ordered]@{
        ok = ($failCount -eq 0)
        plugin_dir = $pluginDir
        api_version = [string]$apiVersion
        items = $items
        summary = [ordered]@{
            total = @($items).Count
            ok = $okCount
            fail = $failCount
        }
    }
}

function Push-Recent($item) {
    $script:Recent.Insert(0, $item)
    while ($script:Recent.Count -gt 80) { $script:Recent.RemoveAt($script:Recent.Count - 1) }
}

function Initialize-AuditLog {
    New-Item -ItemType Directory -Path $script:AuditDir -Force | Out-Null
    $cutoff = (Get-Date).ToUniversalTime().AddDays(-1 * $script:LogRetentionDays)
    $old = Get-ChildItem -Path $script:AuditDir -Filter "*.json" -ErrorAction SilentlyContinue | Where-Object { $_.LastWriteTimeUtc -lt $cutoff }
    foreach ($f in $old) { Remove-Item -Force -Path $f.FullName -ErrorAction SilentlyContinue }
}

function Write-AuditLog($entry) {
    if ($null -eq $entry -or $entry.audit_logged) { return }
    New-Item -ItemType Directory -Path $script:AuditDir -Force | Out-Null
    $safeAction = ([string]$entry.action -replace "[^a-zA-Z0-9_-]", "_")
    $stamp = [DateTime]::UtcNow.ToString("yyyyMMdd_HHmmss")
    $path = Join-Path $script:AuditDir ("{0}_{1}_{2}.json" -f $stamp, $safeAction, $entry.id)
    $obj = [ordered]@{
        id = $entry.id
        action = $entry.action
        title = $entry.title
        command = $entry.command
        profile = $Profile
        status = $entry.status
        ok = [bool]$entry.ok
        exit_code = $entry.exit_code
        queued_utc = $entry.queued_utc
        started_utc = $entry.started_utc
        finished_utc = $entry.finished_utc
        queue_wait_ms = [int]$entry.queue_wait_ms
        duration_ms = [int]$entry.duration_ms
        line_count = [int]$entry.line_count
        priority = $entry.priority
        source = $entry.source
        token_protected = $true
        max_concurrency = $script:MaxConcurrency
    }
    ($obj | ConvertTo-Json -Depth 8) | Set-Content -Path $path -Encoding UTF8
    $entry.audit_logged = $true
}

function Start-JobProcess($entry) {
    if ($null -eq $entry -or $entry.status -ne "queued") { return }
    $job = Start-Job -ScriptBlock {
        param($HypercorePath, $RunProfile, $UseNoLock, $Cmd, $CmdArgs)
        $args = @("-ExecutionPolicy", "Bypass", "-File", $HypercorePath, "-Command", $Cmd, "-Profile", $RunProfile)
        if ($UseNoLock) { $args += "-NoLock" }
        if ($CmdArgs) { $args += $CmdArgs }
        & powershell @args 2>&1 | ForEach-Object { [string]$_ }
        $exitCode = $LASTEXITCODE
        Write-Output ("__HC_EXIT_CODE:{0}" -f $exitCode)
    } -ArgumentList $hypercorePath, $Profile, [bool]$NoLock, [string]$entry.command, @($entry.command_args)

    $entry.job = $job
    $entry.started_utc = [DateTime]::UtcNow.ToString("o")
    $qTs = [DateTime]::Parse($entry.queued_utc)
    $sTs = [DateTime]::Parse($entry.started_utc)
    $entry.queue_wait_ms = [int](($sTs - $qTs).TotalMilliseconds)
    $entry.status = "running"
}

function Finalize-JobEntry($entry, [string]$forcedStatus = "") {
    if ($entry.status -in @("completed", "failed", "cancelled")) {
        Write-AuditLog $entry
        return
    }

    $now = [DateTime]::UtcNow
    $entry.finished_utc = $now.ToString("o")
    $startTs = if ($entry.started_utc) { [DateTime]::Parse($entry.started_utc) } else { $now }
    $entry.duration_ms = [int](($now - $startTs).TotalMilliseconds)

    if ($forcedStatus -eq "cancelled") {
        $entry.status = "cancelled"
        if ($entry.exit_code -eq $null) { $entry.exit_code = 130 }
        $entry.ok = $false
    } else {
        if ($entry.exit_code -eq $null) { $entry.exit_code = if ($forcedStatus -eq "completed") { 0 } else { 1 } }
        $entry.ok = ([int]$entry.exit_code -eq 0)
        $entry.status = if ($entry.ok) { "completed" } else { "failed" }
    }

    Push-Recent ([ordered]@{
        id = $entry.id
        action = $entry.action
        title = $entry.title
        timestamp_utc = $entry.finished_utc
        status = $entry.status
        ok = [bool]$entry.ok
        exit_code = [int]$entry.exit_code
        duration_ms = [int]$entry.duration_ms
        queue_wait_ms = [int]$entry.queue_wait_ms
        priority = $entry.priority
    })

    Write-AuditLog $entry
}

function Update-JobEntry($entry) {
    if ($null -eq $entry) { return }
    if ($entry.status -eq "queued" -or $entry.status -eq "cancelled") { return }
    if ($null -eq $entry.job) { return }

    $new = @()
    try { $new = @(Receive-Job -Job $entry.job -Keep -ErrorAction SilentlyContinue) } catch {}
    foreach ($ln in $new) {
        $s = [string]$ln
        if ($s.StartsWith("__HC_EXIT_CODE:")) {
            $codeStr = $s.Substring(15)
            $code = 1
            [void][int]::TryParse($codeStr, [ref]$code)
            $entry.exit_code = $code
            continue
        }
        $entry.lines.Add($s)
    }

    $entry.line_count = $entry.lines.Count
    $entry.last_poll_utc = [DateTime]::UtcNow.ToString("o")

    $state = [string]$entry.job.State
    if ($state -eq "Completed") {
        Finalize-JobEntry -entry $entry -forcedStatus "completed"
    } elseif ($state -eq "Failed" -or $state -eq "Stopped") {
        Finalize-JobEntry -entry $entry -forcedStatus "failed"
    }
}

function Test-JobExists([string]$jobId) {
    if (-not $jobId) { return $false }
    if ($script:Jobs -is [System.Collections.IDictionary]) {
        return [bool]$script:Jobs.Contains($jobId)
    }
    foreach ($e in @($script:Jobs)) {
        if ($e -and ([string]$e.id -eq $jobId)) { return $true }
    }
    return $false
}

function Get-JobEntryById([string]$jobId) {
    if (-not (Test-JobExists -jobId $jobId)) { return $null }
    if ($script:Jobs -is [System.Collections.IDictionary]) {
        return $script:Jobs[$jobId]
    }
    foreach ($e in @($script:Jobs)) {
        if ($e -and ([string]$e.id -eq $jobId)) { return $e }
    }
    return $null
}

function Update-AllJobs {
    foreach ($id in @($script:Jobs.Keys)) {
        if (-not $id) { continue }
        if (-not (Test-JobExists -jobId $id)) { continue }
        Update-JobEntry (Get-JobEntryById -jobId $id)
    }
}

function Get-RunningCount {
    Update-AllJobs
    $count = 0
    foreach ($id in @($script:Jobs.Keys)) {
        if (-not $id) { continue }
        if (-not (Test-JobExists -jobId $id)) { continue }
        if ((Get-JobEntryById -jobId $id).status -eq "running") { $count += 1 }
    }
    return $count
}

function Get-QueueCount {
    $count = 0
    foreach ($id in @($script:Jobs.Keys)) {
        if (-not $id) { continue }
        if (-not (Test-JobExists -jobId $id)) { continue }
        if ((Get-JobEntryById -jobId $id).status -eq "queued") { $count += 1 }
    }
    return $count
}

function Get-AgentBusy {
    return ((Get-RunningCount) -gt 0 -or (Get-QueueCount) -gt 0)
}

function Insert-QueueByPriority([string]$jobId) {
    $entry = $script:Jobs[$jobId]
    if ($null -eq $entry) { return }

    $inserted = $false
    for ($i = 0; $i -lt $script:Queue.Count; $i++) {
        $otherId = $script:Queue[$i]
        $other = $script:Jobs[$otherId]
        if ($null -eq $other -or $other.status -ne "queued") { continue }
        $p1 = [int]$script:PriorityOrder[[string]$entry.priority]
        $p2 = [int]$script:PriorityOrder[[string]$other.priority]
        if ($p1 -lt $p2) {
            $script:Queue.Insert($i, $jobId)
            $inserted = $true
            break
        }
    }
    if (-not $inserted) { $script:Queue.Add($jobId) }
}

function Enqueue-Action($actionDef, [string]$actionId, [string]$priority = "normal", [string]$source = "api") {
    if (-not $script:PriorityOrder.ContainsKey($priority)) { $priority = "normal" }

    $jobId = [guid]::NewGuid().ToString("N")
    $entry = [ordered]@{
        id = $jobId
        action = $actionId
        title = [string]$actionDef.title
        command = [string]$actionDef.cmd
        command_args = @($actionDef.args)
        risk = [string]$actionDef.risk
        category = [string]$actionDef.category
        queued_utc = [DateTime]::UtcNow.ToString("o")
        started_utc = ""
        finished_utc = ""
        status = "queued"
        priority = $priority
        source = $source
        ok = $false
        exit_code = $null
        queue_wait_ms = 0
        duration_ms = 0
        last_poll_utc = ""
        lines = New-Object System.Collections.Generic.List[string]
        line_count = 0
        audit_logged = $false
        job = $null
    }
    $script:Jobs[$jobId] = $entry
    Insert-QueueByPriority -jobId $jobId
    return $entry
}

function Dispatch-Queue {
    Update-AllJobs

    $running = Get-RunningCount
    while ($running -lt $script:MaxConcurrency -and $script:Queue.Count -gt 0) {
        $nextId = $script:Queue[0]
        $script:Queue.RemoveAt(0)
        if (-not (Test-JobExists -jobId $nextId)) { continue }
        $entry = Get-JobEntryById -jobId $nextId
        if ($entry.status -ne "queued") { continue }
        Start-JobProcess -entry $entry
        $running += 1
    }
}

function Tick-Scheduler {
    if (-not $script:SchedulerEnabled) { return }
    if ($script:Schedules.Count -eq 0) { return }

    $now = [DateTime]::UtcNow
    foreach ($sid in @($script:Schedules.Keys)) {
        $s = $script:Schedules[$sid]
        if (-not $s.enabled) { continue }
        $next = [DateTime]::Parse([string]$s.next_run_utc)
        if ($now -lt $next) { continue }
        if (Get-QueueCount -ge $script:MaxQueue) {
            $s.next_run_utc = $now.AddSeconds([int]$s.interval_sec).ToString("o")
            continue
        }
        if ($actionMap.ContainsKey([string]$s.action)) {
            [void](Enqueue-Action -actionDef $actionMap[[string]$s.action] -actionId [string]$s.action -priority [string]$s.priority -source [string]$s.source)
        }
        $s.last_run_utc = $now.ToString("o")
        $s.next_run_utc = $now.AddSeconds([int]$s.interval_sec).ToString("o")
    }
}

function Get-JobSummary($entry) {
    Update-JobEntry $entry
    return [ordered]@{
        id = $entry.id
        action = $entry.action
        title = $entry.title
        command = $entry.command
        risk = $entry.risk
        category = $entry.category
        status = $entry.status
        priority = $entry.priority
        source = $entry.source
        ok = [bool]$entry.ok
        exit_code = $entry.exit_code
        queued_utc = $entry.queued_utc
        started_utc = $entry.started_utc
        finished_utc = $entry.finished_utc
        queue_wait_ms = [int]$entry.queue_wait_ms
        duration_ms = [int]$entry.duration_ms
        line_count = [int]$entry.line_count
        last_poll_utc = $entry.last_poll_utc
    }
}

function Get-RecentLines($entry, [int]$Tail = 300) {
    Update-JobEntry $entry
    $all = @($entry.lines)
    if ($all.Count -le $Tail) { return $all }
    return @($all | Select-Object -Last $Tail)
}

function Cancel-JobEntry($entry) {
    if ($null -eq $entry) { return }
    if ($entry.status -in @("completed", "failed", "cancelled")) { return }

    if ($entry.status -eq "queued") {
        $entry.exit_code = 130
        Finalize-JobEntry -entry $entry -forcedStatus "cancelled"
        return
    }

    try {
        if ($entry.job) {
            Stop-Job -Job $entry.job -ErrorAction SilentlyContinue
            Remove-Job -Job $entry.job -Force -ErrorAction SilentlyContinue
        }
    } catch {}
    $entry.exit_code = 130
    Finalize-JobEntry -entry $entry -forcedStatus "cancelled"
}

$listener = New-Object System.Net.HttpListener
$prefix = "http://127.0.0.1:$Port/"
$listener.Prefixes.Add($prefix)
$listener.Start()
Initialize-AuditLog
Initialize-Scheduler
Initialize-Hosts
Initialize-RolePolicies
Write-Host "[dashboard-agent] listening on $prefix" -ForegroundColor Green
Write-Host "[dashboard-agent] profile=$Profile max_concurrency=$script:MaxConcurrency max_queue=$script:MaxQueue" -ForegroundColor DarkGray
Write-Host "[dashboard-agent] scheduler_enabled=$script:SchedulerEnabled tasks=$($script:Schedules.Count)" -ForegroundColor DarkGray
Write-Host "[dashboard-agent] allowed_origins=$($script:AllowedOrigins -join ', ')" -ForegroundColor DarkGray
Write-Host "[dashboard-agent] token=$script:AuthToken" -ForegroundColor Yellow
Write-Host "[dashboard-agent] auth_mode=$(if($script:UnsafeNoAuth){'unsafe'}else{'strict'})" -ForegroundColor $(if($script:UnsafeNoAuth){'Red'}else{'DarkGray'})
Write-Host "[dashboard-agent] press Ctrl+C to stop" -ForegroundColor DarkGray

try {
    while ($listener.IsListening) {
        $ctx = $listener.GetContext()
        $req = $ctx.Request
        $resp = $ctx.Response
        $path = ($req.Url.AbsolutePath.Trim("/")).ToLowerInvariant()
        try {

            Tick-Scheduler
            Dispatch-Queue
            Prune-ExpiredConfirmations

            if (-not (Is-OriginAllowed ([string]$req.Headers["Origin"]))) {
                Write-Err $ctx 403 "origin_not_allowed" "Origin is not allowed by agent CORS policy." "origin_forbidden" @{ origin = [string]$req.Headers["Origin"] }
                continue
            }

            if ($req.HttpMethod -eq "OPTIONS") {
                Add-CorsHeaders $req $resp
                $resp.StatusCode = 204
                $resp.OutputStream.Close()
                continue
            }

            if (Is-MutatingRequest -req $req -path $path) {
                if (-not $script:UnsafeNoAuth -and -not (Test-AuthToken $req)) {
                    Write-Err $ctx 401 "unauthorized" "Missing or invalid X-HyperCore-Token." "unauthorized" $null
                    continue
                }
            }

            if ($path -eq "health" -and $req.HttpMethod -eq "GET") {
                $resolvedRole = Resolve-RequestRole $req
                Write-Ok $ctx 200 "agent healthy" ([ordered]@{
                    busy = (Get-AgentBusy)
                    profile = $Profile
                    started_utc = $script:StartedUtc
                    now_utc = [DateTime]::UtcNow.ToString("o")
                    max_concurrency = $script:MaxConcurrency
                    max_queue = $script:MaxQueue
                    running_count = (Get-RunningCount)
                    queue_count = (Get-QueueCount)
                    scheduler_enabled = $script:SchedulerEnabled
                    auth_mode = if ($script:UnsafeNoAuth) { "unsafe" } else { "strict" }
                    unsafe_no_auth = [bool]$script:UnsafeNoAuth
                    role = if ($resolvedRole) { $resolvedRole } else { "anonymous" }
                })
                continue
            }

            if ($path -eq "auth/status" -and $req.HttpMethod -eq "GET") {
                if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "viewer")) { continue }
                Write-Ok $ctx 200 "auth_status" (Get-AuthStatusPayload)
                continue
            }

            if ($path -eq "auth/rotate" -and $req.HttpMethod -eq "POST") {
                if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
                $body = Read-RequestJson $req
                $unknown = @()
                if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("role", "rotate_all") -unknownKeys ([ref]$unknown))) {
                    Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                    continue
                }
                $rotateAll = [bool]$body.rotate_all
                $role = [string]$body.role
                $targets = @()
                if ($rotateAll) {
                    $targets = @("viewer", "operator", "admin")
                } else {
                    if (-not $role -or -not ($role -in @("viewer","operator","admin"))) {
                        Write-Err $ctx 400 "invalid_role" "role must be one of viewer|operator|admin when rotate_all is false." "invalid_payload" @{ role = $role }
                        continue
                    }
                    $targets = @($role)
                }
                $rotated = @()
                foreach ($rk in $targets) {
                    $token = Rotate-RoleToken -role $rk
                    if ($token) {
                        $rotated += [ordered]@{ role = $rk; token = $token; updated_utc = [string]$script:TokenMeta[$rk].updated_utc }
                    }
                }
                Write-Ok $ctx 200 "auth_rotated" ([ordered]@{
                    rotated = $rotated
                    auth = (Get-AuthStatusPayload)
                })
                continue
            }

        if ($path -eq "catalog" -and $req.HttpMethod -eq "GET") {
            $catalog = @()
            foreach ($a in $actions) {
                $catalog += [ordered]@{
                    id = [string]$a.id
                    title = [string]$a.title
                    desc = [string]$a.desc
                    risk = [string]$a.risk
                    category = [string]$a.category
                    impact = [string]$a.impact
                }
            }
            Write-Ok $ctx 200 "catalog" ([ordered]@{ actions = $catalog })
            continue
        }

        if ($path -eq "roadmap/status" -and $req.HttpMethod -eq "GET") {
            $payload = Get-RoadmapStatusPayload
            Write-Ok $ctx 200 "roadmap_status" $payload
            continue
        }

        if ($path -eq "roadmap/master" -and $req.HttpMethod -eq "GET") {
            $payload = Get-MasterBacklogStatusPayload
            Write-Ok $ctx 200 "roadmap_master" $payload
            continue
        }

        if ($path -eq "roadmap/master/update" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
            $body = Read-RequestJson $req
            $unknown = @()
            if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("item_id","done","owner","eta_utc") -unknownKeys ([ref]$unknown))) {
                Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                continue
            }
            $itemId = [string]$body.item_id
            $hasDone = $body.PSObject.Properties.Name.Contains("done")
            $doneRaw = if ($hasDone) { $body.done } else { $null }
            $owner = if ($body.PSObject.Properties.Name.Contains("owner")) { [string]$body.owner } else { $null }
            $etaUtc = if ($body.PSObject.Properties.Name.Contains("eta_utc")) { [string]$body.eta_utc } else { $null }
            $result = Update-MasterBacklogEntry -itemId $itemId -doneRaw $doneRaw -hasDone $hasDone -owner $owner -etaUtc $etaUtc
            if (-not $result.ok) {
                Write-Err $ctx 400 ([string]$result.code) ([string]$result.message) "roadmap_update_failed" $result
                continue
            }
            $payload = Get-MasterBacklogStatusPayload
            Write-Ok $ctx 200 "roadmap_master_updated" ([ordered]@{
                update = $result
                master = $payload
            })
            continue
        }

        if ($path -eq "roadmap/batch/record" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "operator")) { continue }
            $body = Read-RequestJson $req
            $unknown = @()
            if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("phase","actions","ok","fail","started_utc","duration_ms") -unknownKeys ([ref]$unknown))) {
                Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                continue
            }
            $phase = [string]$body.phase
            $actions = if ($body.PSObject.Properties.Name.Contains("actions")) { @($body.actions) } else { @() }
            $okCount = if ($body.PSObject.Properties.Name.Contains("ok")) { [int]$body.ok } else { 0 }
            $failCount = if ($body.PSObject.Properties.Name.Contains("fail")) { [int]$body.fail } else { 0 }
            $startedUtc = if ($body.PSObject.Properties.Name.Contains("started_utc")) { [string]$body.started_utc } else { "" }
            $durationMs = if ($body.PSObject.Properties.Name.Contains("duration_ms")) { [int]$body.duration_ms } else { 0 }
            $recordResult = Record-RoadmapBatchHistory -phase $phase -actions $actions -ok $okCount -fail $failCount -startedUtc $startedUtc -durationMs $durationMs
            if (-not $recordResult.ok) {
                Write-Err $ctx 400 ([string]$recordResult.code) ([string]$recordResult.message) "roadmap_batch_record_failed" $recordResult
                continue
            }
            Write-Ok $ctx 200 "roadmap_batch_recorded" ([ordered]@{
                batch = $recordResult
            })
            continue
        }

        if ($path -eq "config" -and $req.HttpMethod -eq "GET") {
            try {
                $payload = Build-ConfigPayload
                Write-Ok $ctx 200 "config" $payload
            } catch {
                Write-Err $ctx 500 "config_read_failed" "Unable to read config file." "config_error" @{ message = [string]$_.Exception.Message; config_path = $ConfigPath }
            }
            continue
        }

        if ($path -eq "config/compose" -and $req.HttpMethod -eq "GET") {
            try {
                $q = Get-QueryMap $req
                $goal = if ($q.ContainsKey("goal")) { [string]$q["goal"] } else { "linux_full" }
                $minimal = $false
                if ($q.ContainsKey("minimal")) {
                    $minimal = ([string]$q["minimal"]).ToLowerInvariant() -in @("1","true","yes","on")
                }
                $rec = Select-BuildFeatureProfile -goal $goal -minimal $minimal
                Write-Ok $ctx 200 "config compose recommendation" ([ordered]@{ recommendation = $rec })
            } catch {
                Write-Err $ctx 400 "config_compose_failed" "Unable to compose build profile recommendation." "config_error" @{ message = [string]$_.Exception.Message }
            }
            continue
        }

        if ($path -eq "config/drift" -and $req.HttpMethod -eq "GET") {
            try {
                $cfgObj = Load-AgentConfig
                $drift = Build-ConfigDriftReport -cfgObj $cfgObj
                Write-Ok $ctx 200 "config drift report" ([ordered]@{ drift = $drift })
            } catch {
                Write-Err $ctx 500 "config_drift_failed" "Unable to compute config drift report." "config_error" @{ message = [string]$_.Exception.Message }
            }
            continue
        }

        if ($path -eq "config/drift/apply" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
            $body = Read-RequestJson $req
            $unknown = @()
            if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("goal","mode","minimal","no_default_features") -unknownKeys ([ref]$unknown))) {
                Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                continue
            }
            $goal = if ($body.PSObject.Properties.Name.Contains("goal")) { [string]$body.goal } else { "linux_full" }
            $mode = if ($body.PSObject.Properties.Name.Contains("mode")) { [string]$body.mode } else { "full" }
            if (-not ($mode -in @("full", "missing_only"))) {
                Write-Err $ctx 400 "invalid_mode" "mode must be full|missing_only." "invalid_payload" @{ mode = $mode }
                continue
            }
            $minimal = $false
            if ($body.PSObject.Properties.Name.Contains("minimal")) { $minimal = [bool]$body.minimal }
            try {
                $cfgObj = Load-AgentConfig
                $currentRaw = ""
                if ($cfgObj.PSObject.Properties.Name.Contains("build") -and $cfgObj.build.PSObject.Properties.Name.Contains("cargo_features")) {
                    $currentRaw = [string]$cfgObj.build.cargo_features
                }
                $current = @(Parse-CargoFeatureCsv -raw $currentRaw)
                $currentSet = @{}
                foreach ($cf in $current) { $currentSet[[string]$cf] = $true }

                $rec = Select-BuildFeatureProfile -goal $goal -minimal $minimal
                $recommended = @([string[]]$rec.selected_features)
                $recommendedSet = @{}
                foreach ($rf in $recommended) { $recommendedSet[[string]$rf] = $true }

                $nextSet = @{}
                if ($mode -eq "full") {
                    foreach ($rf in $recommendedSet.Keys) { $nextSet[[string]$rf] = $true }
                } else {
                    foreach ($cf in $currentSet.Keys) { $nextSet[[string]$cf] = $true }
                    foreach ($rf in $recommendedSet.Keys) { $nextSet[[string]$rf] = $true }
                }
                $nextList = @($nextSet.Keys | Sort-Object)
                $noDefaults = [bool]$rec.no_default_features
                if ($body.PSObject.Properties.Name.Contains("no_default_features")) {
                    $noDefaults = [bool]$body.no_default_features
                }

                $updates = @(
                    [ordered]@{ path = "build.cargo_features"; value = [string]([string[]]$nextList -join ",") },
                    [ordered]@{ path = "build.cargo_no_default_features"; value = [bool]$noDefaults }
                )
                $applied = Apply-ConfigUpdates -updates $updates
                $payload = Build-ConfigPayload
                $drift = Build-ConfigDriftReport -cfgObj (Load-AgentConfig)
                Write-Ok $ctx 200 "config drift apply" ([ordered]@{
                    goal = $goal
                    mode = $mode
                    minimal = [bool]$minimal
                    recommendation = $rec
                    applied = $applied
                    config = $payload
                    drift = $drift
                    restart_hint = "Restart dashboard agent after changing agent.port/auth_mode."
                })
            } catch {
                Write-Err $ctx 400 "config_drift_apply_failed" "Unable to apply drift fix." "config_error" @{ message = [string]$_.Exception.Message; goal = $goal; mode = $mode; minimal = $minimal }
            }
            continue
        }

        if ($path -eq "config/export" -and $req.HttpMethod -eq "GET") {
            try {
                $q = Get-QueryMap $req
                $profileName = if ($q.ContainsKey("name")) { [string]$q["name"] } else { "default" }
                $profile = Export-ConfigProfile -profileName $profileName
                Write-Ok $ctx 200 "config profile export" ([ordered]@{ profile = $profile })
            } catch {
                Write-Err $ctx 500 "config_export_failed" "Unable to export config profile." "config_error" @{ message = [string]$_.Exception.Message }
            }
            continue
        }

        if ($path -eq "config/overrides/template" -and $req.HttpMethod -eq "GET") {
            try {
                $q = Get-QueryMap $req
                $mode = if ($q.ContainsKey("mode")) { [string]$q["mode"] } else { "minimal" }
                $template = Export-ConfigFieldOverrideTemplate -mode $mode
                Write-Ok $ctx 200 "config override template export" ([ordered]@{ template = $template })
            } catch {
                Write-Err $ctx 400 "config_override_template_failed" "Unable to export config override template." "config_error" @{ message = [string]$_.Exception.Message }
            }
            continue
        }

        if ($path -eq "config/update" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
            $body = Read-RequestJson $req
            $unknown = @()
            if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("updates") -unknownKeys ([ref]$unknown))) {
                Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                continue
            }
            $updates = @($body.updates)
            if ($updates.Count -eq 0) {
                Write-Err $ctx 400 "invalid_payload" "updates array is required and cannot be empty." "invalid_payload" @{ updates = $updates }
                continue
            }
            try {
                $applied = Apply-ConfigUpdates -updates $updates
                $payload = Build-ConfigPayload
                Write-Ok $ctx 200 "config updated" ([ordered]@{
                    applied = $applied
                    config = $payload
                    restart_hint = "Restart dashboard agent after changing agent.port/auth_mode."
                })
            } catch {
                Write-Err $ctx 400 "config_update_failed" "Config update failed." "config_error" @{ message = [string]$_.Exception.Message }
            }
            continue
        }

        if ($path -eq "config/auto" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
            $body = Read-RequestJson $req
            $unknown = @()
            if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("mode") -unknownKeys ([ref]$unknown))) {
                Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                continue
            }
            $mode = [string]$body.mode
            if (-not $mode) { $mode = "balanced" }
            try {
                $presetUpdates = Build-AutoPresetUpdates -mode $mode
                $applied = Apply-ConfigUpdates -updates $presetUpdates
                $payload = Build-ConfigPayload
                Write-Ok $ctx 200 "config auto preset applied" ([ordered]@{
                    mode = $mode
                    applied = $applied
                    config = $payload
                    restart_hint = "Restart dashboard agent after changing agent.port/auth_mode."
                })
            } catch {
                Write-Err $ctx 400 "config_auto_failed" "Config auto preset failed." "config_error" @{ message = [string]$_.Exception.Message; mode = $mode }
            }
            continue
        }

        if ($path -eq "config/import" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
            $body = Read-RequestJson $req
            $unknown = @()
            if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("profile","values") -unknownKeys ([ref]$unknown))) {
                Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                continue
            }
            try {
                $profileObj = if ($body.PSObject.Properties.Name.Contains("profile")) { $body.profile } else { [ordered]@{ values = $body.values } }
                $applied = Import-ConfigProfile -profileObj $profileObj
                $payload = Build-ConfigPayload
                Write-Ok $ctx 200 "config imported" ([ordered]@{
                    applied = $applied
                    config = $payload
                    restart_hint = "Restart dashboard agent after changing agent.port/auth_mode."
                })
            } catch {
                Write-Err $ctx 400 "config_import_failed" "Config profile import failed." "config_error" @{ message = [string]$_.Exception.Message }
            }
            continue
        }

        if ($path -eq "config/compose/apply" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
            $body = Read-RequestJson $req
            $unknown = @()
            if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("goal","minimal","no_default_features") -unknownKeys ([ref]$unknown))) {
                Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                continue
            }
            $goal = if ($body.PSObject.Properties.Name.Contains("goal")) { [string]$body.goal } else { "linux_full" }
            $minimal = $false
            if ($body.PSObject.Properties.Name.Contains("minimal")) { $minimal = [bool]$body.minimal }
            try {
                $rec = Select-BuildFeatureProfile -goal $goal -minimal $minimal
                $noDefaults = [bool]$rec.no_default_features
                if ($body.PSObject.Properties.Name.Contains("no_default_features")) {
                    $noDefaults = [bool]$body.no_default_features
                }
                $updates = @(
                    [ordered]@{ path = "build.cargo_features"; value = [string]([string[]]$rec.selected_features -join ",") },
                    [ordered]@{ path = "build.cargo_no_default_features"; value = [bool]$noDefaults }
                )
                $applied = Apply-ConfigUpdates -updates $updates
                $payload = Build-ConfigPayload
                Write-Ok $ctx 200 "config compose applied" ([ordered]@{
                    goal = $goal
                    minimal = [bool]$minimal
                    recommendation = $rec
                    applied = $applied
                    config = $payload
                    restart_hint = "Restart dashboard agent after changing agent.port/auth_mode."
                })
            } catch {
                Write-Err $ctx 400 "config_compose_apply_failed" "Unable to apply composed build profile." "config_error" @{ message = [string]$_.Exception.Message; goal = $goal; minimal = $minimal }
            }
            continue
        }

        if ($path -eq "hosts" -and $req.HttpMethod -eq "GET") {
            Write-Ok $ctx 200 "hosts" ([ordered]@{
                hosts = @($script:Hosts | Where-Object { $_.enabled -ne $false })
                current_host_id = "local"
            })
            continue
        }

        if ($path -eq "hosts/register" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
            $body = Read-RequestJson $req
            $unknown = @()
            if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("id","name","url","enabled","role_hint","token") -unknownKeys ([ref]$unknown))) {
                Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                continue
            }
            if (-not (Upsert-HostEntry $body)) {
                Write-Err $ctx 400 "invalid_host" "Host registration failed." "invalid_payload" @{ id = [string]$body.id; url = [string]$body.url }
                continue
            }
            Write-Ok $ctx 200 "host_registered" ([ordered]@{
                hosts = @($script:Hosts | Where-Object { $_.enabled -ne $false })
            })
            continue
        }

        if ($path -eq "hosts/update" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
            $body = Read-RequestJson $req
            $unknown = @()
            if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("id","name","url","enabled","role_hint","token") -unknownKeys ([ref]$unknown))) {
                Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                continue
            }
            if (-not (Upsert-HostEntry $body)) {
                Write-Err $ctx 400 "invalid_host" "Host update failed." "invalid_payload" @{ id = [string]$body.id; url = [string]$body.url }
                continue
            }
            Write-Ok $ctx 200 "host_updated" ([ordered]@{
                hosts = @($script:Hosts | Where-Object { $_.enabled -ne $false })
            })
            continue
        }

        if ($path -eq "hosts/remove" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
            $body = Read-RequestJson $req
            $unknown = @()
            if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("id") -unknownKeys ([ref]$unknown))) {
                Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                continue
            }
            $hostId = [string]$body.id
            if (-not (Remove-HostEntry -hostId $hostId)) {
                Write-Err $ctx 404 "host_not_found" "Host remove failed (not found or protected)." "host_not_found" @{ id = $hostId }
                continue
            }
            Write-Ok $ctx 200 "host_removed" ([ordered]@{
                hosts = @($script:Hosts | Where-Object { $_.enabled -ne $false })
            })
            continue
        }

        if ($path -eq "status/hosts" -and $req.HttpMethod -eq "GET") {
            $hostRows = @()
            foreach ($h in @($script:Hosts | Where-Object { $_.enabled -ne $false })) {
                $row = [ordered]@{
                    id = [string]$h.id
                    name = [string]$h.name
                    url = [string]$h.url
                    reachable = $false
                    busy = $false
                    running_count = 0
                    queue_count = 0
                    role = "unknown"
                    error = ""
                }
                try {
                    if ([string]$h.id -eq "local") {
                        $row.reachable = $true
                        $row.busy = (Get-AgentBusy)
                        $row.running_count = (Get-RunningCount)
                        $row.queue_count = (Get-QueueCount)
                        $row.role = "local"
                    } else {
                        $hr = Invoke-WebRequest -Method Get -Uri ("{0}/health" -f [string]$h.url) -TimeoutSec 3
                        $hp = $hr.Content | ConvertFrom-Json
                        $row.reachable = [bool]$hp.ok
                        $row.busy = [bool]$hp.busy
                        $row.role = if ($hp.role) { [string]$hp.role } else { "anonymous" }
                        $sr = Invoke-WebRequest -Method Get -Uri ("{0}/status" -f [string]$h.url) -TimeoutSec 3
                        $sp = $sr.Content | ConvertFrom-Json
                        $row.running_count = [int]$sp.running_count
                        $row.queue_count = [int]$sp.queue_count
                    }
                } catch {
                    $row.error = [string]$_.Exception.Message
                }
                $hostRows += $row
            }
            Write-Ok $ctx 200 "hosts_status" ([ordered]@{ hosts = $hostRows })
            continue
        }

        if ($path -eq "policy" -and $req.HttpMethod -eq "GET") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "viewer")) { continue }
            Write-Ok $ctx 200 "policy" (Get-PolicySnapshotPayload)
            continue
        }

        if ($path -eq "policy/template" -and $req.HttpMethod -eq "GET") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "viewer")) { continue }
            $payload = Get-PolicySnapshotPayload
            $payload["template"] = [ordered]@{
                roles = (Get-DefaultRolePolicies)
            }
            Write-Ok $ctx 200 "policy_template" $payload
            continue
        }

        if ($path -eq "policy/apply" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
            $body = Read-RequestJson $req
            $unknown = @()
            if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("roles") -unknownKeys ([ref]$unknown))) {
                Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                continue
            }
            Set-RolePoliciesFromInput -rolesObjectOrArray $body.roles -source "policy_apply"
            Write-Ok $ctx 200 "policy_applied" (Get-PolicySnapshotPayload)
            continue
        }

        if ($path -eq "policy/validate" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "viewer")) { continue }
            $body = Read-RequestJson $req
            $unknown = @()
            if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("roles") -unknownKeys ([ref]$unknown))) {
                Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                continue
            }
            $normalized = New-NormalizedRolePolicies $body.roles
            $issues = @()
            foreach ($rk in @("viewer","operator","admin")) {
                $rp = $normalized[$rk]
                if ($null -eq $rp) {
                    $issues += [ordered]@{ role = $rk; code = "missing_role"; detail = "role policy not present after normalize" }
                    continue
                }
                $risk = [string]$rp.max_risk
                if (-not ($risk -in @("INFO","MED","HIGH"))) {
                    $issues += [ordered]@{ role = $rk; code = "invalid_risk"; detail = $risk }
                }
            }
            $policyRows = @()
            foreach ($rk in @("viewer","operator","admin")) {
                $policyRows += [ordered]@{
                    role = $rk
                    max_risk = [string]$normalized[$rk].max_risk
                    denied_actions = @($normalized[$rk].denied_actions)
                    denied_categories = @($normalized[$rk].denied_categories)
                }
            }
            Write-Ok $ctx 200 "policy_validated" ([ordered]@{
                valid = (@($issues).Count -eq 0)
                issues = $issues
                normalized_roles = $policyRows
            })
            continue
        }

        if ($path -eq "policy/reset" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
            $script:RolePolicies = Get-DefaultRolePolicies
            Push-PolicyTrace ([ordered]@{
                ts_utc = [DateTime]::UtcNow.ToString("o")
                source = "policy_reset"
                role = "admin"
                action = "policy_reset"
                category = "policy"
                risk = "INFO"
                allowed = $true
                reason = "policy_reset_to_defaults"
            })
            Write-Ok $ctx 200 "policy_reset" (Get-PolicySnapshotPayload)
            continue
        }

        if ($path -eq "policy/simulate" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "viewer")) { continue }
            $body = Read-RequestJson $req
            $unknown = @()
            if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("role","action") -unknownKeys ([ref]$unknown))) {
                Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                continue
            }
            $role = [string]$body.role
            if (-not $role) { $role = "viewer" }
            if (-not $script:RolePolicies.ContainsKey($role)) {
                Write-Err $ctx 400 "unknown_role" "Role is not defined in policy." "invalid_payload" @{ role = $role }
                continue
            }
            $actionId = [string]$body.action
            $def = $null
            if ($actionId -and $actionMap.ContainsKey($actionId)) {
                $def = $actionMap[$actionId]
            }
            $result = Evaluate-ActionPolicy -role $role -actionDef $def -actionId $actionId
            Push-PolicyTrace ([ordered]@{
                ts_utc = [DateTime]::UtcNow.ToString("o")
                source = "simulate"
                role = $role
                action = $actionId
                category = [string]$result.category
                risk = [string]$result.risk
                allowed = [bool]$result.allowed
                reason = [string]$result.reason
            })
            Write-Ok $ctx 200 "policy_simulation" ([ordered]@{
                result = $result
            })
            continue
        }

        if ($path -eq "policy/traces" -and $req.HttpMethod -eq "GET") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "viewer")) { continue }
            $query = Get-QueryMap $req
            $tail = 60
            if ($query.ContainsKey("tail")) {
                $tv = [int]$query["tail"]
                if ($tv -gt 0) { $tail = [Math]::Min(500, $tv) }
            }
            $rows = @()
            foreach ($r in $script:PolicyTrace) { $rows += $r }
            if ($rows.Count -gt $tail) {
                $rows = @($rows | Select-Object -First $tail)
            }
            Write-Ok $ctx 200 "policy_traces" ([ordered]@{
                traces = $rows
                count = @($rows).Count
            })
            continue
        }

        if ($path -eq "crash/summary" -and $req.HttpMethod -eq "GET") {
            Write-Ok $ctx 200 "crash_summary" ([ordered]@{
                crash = (Get-CrashSummaryPayload)
            })
            continue
        }

        if ($path -eq "plugins/health" -and $req.HttpMethod -eq "GET") {
            Write-Ok $ctx 200 "plugins_health" ([ordered]@{
                plugins = (Get-PluginHealthPayload)
            })
            continue
        }

        if ($path -eq "status" -and $req.HttpMethod -eq "GET") {
            $running = @()
            $queued = @()
            foreach ($id in @($script:Jobs.Keys)) {
                if (-not $id) { continue }
                if (-not (Test-JobExists -jobId $id)) { continue }
                $summary = Get-JobSummary (Get-JobEntryById -jobId $id)
                if ($summary.status -eq "running") { $running += $summary }
                if ($summary.status -eq "queued") { $queued += $summary }
            }
            $queued = @(
                $queued | Sort-Object -Property @(
                    @{
                        Expression = {
                            $p = [string]$_.priority
                            if ($script:PriorityOrder.ContainsKey($p)) { return [int]$script:PriorityOrder[$p] }
                            return 99
                        }
                        Ascending = $true
                    },
                    @{
                        Expression = { [string]$_.queued_utc }
                        Ascending = $true
                    }
                )
            )
            $statusData = [ordered]@{}
            $statusData["busy"] = [bool](Get-AgentBusy)
            $statusData["running"] = @($running)
            $statusData["queued"] = @($queued)
            $recentRows = @()
            foreach ($r in $script:Recent) { $recentRows += $r }
            $statusData["recent"] = $recentRows
            $statusData["running_count"] = [int](Get-RunningCount)
            $statusData["queue_count"] = [int](Get-QueueCount)
            Write-Ok $ctx 200 "status" $statusData
            continue
        }

        if ($path -eq "metrics" -and $req.HttpMethod -eq "GET") {
            try {
                $cpuLoad = [int](Get-CimInstance Win32_Processor | Measure-Object -Property LoadPercentage -Average).Average
                $os = Get-CimInstance Win32_OperatingSystem
                $memTotal = [double]$os.TotalVisibleMemorySize
                $memFree = [double]$os.FreePhysicalMemory
                $memPct = if ($memTotal -gt 0) { [Math]::Round(100.0 * (1.0 - $memFree / $memTotal), 1) } else { 0 }
                $sysDrive = ($env:SystemDrive -replace ":", "") + ":"
                $disk = Get-CimInstance Win32_LogicalDisk -Filter "DriveType=3 AND DeviceID='$sysDrive'" -ErrorAction SilentlyContinue
                $diskPct = if ($disk -and $disk.Size -gt 0) { [Math]::Round(100.0 * (1.0 - $disk.FreeSpace / $disk.Size), 1) } else { 0 }
                $uptimeSec = [int]([DateTime]::UtcNow - [DateTime]::Parse($script:StartedUtc)).TotalSeconds
                Write-Ok $ctx 200 "metrics" ([ordered]@{
                    cpu_load = $cpuLoad
                    mem_pct  = $memPct
                    disk_pct = $diskPct
                    uptime   = $uptimeSec
                    latency  = 0
                })
            } catch {
                Write-Err $ctx 500 "metrics_failed" "Failed to collect system metrics." "metrics_error" @{ message = [string]$_.Exception.Message }
            }
            continue
        }

        if ($path -eq "state" -and $req.HttpMethod -eq "GET") {
            $incidents = @()
            foreach ($r in $script:Recent) {
                $jStatus = [string]$r.status
                if ($jStatus -ne "failed" -and $jStatus -ne "timeout") { continue }
                $ts = if ($r.finished_utc) { [string]$r.finished_utc } elseif ($r.started_utc) { [string]$r.started_utc } else { [DateTime]::UtcNow.ToString("o") }
                $incidents += [ordered]@{
                    id        = [string]$r.id
                    type      = "job_failure"
                    severity  = if ($jStatus -eq "timeout") { "medium" } else { "high" }
                    timestamp = $ts
                    message   = "Job '{0}' {1}" -f [string]$r.action, $jStatus
                    nodeId    = "local"
                    status    = "open"
                }
            }
            Write-Ok $ctx 200 "state" ([ordered]@{ incidents = $incidents })
            continue
        }

        if ($path -eq "api/launcher/agent-status" -and $req.HttpMethod -eq "GET") {
            Write-Ok $ctx 200 "launcher_agent_status" ([ordered]@{
                status    = if (Get-AgentBusy) { "running" } else { "idle" }
                state     = "online"
                pid       = $PID
                timestamp = [DateTime]::UtcNow.ToString("o")
            })
            continue
        }

        if ($path -eq "api/launcher/audit" -and $req.HttpMethod -eq "GET") {
            $q = Get-QueryMap $req
            $tail = 40
            if ($q.ContainsKey("tail")) {
                $tv = [int]$q["tail"]
                if ($tv -gt 0) { $tail = [Math]::Min(200, $tv) }
            }
            $rows = @()
            if (Test-Path $script:AuditDir) {
                $auditFiles = Get-ChildItem -Path $script:AuditDir -Filter "*.json" -ErrorAction SilentlyContinue |
                    Sort-Object -Property LastWriteTimeUtc -Descending |
                    Select-Object -First $tail
                foreach ($f in $auditFiles) {
                    try {
                        $obj = Get-Content -Raw -Path $f.FullName | ConvertFrom-Json
                        $ts = if ($obj.finished_utc) { [string]$obj.finished_utc } else { [string]$obj.started_utc }
                        $rows += [ordered]@{
                            id       = [string]$obj.id
                            timestamp = $ts
                            action   = [string]$obj.action
                            operator = "agent"
                            status   = if ([bool]$obj.ok) { "success" } else { "failure" }
                            details  = "exit_code={0} duration_ms={1}" -f [string]$obj.exit_code, [string]$obj.duration_ms
                        }
                    } catch { }
                }
            }
            Write-Ok $ctx 200 "launcher_audit" ([ordered]@{ rows = $rows })
            continue
        }

        if ($path -eq "api/launcher/start-agent" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "operator")) { continue }
            Write-Ok $ctx 200 "start_agent" ([ordered]@{
                status  = "running"
                message = "Agent is already running in standalone mode."
            })
            continue
        }

        if ($path -eq "api/launcher/stop-agent" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
            Write-Ok $ctx 200 "stop_agent" ([ordered]@{
                status  = "stopping"
                message = "Agent is shutting down."
            })
            [System.Threading.ThreadPool]::QueueUserWorkItem({ param($l) Start-Sleep -Milliseconds 600; $l.Stop() }, $listener) | Out-Null
            continue
        }

        if ($path -eq "api/launcher/restart-agent" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
            Write-Ok $ctx 200 "restart_agent" ([ordered]@{
                status  = "not_supported"
                message = "Restart is not supported in standalone mode. Stop and restart the script manually."
            })
            continue
        }

        if ($path -eq "confirm/list" -and $req.HttpMethod -eq "GET") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "operator")) { continue }
            $query = Get-QueryMap $req
            $stateFilter = if ($query.ContainsKey("state")) { [string]$query["state"] } else { "pending" }
            $tail = 100
            if ($query.ContainsKey("tail")) {
                $tv = [int]$query["tail"]
                if ($tv -gt 0) { $tail = [Math]::Min(500, $tv) }
            }
            $rows = @(Get-ConfirmationRows)
            if ($stateFilter -and $stateFilter -ne "all") {
                $rows = @($rows | Where-Object { [string]$_.state -eq $stateFilter })
            }
            if ($rows.Count -gt $tail) {
                $rows = @($rows | Select-Object -First $tail)
            }
            Write-Ok $ctx 200 "confirmations" ([ordered]@{
                rows = $rows
                count = @($rows).Count
            })
            continue
        }

        if ($path -eq "confirm/revoke" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
            $body = Read-RequestJson $req
            $unknown = @()
            if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("id") -unknownKeys ([ref]$unknown))) {
                Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                continue
            }
            $confirmationId = [string]$body.id
            if (-not $confirmationId -or -not $script:Confirmations.ContainsKey($confirmationId)) {
                Write-Err $ctx 404 "confirmation_not_found" "Confirmation id not found." "not_found" @{ id = $confirmationId }
                continue
            }
            $script:Confirmations.Remove($confirmationId) | Out-Null
            Write-Ok $ctx 200 "confirmation_revoked" ([ordered]@{ id = $confirmationId })
            continue
        }

        if ($path -eq "confirm/request" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "operator")) { continue }
            $body = Read-RequestJson $req
            $unknown = @()
            if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("action") -unknownKeys ([ref]$unknown))) {
                Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                continue
            }
            $actionId = [string]$body.action
            if (-not $actionId -or -not $actionMap.ContainsKey($actionId)) {
                Write-Err $ctx 400 "unknown_action" "Action is not in catalog." "unknown_action" @{ action = $actionId }
                continue
            }
            $def = $actionMap[$actionId]
            if (-not (Is-CriticalAction $def)) {
                Write-Err $ctx 409 "confirmation_not_required" "Action does not require confirmation." "invalid_request" @{ action = $actionId }
                continue
            }
            $role = Resolve-RequestRole $req
            $c = New-Confirmation -actionId $actionId -role $role
            Write-Ok $ctx 202 "confirmation_created" ([ordered]@{ confirmation = $c })
            continue
        }

        if ($path -eq "run" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "operator")) { continue }
            if (Get-QueueCount -ge $script:MaxQueue) {
                Write-Err $ctx 429 "queue_full" "Agent queue is full." "queue_full" @{ max_queue = $script:MaxQueue }
                continue
            }
            $body = Read-RequestJson $req
            if (-not (Validate-RunPayload -ctx $ctx -body $body)) { continue }
            $actionId = [string]$body.action
            $priority = [string]$body.priority
            if (-not $priority) { $priority = "normal" }
            if (-not $actionMap.ContainsKey($actionId)) {
                Write-Err $ctx 400 "unknown_action" "Action is not in catalog." "unknown_action" @{ action = $actionId }
                continue
            }
            $def = $actionMap[$actionId]
            $role = Resolve-RequestRole $req
            if (-not (Ensure-ActionAllowedByPolicy -ctx $ctx -role $role -actionDef $def -actionId $actionId)) { continue }
            if (Is-CriticalAction $def) {
                $confirmationId = [string]$body.confirmation_id
                if (-not (Validate-Confirmation -confirmationId $confirmationId -actionId $actionId -role $role)) {
                    Write-Err $ctx 409 "confirmation_required" "Critical action requires valid confirmation_id." "confirmation_required" @{ action = $actionId; ttl_sec = $script:ConfirmationTtlSec }
                    continue
                }
            }
            $entry = Enqueue-Action -actionDef $def -actionId $actionId -priority $priority -source "api:run"
            while ($entry.status -eq "queued" -or $entry.status -eq "running") {
                Start-Sleep -Milliseconds 250
                Tick-Scheduler
                Dispatch-Queue
                Update-JobEntry $entry
            }
            $lines = Get-RecentLines -entry $entry -Tail 300
            $statusCode = if ($entry.ok) { 200 } else { 500 }
            Write-ApiResponse $ctx $statusCode ([bool]$entry.ok) (if ($entry.ok) { "ok" } else { "command_failed" }) "run completed" (if ($entry.ok) { $null } else { "command_failed" }) $null ([ordered]@{
                exit_code = $entry.exit_code
                queue_wait_ms = [int]$entry.queue_wait_ms
                duration_ms = [int]$entry.duration_ms
                output = ($lines -join "`n")
                command = [string]$entry.command
                profile = $Profile
                job_id = $entry.id
                status = $entry.status
                priority = $entry.priority
            })
            continue
        }

        if ($path -eq "run_async" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "operator")) { continue }
            if (Get-QueueCount -ge $script:MaxQueue) {
                Write-Err $ctx 429 "queue_full" "Agent queue is full." "queue_full" @{ max_queue = $script:MaxQueue }
                continue
            }
            $body = Read-RequestJson $req
            if (-not (Validate-RunPayload -ctx $ctx -body $body)) { continue }
            $actionId = [string]$body.action
            $priority = [string]$body.priority
            if (-not $priority) { $priority = "normal" }
            if (-not $actionMap.ContainsKey($actionId)) {
                Write-Err $ctx 400 "unknown_action" "Action is not in catalog." "unknown_action" @{ action = $actionId }
                continue
            }
            $def = $actionMap[$actionId]
            $role = Resolve-RequestRole $req
            if (-not (Ensure-ActionAllowedByPolicy -ctx $ctx -role $role -actionDef $def -actionId $actionId)) { continue }
            if (Is-CriticalAction $def) {
                $confirmationId = [string]$body.confirmation_id
                if (-not (Validate-Confirmation -confirmationId $confirmationId -actionId $actionId -role $role)) {
                    Write-Err $ctx 409 "confirmation_required" "Critical action requires valid confirmation_id." "confirmation_required" @{ action = $actionId; ttl_sec = $script:ConfirmationTtlSec }
                    continue
                }
            }
            $entry = Enqueue-Action -actionDef $def -actionId $actionId -priority $priority -source "api:run_async"
            Dispatch-Queue
            Write-Ok $ctx 202 "accepted" ([ordered]@{
                accepted = $true
                job = (Get-JobSummary $entry)
            })
            continue
        }

        if ($path -eq "dispatch/run_async" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "operator")) { continue }
            $body = Read-RequestJson $req
            if (-not (Validate-DispatchPayload -ctx $ctx -body $body)) { continue }

            $hostId = [string]$body.host_id
            if (-not $hostId) { $hostId = "local" }
            $actionId = [string]$body.action
            $priority = [string]$body.priority
            if (-not $priority) { $priority = "normal" }
            $confirmationId = [string]$body.confirmation_id

            if (-not $actionMap.ContainsKey($actionId)) {
                Write-Err $ctx 400 "unknown_action" "Action is not in catalog." "unknown_action" @{ action = $actionId }
                continue
            }
            $def = $actionMap[$actionId]
            $role = Resolve-RequestRole $req
            if (-not (Ensure-ActionAllowedByPolicy -ctx $ctx -role $role -actionDef $def -actionId $actionId)) { continue }

            if ($hostId -eq "local") {
                if (Get-QueueCount -ge $script:MaxQueue) {
                    Write-Err $ctx 429 "queue_full" "Agent queue is full." "queue_full" @{ max_queue = $script:MaxQueue }
                    continue
                }
                if (Is-CriticalAction $def) {
                    if (-not (Validate-Confirmation -confirmationId $confirmationId -actionId $actionId -role $role)) {
                        Write-Err $ctx 409 "confirmation_required" "Critical action requires valid confirmation_id." "confirmation_required" @{ action = $actionId; ttl_sec = $script:ConfirmationTtlSec }
                        continue
                    }
                }
                $entry = Enqueue-Action -actionDef $def -actionId $actionId -priority $priority -source "api:dispatch:local"
                Dispatch-Queue
                Write-Ok $ctx 202 "accepted" ([ordered]@{
                    host_id = "local"
                    dispatched = "local"
                    job = (Get-JobSummary $entry)
                })
                continue
            }

            $host = Get-HostById -hostId $hostId
            if ($null -eq $host -or -not [bool]$host.enabled) {
                Write-Err $ctx 404 "host_not_found" "Host id not found or disabled." "host_not_found" @{ host_id = $hostId }
                continue
            }
            if (-not [string]$host.token) {
                Write-Err $ctx 400 "host_missing_token" "Target host is missing token configuration." "invalid_host_config" @{ host_id = $hostId }
                continue
            }
            try {
                $remoteHeaders = @{ "X-HyperCore-Token" = [string]$host.token }
                $remoteBody = [ordered]@{
                    action = $actionId
                    priority = $priority
                }
                if ($confirmationId) { $remoteBody["confirmation_id"] = $confirmationId }
                $remoteResp = Invoke-WebRequest -Method Post -Uri ("{0}/run_async" -f [string]$host.url) -Headers $remoteHeaders -ContentType "application/json" -Body ($remoteBody | ConvertTo-Json -Depth 8 -Compress) -TimeoutSec 8
                $remoteStatus = [int]$remoteResp.StatusCode
                $remoteJson = $null
                if ($remoteResp.Content) {
                    try { $remoteJson = $remoteResp.Content | ConvertFrom-Json } catch {}
                }
                if ($remoteStatus -ge 200 -and $remoteStatus -lt 300 -and $remoteJson) {
                    Write-Ok $ctx 202 "accepted" ([ordered]@{
                        host_id = [string]$host.id
                        dispatched = "remote"
                        remote = $remoteJson
                    })
                } else {
                    Write-Err $ctx 502 "remote_dispatch_failed" "Remote host rejected dispatch." "remote_error" @{ host_id = [string]$host.id; status = $remoteStatus; response = $remoteResp.Content }
                }
            } catch {
                Write-Err $ctx 502 "remote_dispatch_error" "Remote dispatch failed." "remote_error" @{ host_id = [string]$host.id; message = [string]$_.Exception.Message }
            }
            continue
        }

        if ($path -eq "dispatch/fanout" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "operator")) { continue }
            $body = Read-RequestJson $req
            $unknown = @()
            if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("action","priority","host_ids","include_local","enabled_only") -unknownKeys ([ref]$unknown))) {
                Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                continue
            }
            $actionId = [string]$body.action
            if (-not $actionId -or -not $actionMap.ContainsKey($actionId)) {
                Write-Err $ctx 400 "unknown_action" "Action is not in catalog." "unknown_action" @{ action = $actionId }
                continue
            }
            $priority = [string]$body.priority
            if (-not $priority) { $priority = "normal" }
            if (-not $script:PriorityOrder.ContainsKey($priority)) { $priority = "normal" }
            $includeLocal = $true
            if ($body.PSObject.Properties.Name.Contains("include_local")) { $includeLocal = [bool]$body.include_local }
            $enabledOnly = $true
            if ($body.PSObject.Properties.Name.Contains("enabled_only")) { $enabledOnly = [bool]$body.enabled_only }

            $role = Resolve-RequestRole $req
            $def = $actionMap[$actionId]
            if (-not (Ensure-ActionAllowedByPolicy -ctx $ctx -role $role -actionDef $def -actionId $actionId)) { continue }

            $targets = @()
            if ($body.PSObject.Properties.Name.Contains("host_ids") -and $body.host_ids) {
                foreach ($hid in @($body.host_ids)) {
                    $id = [string]$hid
                    if ($id) { $targets += $id }
                }
            } else {
                if ($includeLocal) { $targets += "local" }
                foreach ($h in @($script:Hosts)) {
                    $id = [string]$h.id
                    if (-not $id -or $id -eq "local") { continue }
                    if ($enabledOnly -and -not [bool]$h.enabled) { continue }
                    $targets += $id
                }
            }
            $targets = @($targets | Select-Object -Unique)
            if ($targets.Count -eq 0) {
                Write-Err $ctx 400 "no_targets" "No target hosts resolved for fanout." "invalid_payload" @{ include_local = $includeLocal; enabled_only = $enabledOnly }
                continue
            }

            $results = @()
            foreach ($hostId in $targets) {
                if ($hostId -eq "local") {
                    if (Get-QueueCount -ge $script:MaxQueue) {
                        $results += [ordered]@{ host_id = "local"; ok = $false; code = "queue_full"; message = "Local queue full."; job = $null }
                        continue
                    }
                    if (Is-CriticalAction $def) {
                        $autoConf = New-Confirmation -actionId $actionId -role $role
                        if (-not (Validate-Confirmation -confirmationId ([string]$autoConf.id) -actionId $actionId -role $role)) {
                            $results += [ordered]@{ host_id = "local"; ok = $false; code = "confirmation_required"; message = "Local confirmation validation failed."; job = $null }
                            continue
                        }
                    }
                    $entry = Enqueue-Action -actionDef $def -actionId $actionId -priority $priority -source "api:fanout:local"
                    Dispatch-Queue
                    $results += [ordered]@{ host_id = "local"; ok = $true; code = "accepted"; message = "Local dispatch accepted."; job = (Get-JobSummary $entry) }
                    continue
                }

                $host = Get-HostById -hostId $hostId
                if ($null -eq $host -or -not [bool]$host.enabled) {
                    $results += [ordered]@{ host_id = $hostId; ok = $false; code = "host_not_found"; message = "Host not found or disabled."; job = $null }
                    continue
                }
                if (-not [string]$host.token) {
                    $results += [ordered]@{ host_id = $hostId; ok = $false; code = "host_missing_token"; message = "Host token missing."; job = $null }
                    continue
                }
                try {
                    $remoteConfirmationId = ""
                    if (Is-CriticalAction $def) {
                        $confirmResp = Invoke-RemoteJson -Method "POST" -Url ("{0}/confirm/request" -f [string]$host.url) -Token ([string]$host.token) -Body ([ordered]@{ action = $actionId })
                        if ($confirmResp.status -lt 200 -or $confirmResp.status -ge 300) {
                            $results += [ordered]@{ host_id = $hostId; ok = $false; code = "remote_confirmation_failed"; message = "Remote confirmation request failed."; job = $null; status = $confirmResp.status }
                            continue
                        }
                        $remoteConfirmationId = [string]$confirmResp.json.confirmation.id
                    }
                    $remoteBody = [ordered]@{
                        action = $actionId
                        priority = $priority
                    }
                    if ($remoteConfirmationId) { $remoteBody["confirmation_id"] = $remoteConfirmationId }
                    $remote = Invoke-RemoteJson -Method "POST" -Url ("{0}/run_async" -f [string]$host.url) -Token ([string]$host.token) -Body $remoteBody
                    if ($remote.status -ge 200 -and $remote.status -lt 300) {
                        $results += [ordered]@{ host_id = $hostId; ok = $true; code = "accepted"; message = "Remote dispatch accepted."; job = $remote.json.job }
                    } else {
                        $results += [ordered]@{ host_id = $hostId; ok = $false; code = "remote_dispatch_failed"; message = "Remote dispatch rejected."; job = $null; status = $remote.status }
                    }
                } catch {
                    $results += [ordered]@{ host_id = $hostId; ok = $false; code = "remote_dispatch_error"; message = [string]$_.Exception.Message; job = $null }
                }
            }

            $accepted = @($results | Where-Object { $_.ok }).Count
            Write-Ok $ctx 202 "fanout_dispatched" ([ordered]@{
                accepted = $accepted
                total = @($results).Count
                action = $actionId
                priority = $priority
                results = $results
            })
            continue
        }

        if ($path -eq "dispatch/jobs" -and $req.HttpMethod -eq "GET") {
            $q = Get-QueryMap $req
            $hostId = [string]$q["host_id"]
            if (-not $hostId) { $hostId = "local" }
            if ($hostId -eq "local") {
                $jobs = @()
                foreach ($id in @($script:Jobs.Keys)) {
                    if (-not $id) { continue }
                    if (-not (Test-JobExists -jobId $id)) { continue }
                    $jobs += (Get-JobSummary (Get-JobEntryById -jobId $id))
                }
                $jobs = @($jobs | Sort-Object queued_utc -Descending)
                Write-Ok $ctx 200 "jobs" ([ordered]@{ host_id = "local"; jobs = $jobs })
                continue
            }
            $host = Get-HostById -hostId $hostId
            if ($null -eq $host -or -not [bool]$host.enabled) {
                Write-Err $ctx 404 "host_not_found" "Host id not found or disabled." "host_not_found" @{ host_id = $hostId }
                continue
            }
            if (-not [string]$host.token) {
                Write-Err $ctx 400 "host_missing_token" "Target host is missing token configuration." "invalid_host_config" @{ host_id = $hostId }
                continue
            }
            try {
                $remote = Invoke-RemoteJson -Method "GET" -Url ("{0}/jobs" -f [string]$host.url) -Token ([string]$host.token)
                Write-Ok $ctx 200 "jobs" ([ordered]@{
                    host_id = [string]$host.id
                    proxy = $true
                    jobs = if ($remote.json -and $remote.json.jobs) { $remote.json.jobs } else { @() }
                })
            } catch {
                Write-Err $ctx 502 "remote_dispatch_error" "Remote jobs query failed." "remote_error" @{ host_id = [string]$host.id; message = [string]$_.Exception.Message }
            }
            continue
        }

        if ($path -eq "dispatch/job" -and $req.HttpMethod -eq "GET") {
            $q = Get-QueryMap $req
            $hostId = [string]$q["host_id"]
            if (-not $hostId) { $hostId = "local" }
            $jobId = [string]$q["id"]
            $tail = if ($q.ContainsKey("tail")) { [string]$q["tail"] } else { "400" }
            if (-not $jobId) {
                Write-Err $ctx 400 "invalid_payload" "id query parameter is required." "invalid_payload" @{ id = $jobId }
                continue
            }
            if ($hostId -eq "local") {
                if (-not (Test-JobExists -jobId $jobId)) {
                    Write-Err $ctx 404 "job_not_found" "Job id not found." "job_not_found" @{ id = $jobId }
                    continue
                }
                $entry = Get-JobEntryById -jobId $jobId
                $summary = Get-JobSummary $entry
                $tailNum = 400
                [void][int]::TryParse($tail, [ref]$tailNum)
                $tailNum = [Math]::Max(20, [Math]::Min(4000, $tailNum))
                $lines = Get-RecentLines -entry $entry -Tail $tailNum
                Write-Ok $ctx 200 "job" ([ordered]@{
                    host_id = "local"
                    job = $summary
                    output = ($lines -join "`n")
                    line_count = $entry.line_count
                })
                continue
            }
            $host = Get-HostById -hostId $hostId
            if ($null -eq $host -or -not [bool]$host.enabled) {
                Write-Err $ctx 404 "host_not_found" "Host id not found or disabled." "host_not_found" @{ host_id = $hostId }
                continue
            }
            if (-not [string]$host.token) {
                Write-Err $ctx 400 "host_missing_token" "Target host is missing token configuration." "invalid_host_config" @{ host_id = $hostId }
                continue
            }
            try {
                $url = "{0}/job?id={1}&tail={2}" -f [string]$host.url, [System.Web.HttpUtility]::UrlEncode($jobId), [System.Web.HttpUtility]::UrlEncode($tail)
                $remote = Invoke-RemoteJson -Method "GET" -Url $url -Token ([string]$host.token)
                if ($remote.status -ge 200 -and $remote.status -lt 300) {
                    Write-Ok $ctx 200 "job" ([ordered]@{
                        host_id = [string]$host.id
                        proxy = $true
                        job = $remote.json.job
                        output = $remote.json.output
                        line_count = $remote.json.line_count
                    })
                } else {
                    Write-Err $ctx 502 "remote_dispatch_failed" "Remote job query failed." "remote_error" @{ host_id = [string]$host.id; status = $remote.status; response = $remote.raw }
                }
            } catch {
                Write-Err $ctx 502 "remote_dispatch_error" "Remote job query failed." "remote_error" @{ host_id = [string]$host.id; message = [string]$_.Exception.Message }
            }
            continue
        }

        if ($path -eq "dispatch/job/cancel" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
            $body = Read-RequestJson $req
            $unknown = @()
            if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("host_id","id") -unknownKeys ([ref]$unknown))) {
                Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                continue
            }
            $hostId = [string]$body.host_id
            if (-not $hostId) { $hostId = "local" }
            $jobId = [string]$body.id
            if (-not $jobId) {
                Write-Err $ctx 400 "invalid_payload" "id is required." "invalid_payload" @{ id = $jobId }
                continue
            }
            if ($hostId -eq "local") {
                if (-not (Test-JobExists -jobId $jobId)) {
                    Write-Err $ctx 404 "job_not_found" "Job id not found." "job_not_found" @{ id = $jobId }
                    continue
                }
                $entry = Get-JobEntryById -jobId $jobId
                if ($entry.status -in @("completed","failed","cancelled")) {
                    Write-Err $ctx 409 "job_not_running" "Job is not running." "job_not_running" @{ id = $jobId; status = $entry.status }
                    continue
                }
                Cancel-JobEntry $entry
                Dispatch-Queue
                Write-Ok $ctx 200 "job cancelled" ([ordered]@{ host_id = "local"; job = (Get-JobSummary $entry) })
                continue
            }
            $host = Get-HostById -hostId $hostId
            if ($null -eq $host -or -not [bool]$host.enabled) {
                Write-Err $ctx 404 "host_not_found" "Host id not found or disabled." "host_not_found" @{ host_id = $hostId }
                continue
            }
            if (-not [string]$host.token) {
                Write-Err $ctx 400 "host_missing_token" "Target host is missing token configuration." "invalid_host_config" @{ host_id = $hostId }
                continue
            }
            try {
                $remote = Invoke-RemoteJson -Method "POST" -Url ("{0}/job/cancel" -f [string]$host.url) -Token ([string]$host.token) -Body ([ordered]@{ id = $jobId })
                if ($remote.status -ge 200 -and $remote.status -lt 300) {
                    Write-Ok $ctx 200 "job cancelled" ([ordered]@{
                        host_id = [string]$host.id
                        proxy = $true
                        remote = $remote.json
                    })
                } else {
                    Write-Err $ctx 502 "remote_dispatch_failed" "Remote cancel failed." "remote_error" @{ host_id = [string]$host.id; status = $remote.status; response = $remote.raw }
                }
            } catch {
                Write-Err $ctx 502 "remote_dispatch_error" "Remote cancel failed." "remote_error" @{ host_id = [string]$host.id; message = [string]$_.Exception.Message }
            }
            continue
        }

        if ($path -eq "job" -and $req.HttpMethod -eq "GET") {
            $q = Get-QueryMap $req
            $jobId = [string]$q["id"]
            if (-not $jobId -or -not (Test-JobExists -jobId $jobId)) {
                Write-Err $ctx 404 "job_not_found" "Job id not found." "job_not_found" @{ id = $jobId }
                continue
            }
            $entry = Get-JobEntryById -jobId $jobId
            $summary = Get-JobSummary $entry
            $tail = 400
            if ($q.ContainsKey("tail")) {
                $parsed = 400
                [void][int]::TryParse([string]$q["tail"], [ref]$parsed)
                $tail = [Math]::Max(20, [Math]::Min(4000, $parsed))
            }
            $lines = Get-RecentLines -entry $entry -Tail $tail
            Write-Ok $ctx 200 "job" ([ordered]@{
                job = $summary
                output = ($lines -join "`n")
                line_count = $entry.line_count
            })
            continue
        }

        if ($path -eq "job/stream" -and $req.HttpMethod -eq "GET") {
            $q = Get-QueryMap $req
            $jobId = [string]$q["id"]
            if (-not $jobId -or -not (Test-JobExists -jobId $jobId)) {
                Write-Err $ctx 404 "job_not_found" "Job id not found." "job_not_found" @{ id = $jobId }
                continue
            }

            $timeoutSec = 120
            if ($q.ContainsKey("timeout_sec")) {
                $parsed = 120
                [void][int]::TryParse([string]$q["timeout_sec"], [ref]$parsed)
                $timeoutSec = [Math]::Max(5, [Math]::Min(600, $parsed))
            }
            $pollMs = 600
            if ($q.ContainsKey("poll_ms")) {
                $parsedPoll = 600
                [void][int]::TryParse([string]$q["poll_ms"], [ref]$parsedPoll)
                $pollMs = [Math]::Max(200, [Math]::Min(2000, $parsedPoll))
            }

            $resp = $ctx.Response
            Add-CorsHeaders $req $resp
            $resp.StatusCode = 200
            $resp.ContentType = "text/event-stream"
            $resp.Headers["Cache-Control"] = "no-cache"
            $resp.Headers["Connection"] = "keep-alive"

            $entry = Get-JobEntryById -jobId $jobId
            $start = [DateTime]::UtcNow
            $lastCount = -1
            try {
                Write-SseEvent $resp "hello" ([ordered]@{ ok = $true; job_id = $jobId; started_utc = [DateTime]::UtcNow.ToString("o") })
                while ($true) {
                    Update-JobEntry $entry
                    $summary = Get-JobSummary $entry
                    if ($entry.line_count -ne $lastCount) {
                        $lastCount = $entry.line_count
                        $tail = Get-RecentLines -entry $entry -Tail 200
                        Write-SseEvent $resp "tail" ([ordered]@{
                            job = $summary
                            line_count = $entry.line_count
                            output = ($tail -join "`n")
                        })
                    } else {
                        Write-SseEvent $resp "heartbeat" ([ordered]@{ job = $summary })
                    }

                    if ($summary.status -in @("completed","failed","cancelled")) {
                        Write-SseEvent $resp "done" ([ordered]@{ job = $summary })
                        break
                    }
                    if (([DateTime]::UtcNow - $start).TotalSeconds -ge $timeoutSec) {
                        Write-SseEvent $resp "timeout" ([ordered]@{ job = $summary; timeout_sec = $timeoutSec })
                        break
                    }
                    Start-Sleep -Milliseconds $pollMs
                }
            } catch {
                try {
                    Write-SseEvent $resp "error" ([ordered]@{ ok = $false; message = [string]$_.Exception.Message })
                } catch {}
            } finally {
                try { $resp.OutputStream.Close() } catch {}
            }
            continue
        }

        if ($path -eq "dispatch/job/stream" -and $req.HttpMethod -eq "GET") {
            $q = Get-QueryMap $req
            $hostId = [string]$q["host_id"]
            if (-not $hostId) { $hostId = "local" }
            $jobId = [string]$q["id"]
            if (-not $jobId) {
                Write-Err $ctx 400 "invalid_payload" "id query parameter is required." "invalid_payload" @{ id = $jobId }
                continue
            }

            $timeoutSec = 120
            if ($q.ContainsKey("timeout_sec")) {
                $parsed = 120
                [void][int]::TryParse([string]$q["timeout_sec"], [ref]$parsed)
                $timeoutSec = [Math]::Max(5, [Math]::Min(600, $parsed))
            }
            $pollMs = 600
            if ($q.ContainsKey("poll_ms")) {
                $parsedPoll = 600
                [void][int]::TryParse([string]$q["poll_ms"], [ref]$parsedPoll)
                $pollMs = [Math]::Max(200, [Math]::Min(2000, $parsedPoll))
            }

            $resp = $ctx.Response
            Add-CorsHeaders $req $resp
            $resp.StatusCode = 200
            $resp.ContentType = "text/event-stream"
            $resp.Headers["Cache-Control"] = "no-cache"
            $resp.Headers["Connection"] = "keep-alive"

            $start = [DateTime]::UtcNow
            $lastHash = ""
            try {
                Write-SseEvent $resp "hello" ([ordered]@{ ok = $true; job_id = $jobId; host_id = $hostId; started_utc = [DateTime]::UtcNow.ToString("o") })
                while ($true) {
                    if ($hostId -eq "local") {
                        if (-not (Test-JobExists -jobId $jobId)) {
                            Write-SseEvent $resp "error" ([ordered]@{ ok = $false; code = "job_not_found"; job_id = $jobId; host_id = $hostId })
                            break
                        }
                        $entry = Get-JobEntryById -jobId $jobId
                        $summary = Get-JobSummary $entry
                        $tailLines = Get-RecentLines -entry $entry -Tail 200
                        $output = ($tailLines -join "`n")
                        $hash = ("{0}:{1}:{2}" -f [string]$summary.status, [int]$summary.line_count, [int]$summary.duration_ms)
                        if ($hash -ne $lastHash) {
                            $lastHash = $hash
                            Write-SseEvent $resp "tail" ([ordered]@{ job = $summary; line_count = $summary.line_count; output = $output; host_id = $hostId })
                        } else {
                            Write-SseEvent $resp "heartbeat" ([ordered]@{ job = $summary; host_id = $hostId })
                        }
                        if ($summary.status -in @("completed","failed","cancelled")) {
                            Write-SseEvent $resp "done" ([ordered]@{ job = $summary; host_id = $hostId })
                            break
                        }
                    } else {
                        $host = Get-HostById -hostId $hostId
                        if ($null -eq $host -or -not [bool]$host.enabled) {
                            Write-SseEvent $resp "error" ([ordered]@{ ok = $false; code = "host_not_found"; host_id = $hostId })
                            break
                        }
                        if (-not [string]$host.token) {
                            Write-SseEvent $resp "error" ([ordered]@{ ok = $false; code = "host_missing_token"; host_id = $hostId })
                            break
                        }
                        $url = "{0}/job?id={1}&tail=200" -f [string]$host.url, [System.Web.HttpUtility]::UrlEncode($jobId)
                        $remote = Invoke-RemoteJson -Method "GET" -Url $url -Token ([string]$host.token)
                        if ($remote.status -lt 200 -or $remote.status -ge 300 -or -not $remote.json -or -not $remote.json.job) {
                            Write-SseEvent $resp "error" ([ordered]@{ ok = $false; code = "remote_job_error"; host_id = $hostId; status = $remote.status })
                            break
                        }
                        $summary = $remote.json.job
                        $output = if ($remote.json.output) { [string]$remote.json.output } else { "" }
                        $hash = ("{0}:{1}:{2}" -f [string]$summary.status, [int]$summary.line_count, [int]$summary.duration_ms)
                        if ($hash -ne $lastHash) {
                            $lastHash = $hash
                            Write-SseEvent $resp "tail" ([ordered]@{ job = $summary; line_count = $summary.line_count; output = $output; host_id = $hostId })
                        } else {
                            Write-SseEvent $resp "heartbeat" ([ordered]@{ job = $summary; host_id = $hostId })
                        }
                        if ([string]$summary.status -in @("completed","failed","cancelled")) {
                            Write-SseEvent $resp "done" ([ordered]@{ job = $summary; host_id = $hostId })
                            break
                        }
                    }

                    if (([DateTime]::UtcNow - $start).TotalSeconds -ge $timeoutSec) {
                        Write-SseEvent $resp "timeout" ([ordered]@{ job_id = $jobId; host_id = $hostId; timeout_sec = $timeoutSec })
                        break
                    }
                    Start-Sleep -Milliseconds $pollMs
                }
            } catch {
                try {
                    Write-SseEvent $resp "error" ([ordered]@{ ok = $false; message = [string]$_.Exception.Message; host_id = $hostId })
                } catch {}
            } finally {
                try { $resp.OutputStream.Close() } catch {}
            }
            continue
        }

        if ($path -eq "jobs" -and $req.HttpMethod -eq "GET") {
            $jobs = @()
            foreach ($id in @($script:Jobs.Keys)) {
                if (-not $id) { continue }
                if (-not (Test-JobExists -jobId $id)) { continue }
                $jobs += (Get-JobSummary (Get-JobEntryById -jobId $id))
            }
            $jobs = @($jobs | Sort-Object queued_utc -Descending)
            Write-Ok $ctx 200 "jobs" ([ordered]@{ jobs = $jobs })
            continue
        }

        if ($path -eq "job/cancel" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
            $body = Read-RequestJson $req
            if (-not (Validate-CancelPayload -ctx $ctx -body $body)) { continue }
            $jobId = [string]$body.id
            if (-not $jobId -or -not (Test-JobExists -jobId $jobId)) {
                Write-Err $ctx 404 "job_not_found" "Job id not found." "job_not_found" @{ id = $jobId }
                continue
            }
            $entry = Get-JobEntryById -jobId $jobId
            if ($entry.status -in @("completed", "failed", "cancelled")) {
                Write-Err $ctx 409 "job_not_running" "Job is not running." "job_not_running" @{ id = $jobId; status = $entry.status }
                continue
            }
            Cancel-JobEntry $entry
            Dispatch-Queue
            Write-Ok $ctx 200 "job cancelled" ([ordered]@{ job = (Get-JobSummary $entry) })
            continue
        }

        if ($path -eq "scheduler" -and $req.HttpMethod -eq "GET") {
            $tasks = @()
            foreach ($sid in @($script:Schedules.Keys)) {
                $tasks += $script:Schedules[$sid]
            }
            $tasks = @($tasks | Sort-Object id)
            Write-Ok $ctx 200 "scheduler" ([ordered]@{
                enabled = $script:SchedulerEnabled
                tasks = $tasks
                queue_count = (Get-QueueCount)
            })
            continue
        }

        if ($path -eq "scheduler/templates" -and $req.HttpMethod -eq "GET") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "viewer")) { continue }
            Write-Ok $ctx 200 "scheduler_templates" ([ordered]@{
                templates = (Get-SchedulerTemplates)
            })
            continue
        }

        if ($path -eq "scheduler/apply_template" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
            $body = Read-RequestJson $req
            $unknown = @()
            if (-not (Test-BodyAllowedKeys -body $body -allowedKeys @("id") -unknownKeys ([ref]$unknown))) {
                Write-Err $ctx 400 "invalid_payload" "Payload contains unsupported fields." "invalid_payload" @{ unknown_fields = $unknown }
                continue
            }
            $tplId = [string]$body.id
            if (-not $tplId) {
                Write-Err $ctx 400 "invalid_payload" "Template id is required." "invalid_payload" @{ id = $tplId }
                continue
            }
            if (-not (Apply-SchedulerTemplate -templateId $tplId)) {
                Write-Err $ctx 404 "template_not_found" "Scheduler template id not found." "not_found" @{ id = $tplId }
                continue
            }
            $tasks = @()
            foreach ($sid in @($script:Schedules.Keys)) { $tasks += $script:Schedules[$sid] }
            $tasks = @($tasks | Sort-Object id)
            Write-Ok $ctx 200 "scheduler_template_applied" ([ordered]@{
                applied_template = $tplId
                enabled = $script:SchedulerEnabled
                tasks = $tasks
            })
            continue
        }

        if ($path -eq "compliance/report" -and $req.HttpMethod -eq "GET") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "viewer")) { continue }
            Write-Ok $ctx 200 "compliance_report" (Get-ComplianceReportPayload)
            continue
        }

        if ($path -eq "security/regression/template" -and $req.HttpMethod -eq "GET") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "viewer")) { continue }
            $report = Get-ComplianceReportPayload
            Write-Ok $ctx 200 "security_regression_template" ([ordered]@{
                template = $report.security_regression_template
            })
            continue
        }

        if ($path -eq "scheduler/run_now" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "operator")) { continue }
            if (Get-QueueCount -ge $script:MaxQueue) {
                Write-Err $ctx 429 "queue_full" "Agent queue is full." "queue_full" @{ max_queue = $script:MaxQueue }
                continue
            }
            $body = Read-RequestJson $req
            if (-not (Validate-SchedulerRunPayload -ctx $ctx -body $body)) { continue }
            $id = [string]$body.id
            if (-not $id -or -not $script:Schedules.ContainsKey($id)) {
                Write-Err $ctx 404 "schedule_not_found" "Schedule id not found." "schedule_not_found" @{ id = $id }
                continue
            }
            $schedule = $script:Schedules[$id]
            $role = Resolve-RequestRole $req
            $def = $actionMap[[string]$schedule.action]
            if (-not (Ensure-ActionAllowedByPolicy -ctx $ctx -role $role -actionDef $def -actionId [string]$schedule.action)) { continue }
            $entry = Enqueue-Action -actionDef $actionMap[[string]$schedule.action] -actionId [string]$schedule.action -priority [string]$schedule.priority -source "scheduler:manual:$id"
            $schedule.last_run_utc = [DateTime]::UtcNow.ToString("o")
            $schedule.next_run_utc = ([DateTime]::UtcNow).AddSeconds([int]$schedule.interval_sec).ToString("o")
            Dispatch-Queue
            Write-Ok $ctx 202 "schedule accepted" ([ordered]@{ job = (Get-JobSummary $entry); schedule = $schedule })
            continue
        }

        if ($path -eq "scheduler/update" -and $req.HttpMethod -eq "POST") {
            if (-not (Ensure-RoleAtLeast -ctx $ctx -requiredRole "admin")) { continue }
            $body = Read-RequestJson $req
            if (-not (Validate-SchedulerUpdatePayload -ctx $ctx -body $body)) { continue }
            $id = [string]$body.id
            if (-not $id -or -not $script:Schedules.ContainsKey($id)) {
                Write-Err $ctx 404 "schedule_not_found" "Schedule id not found." "schedule_not_found" @{ id = $id }
                continue
            }
            $schedule = $script:Schedules[$id]
            if ($body.PSObject.Properties.Name.Contains("enabled")) {
                $schedule.enabled = [bool]$body.enabled
            }
            if ($body.PSObject.Properties.Name.Contains("interval_sec")) {
                $schedule.interval_sec = [Math]::Max(60, [int]$body.interval_sec)
                $schedule.next_run_utc = ([DateTime]::UtcNow).AddSeconds([int]$schedule.interval_sec).ToString("o")
            }
            if ($body.PSObject.Properties.Name.Contains("priority")) {
                $p = [string]$body.priority
                if ($script:PriorityOrder.ContainsKey($p)) {
                    $schedule.priority = $p
                }
            }
            Write-Ok $ctx 200 "schedule updated" ([ordered]@{ schedule = $schedule })
            continue
        }

            Write-Err $ctx 404 "not_found" "Endpoint not found." "not_found" @{ path = $path; method = $req.HttpMethod }
        } catch {
            $line = 0
            try { $line = [int]$_.InvocationInfo.ScriptLineNumber } catch {}
            $msg = [string]$_.Exception.Message
            try {
                Write-Err $ctx 500 "internal_error" "Unhandled agent request error." "internal_error" @{
                    path = $path
                    method = $req.HttpMethod
                    line = $line
                    message = $msg
                }
            } catch {}
            Write-Host ("[dashboard-agent] request error path={0} method={1} line={2} message={3}" -f $path, $req.HttpMethod, $line, $msg) -ForegroundColor Red
            continue
        }
    }
} finally {
    foreach ($id in @($script:Jobs.Keys)) {
        if (-not $id) { continue }
        if (-not (Test-JobExists -jobId $id)) { continue }
        try {
            $e = Get-JobEntryById -jobId $id
            if ($e.job) {
                Stop-Job -Job $e.job -ErrorAction SilentlyContinue
                Remove-Job -Job $e.job -Force -ErrorAction SilentlyContinue
            }
            Write-AuditLog $e
        } catch {}
    }
    if ($listener.IsListening) { $listener.Stop() }
    $listener.Close()
}

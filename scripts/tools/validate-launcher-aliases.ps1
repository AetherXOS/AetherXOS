param()

$ErrorActionPreference = "Stop"

function Get-RepoPath {
    param([string]$RelativePath)
    return (Join-Path $PSScriptRoot "..\..\$RelativePath")
}

function Assert-Exists {
    param(
        [string]$Path,
        [string]$Message
    )

    if (-not (Test-Path $Path)) {
        throw $Message
    }
}

$actionsPath = Get-RepoPath "scripts\config\quick.actions.json"
$launchersDir = Get-RepoPath "scripts\launchers"

Assert-Exists -Path $actionsPath -Message "quick action catalog missing: $actionsPath"
Assert-Exists -Path $launchersDir -Message "launcher directory missing: $launchersDir"

$actions = Get-Content -Raw -Path $actionsPath -Encoding UTF8 | ConvertFrom-Json
if (-not $actions) {
    throw "quick action catalog is empty"
}

$expectedLaunchers = @{
    "guided-bootstrap" = "guided-bootstrap.ps1"
    "environment-audit" = "environment-audit.ps1"
    "environment-repair" = "environment-repair.ps1"
    "dashboard-workspace-setup" = "dashboard-workspace-setup.ps1"
    "release-readiness" = "release-readiness.ps1"
    "workspace-bootstrap" = "workspace-bootstrap.ps1"
    "build-and-smoke" = "build-and-smoke.ps1"
    "build-boot-iso" = "build-boot-iso.ps1"
    "emulator-smoke" = "run-emulator-smoke.ps1"
    "emulator-interactive" = "run-emulator-interactive.ps1"
    "dashboard-build" = "dashboard-build.ps1"
    "dashboard-validation" = "dashboard-validation.ps1"
    "dashboard-open" = "dashboard-open.ps1"
    "quality-gate" = "quality-gate.ps1"
    "agent-contract-verification" = "agent-contract-verification.ps1"
    "agent-service-start" = "start-agent-service.ps1"
    "agent-service-start-unsafe" = "start-agent-service-unsafe.ps1"
}

$catalogKeys = @($actions | ForEach-Object { [string]$_.key })
$duplicates = $catalogKeys | Group-Object | Where-Object { $_.Count -gt 1 }
if ($duplicates) {
    $names = ($duplicates | ForEach-Object { $_.Name }) -join ", "
    throw "duplicate quick action keys: $names"
}

foreach ($entry in $expectedLaunchers.GetEnumerator()) {
    if ($catalogKeys -notcontains $entry.Key) {
        throw "missing quick action key: $($entry.Key)"
    }

    $launcherPath = Join-Path $launchersDir $entry.Value
    Assert-Exists -Path $launcherPath -Message "missing launcher wrapper: $launcherPath"

    $content = Get-Content -Raw -Path $launcherPath -Encoding UTF8
    if ($content -notmatch [regex]::Escape("-Action `"$($entry.Key)`"")) {
        throw "launcher $($entry.Value) does not dispatch action $($entry.Key)"
    }
}

Write-Host "launcher alias validation: PASS" -ForegroundColor Green
exit 0

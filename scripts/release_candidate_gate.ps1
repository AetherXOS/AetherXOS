param(
    [switch]$SkipHostTests,
    [switch]$QemuDryRun,
    [ValidateSet("direct","iso")]
    [string]$QemuBootMode = "direct",
    [string]$QemuIsoPath = "",
    [switch]$QemuBuildIso,
    [string]$QemuLimineBinDir = "",
    [string]$QemuIsoName = "hypercore.iso",
    [switch]$QemuAutoFetchLimine,
    [string]$QemuLimineVersion = "latest",
    [string]$QemuLimineCacheDir = "artifacts/limine/cache",
    [string]$QemuLimineOutDir = "artifacts/limine/bin",
    [switch]$QemuAllowBuildLimine,
    [switch]$QemuAllowTimeoutSuccess,
    [switch]$AllowBaselineBootstrap,
    [switch]$RunAbSlotFlip,
    [switch]$AbSlotFlipDryRun,
    [ValidateSet("debug","release")]
    [string]$AbSlotFlipProfile = "release",
    [string]$AbSlotFlipTarget = "x86_64-unknown-none",
    [ValidateSet("A","B","")]
    [string]$AbSlotFlipForceSlot = "",
    [string]$AbRoot = "artifacts/boot_ab",
    [string]$AbStateJson = "artifacts/boot_ab/state.json",
    [string]$OutDir = "reports/release_candidate",
    [ValidateSet("full","smoke")]
    [string]$Profile = "full"
)

$ErrorActionPreference = "Stop"

function Step($name) {
    Write-Host ""
    Write-Host "==> $name" -ForegroundColor Cyan
}

Step "Release candidate orchestration"
$nightlySummaryDir = Join-Path $OutDir "p0_p1_nightly"
$cmd = @(
    "-ExecutionPolicy", "Bypass",
    "-File", ".\scripts\p0_p1_nightly.ps1",
    "-Profile", "$Profile",
    "-SummaryOutDir", "$nightlySummaryDir",
    "-AbRoot", "$AbRoot",
    "-AbStateJson", "$AbStateJson",
    "-RequireAbHealthyAfterP1"
)
if ($SkipHostTests) { $cmd += "-SkipHostTests" }
if ($QemuDryRun) { $cmd += "-QemuDryRun" }
if (-not [string]::IsNullOrWhiteSpace($QemuBootMode)) {
    $cmd += "-QemuBootMode"
    $cmd += "$QemuBootMode"
}
if (-not [string]::IsNullOrWhiteSpace($QemuIsoPath)) {
    $cmd += "-QemuIsoPath"
    $cmd += "$QemuIsoPath"
}
if ($QemuBuildIso) {
    $cmd += "-QemuBuildIso"
    if (-not [string]::IsNullOrWhiteSpace($QemuLimineBinDir)) {
        $cmd += "-QemuLimineBinDir"
        $cmd += "$QemuLimineBinDir"
    }
    if (-not [string]::IsNullOrWhiteSpace($QemuIsoName)) {
        $cmd += "-QemuIsoName"
        $cmd += "$QemuIsoName"
    }
}
if ($QemuAutoFetchLimine) {
    $cmd += "-QemuAutoFetchLimine"
    $cmd += "-QemuLimineVersion"
    $cmd += "$QemuLimineVersion"
    $cmd += "-QemuLimineCacheDir"
    $cmd += "$QemuLimineCacheDir"
    $cmd += "-QemuLimineOutDir"
    $cmd += "$QemuLimineOutDir"
    if ($QemuAllowBuildLimine) {
        $cmd += "-QemuAllowBuildLimine"
    }
}
if ($QemuAllowTimeoutSuccess) { $cmd += "-QemuAllowTimeoutSuccess" }
if ($AllowBaselineBootstrap) { $cmd += "-AllowBaselineBootstrap" }
if ($RunAbSlotFlip) { $cmd += "-RunAbSlotFlip" }
if ($AbSlotFlipDryRun) { $cmd += "-AbSlotFlipDryRun" }
if (-not [string]::IsNullOrWhiteSpace($AbSlotFlipForceSlot)) {
    $cmd += "-AbSlotFlipForceSlot"
    $cmd += "$AbSlotFlipForceSlot"
}
$cmd += "-AbSlotFlipProfile"
$cmd += "$AbSlotFlipProfile"
$cmd += "-AbSlotFlipTarget"
$cmd += "$AbSlotFlipTarget"

& powershell @cmd
if ($LASTEXITCODE -ne 0) {
    throw "p0_p1_nightly failed"
}

Step "Release candidate verdict"
$summaryPath = Join-Path $nightlySummaryDir "summary.json"
if (-not (Test-Path $summaryPath)) {
    throw "missing summary: $summaryPath"
}
$summary = Get-Content $summaryPath -Raw | ConvertFrom-Json
$verdict = $summary.release_ready_verdict
if ($null -eq $verdict) {
    throw "missing release_ready_verdict in $summaryPath"
}

$ready = [bool]$verdict.ready
$reasons = @()
if ($null -ne $verdict.reasons) {
    $reasons = @($verdict.reasons)
}

$report = [ordered]@{
    ready = $ready
    reasons = $reasons
    summary_path = $summaryPath
    generated_utc = (Get-Date).ToUniversalTime().ToString("o")
}

New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
$reportPath = Join-Path $OutDir "verdict.json"
$report | ConvertTo-Json -Depth 6 | Set-Content -Path $reportPath -Encoding utf8

$md = @(
    "# Release Candidate Verdict",
    "",
    "- ready: $ready",
    "- summary: $summaryPath",
    ""
)
if ($reasons.Count -gt 0) {
    $md += "## Reasons"
    $md += ""
    foreach ($reason in $reasons) {
        $md += "- $reason"
    }
}
Set-Content -Path (Join-Path $OutDir "verdict.md") -Value ($md -join "`n") -Encoding utf8

Write-Host "verdict=$reportPath"
if (-not $ready) {
    throw "release candidate verdict: NOT_READY"
}

Write-Host "release candidate verdict: READY" -ForegroundColor Green

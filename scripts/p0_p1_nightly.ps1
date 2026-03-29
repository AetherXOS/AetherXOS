param(
    [ValidateSet("full", "smoke")]
    [string]$Profile = "full",
    [switch]$SkipHostTests,
    [switch]$QemuDryRun,
    [switch]$AllowBaselineBootstrap,
    [switch]$SkipP0,
    [switch]$SkipP1,
    [switch]$SkipP2Gap,
    [switch]$SkipArchive,
    [switch]$RunAbSlotFlip,
    [switch]$AbSlotFlipDryRun,
    [ValidateSet("debug","release")]
    [string]$AbSlotFlipProfile = "release",
    [string]$AbSlotFlipTarget = "x86_64-unknown-none",
    [ValidateSet("A","B","")]
    [string]$AbSlotFlipForceSlot = "",
    [string]$AbSlotFlipOutDir = "reports/ab_slot_flip",
    [string]$AbRoot = "artifacts/boot_ab",
    [string]$AbStateJson = "artifacts/boot_ab/state.json",
    [switch]$RequireAbHealthyAfterP1,
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
    [string]$SummaryOutDir = "reports/p0_p1_nightly",
    [string]$P1OutDir = "reports/p1_nightly",
    [string]$ArchiveRoot = "artifacts/nightly_runs"
)

$ErrorActionPreference = "Stop"

function Step($name) {
    Write-Host ""
    Write-Host "==> $name" -ForegroundColor Cyan
}

$runSummary = [ordered]@{
    ok = $true
    profile = $Profile
    qemu_dry_run = [bool]$QemuDryRun
    allow_baseline_bootstrap = [bool]$AllowBaselineBootstrap
    run_ab_slot_flip = [bool]$RunAbSlotFlip
    require_ab_healthy_after_p1 = [bool]$RequireAbHealthyAfterP1
    steps = @()
    failures = @()
}

function Add-StepResult($name, $ok, $details = $null) {
    $entry = [ordered]@{
        name = $name
        ok = [bool]$ok
    }
    if ($null -ne $details) {
        $entry.details = $details
    }
    $runSummary.steps += $entry
    if (-not $ok) {
        $runSummary.ok = $false
    }
}

function Add-Failure($message) {
    $runSummary.failures += $message
    $runSummary.ok = $false
}

function Resolve-RepoPath([string]$repoRoot, [string]$pathValue) {
    if ([System.IO.Path]::IsPathRooted($pathValue)) {
        return $pathValue
    }
    return (Join-Path $repoRoot $pathValue)
}

function Load-JsonIfExists([string]$pathValue) {
    if (-not (Test-Path $pathValue)) {
        return $null
    }
    try {
        return Get-Content $pathValue -Raw | ConvertFrom-Json
    } catch {
        return $null
    }
}

$p0SoakRounds = 12
$p0RoundTimeoutSec = 75
$p1SoakRounds = 80
$p1QemuRounds = 24

if ($Profile -eq "smoke") {
    $p0SoakRounds = 2
    $p0RoundTimeoutSec = 30
    $p1SoakRounds = 12
    $p1QemuRounds = 6
}

Step "P0+P1 nightly profile"
Write-Host "Profile=$Profile, P0 rounds=$p0SoakRounds, P1 rounds=$p1SoakRounds/$p1QemuRounds"

$resolvedAbStateJson = $AbStateJson
if ($AbStateJson -eq "artifacts/boot_ab/state.json" -and $AbRoot -ne "artifacts/boot_ab") {
    $resolvedAbStateJson = (Join-Path $AbRoot "state.json")
}

if ($RunAbSlotFlip) {
    Step "A/B nightly slot flip"
    $flipCmd = @(
        "scripts/ab_nightly_slot_flip.py",
        "--ab-root", "$AbRoot",
        "--profile", "$AbSlotFlipProfile",
        "--target", "$AbSlotFlipTarget",
        "--out-dir", "$AbSlotFlipOutDir"
    )
    if (-not [string]::IsNullOrWhiteSpace($AbSlotFlipForceSlot)) {
        $flipCmd += "--force-slot"
        $flipCmd += "$AbSlotFlipForceSlot"
    }
    if ($AbSlotFlipDryRun) {
        $flipCmd += "--dry-run"
    }
    & python @flipCmd
    if ($LASTEXITCODE -ne 0) {
        Add-StepResult "ab_nightly_slot_flip" $false
        Add-Failure "ab_nightly_slot_flip failed"
        throw "ab_nightly_slot_flip failed"
    }
    Add-StepResult "ab_nightly_slot_flip" $true
}

if (-not $SkipP0) {
    if ($QemuDryRun) {
        Step "P0 release acceptance"
        Write-Host "Skipped (QemuDryRun is enabled; P0 requires real soak artifacts)." -ForegroundColor Yellow
    } else {
        Step "P0 release acceptance"
        $p0Cmd = @(
            "-ExecutionPolicy", "Bypass",
            "-File", ".\scripts\p0_release_acceptance.ps1",
            "-SoakProfile", "release",
            "-SoakRounds", "$p0SoakRounds",
            "-SoakRoundTimeoutSec", "$p0RoundTimeoutSec",
            "-SoakBootMode", "$QemuBootMode"
        )
        if (-not [string]::IsNullOrWhiteSpace($QemuIsoPath)) {
            $p0Cmd += "-SoakIsoPath"
            $p0Cmd += "$QemuIsoPath"
        }
        if ($QemuBuildIso) {
            $p0Cmd += "-SoakBuildIso"
            if (-not [string]::IsNullOrWhiteSpace($QemuLimineBinDir)) {
                $p0Cmd += "-SoakLimineBinDir"
                $p0Cmd += "$QemuLimineBinDir"
            }
            if (-not [string]::IsNullOrWhiteSpace($QemuIsoName)) {
                $p0Cmd += "-SoakIsoName"
                $p0Cmd += "$QemuIsoName"
            }
        }
        if ($QemuAutoFetchLimine) {
            $p0Cmd += "-SoakAutoFetchLimine"
            $p0Cmd += "-SoakLimineVersion"
            $p0Cmd += "$QemuLimineVersion"
            $p0Cmd += "-SoakLimineCacheDir"
            $p0Cmd += "$QemuLimineCacheDir"
            $p0Cmd += "-SoakLimineOutDir"
            $p0Cmd += "$QemuLimineOutDir"
            if ($QemuAllowBuildLimine) {
                $p0Cmd += "-SoakAllowBuildLimine"
            }
        }
        if ($QemuAllowTimeoutSuccess) {
            $p0Cmd += "-SoakAllowTimeoutSuccess"
        }
        if ($SkipHostTests) {
            $p0Cmd += "-SkipHostTests"
        }
        & powershell @p0Cmd
        if ($LASTEXITCODE -ne 0) {
            Add-StepResult "p0_release_acceptance" $false
            Add-Failure "p0_release_acceptance failed"
            throw "p0_release_acceptance failed"
        }
        Add-StepResult "p0_release_acceptance" $true
    }
} else {
    Add-StepResult "p0_release_acceptance" $true @{ skipped = $true }
}

if (-not $SkipP1) {
    Step "P1 nightly"
    $p1Cmd = @(
        "-ExecutionPolicy", "Bypass",
        "-File", ".\scripts\p1_nightly.ps1",
        "-AbStateJson", "$resolvedAbStateJson",
        "-OutDir", "$P1OutDir",
        "-SoakRounds", "$p1SoakRounds",
        "-QemuRounds", "$p1QemuRounds",
        "-QemuBootMode", "$QemuBootMode"
    )
    if (-not [string]::IsNullOrWhiteSpace($QemuIsoPath)) {
        $p1Cmd += "-QemuIsoPath"
        $p1Cmd += "$QemuIsoPath"
    }
    if ($QemuBuildIso) {
        $p1Cmd += "-QemuBuildIso"
        if (-not [string]::IsNullOrWhiteSpace($QemuLimineBinDir)) {
            $p1Cmd += "-QemuLimineBinDir"
            $p1Cmd += "$QemuLimineBinDir"
        }
        if (-not [string]::IsNullOrWhiteSpace($QemuIsoName)) {
            $p1Cmd += "-QemuIsoName"
            $p1Cmd += "$QemuIsoName"
        }
    }
    if ($QemuAutoFetchLimine) {
        $p1Cmd += "-QemuAutoFetchLimine"
        $p1Cmd += "-QemuLimineVersion"
        $p1Cmd += "$QemuLimineVersion"
        $p1Cmd += "-QemuLimineCacheDir"
        $p1Cmd += "$QemuLimineCacheDir"
        $p1Cmd += "-QemuLimineOutDir"
        $p1Cmd += "$QemuLimineOutDir"
        if ($QemuAllowBuildLimine) {
            $p1Cmd += "-QemuAllowBuildLimine"
        }
    }
    if ($QemuAllowTimeoutSuccess) {
        $p1Cmd += "-QemuAllowTimeoutSuccess"
    }
    if ($SkipHostTests) {
        $p1Cmd += "-SkipHostTests"
    }
    if ($QemuDryRun) {
        $p1Cmd += "-QemuDryRun"
    }
    if ($AllowBaselineBootstrap) {
        $p1Cmd += "-AllowBaselineBootstrap"
    }

    & powershell @p1Cmd
    if ($LASTEXITCODE -ne 0) {
        Add-StepResult "p1_nightly" $false
        Add-Failure "p1_nightly failed"
        throw "p1_nightly failed"
    }
    Add-StepResult "p1_nightly" $true @{ out_dir = $P1OutDir }

    if ($RequireAbHealthyAfterP1 -and -not $QemuDryRun) {
        Step "A/B state post-check"
        if (-not (Test-Path $resolvedAbStateJson)) {
            Add-StepResult "ab_state_post_check" $false @{ state_path = $resolvedAbStateJson; reason = "missing state file" }
            Add-Failure "ab state file missing after P1"
            throw "ab state file missing after P1: $resolvedAbStateJson"
        }
        $abState = Get-Content $resolvedAbStateJson -Raw | ConvertFrom-Json
        $pending = $abState.pending_slot
        $status = $abState.status
        $active = $abState.active_slot
        $isHealthy = ($null -eq $pending) -and (($status -eq "healthy") -or ($status -eq "rolled_back"))
        Add-StepResult "ab_state_post_check" $isHealthy @{ pending_slot = $pending; status = $status; active_slot = $active; state_path = $resolvedAbStateJson }
        if (-not $isHealthy) {
            Add-Failure "ab state not healthy after P1 (pending_slot/status check failed)"
            throw "ab state not healthy after P1 (pending_slot/status check failed)"
        }
    }
} else {
    Add-StepResult "p1_nightly" $true @{ skipped = $true }
}

if (-not $SkipP2Gap) {
    Step "P2 gap gate"
    $p2Cmd = @("scripts/p2_gap_gate.py")
    if ($AllowBaselineBootstrap) {
        $p2Cmd += "--update-baseline-on-success"
    } else {
        $p2Cmd += "--auto-baseline"
        $p2Cmd += "--update-baseline-on-success"
    }
    & python @p2Cmd
    if ($LASTEXITCODE -ne 0) {
        Add-StepResult "p2_gap_gate" $false
        Add-Failure "p2_gap_gate failed"
        throw "p2_gap_gate failed"
    }
    Add-StepResult "p2_gap_gate" $true
} else {
    Add-StepResult "p2_gap_gate" $true @{ skipped = $true }
}

if (-not $SkipArchive) {
    Step "Archive nightly artifacts"
    $archiveCmd = @(
        "-ExecutionPolicy", "Bypass",
        "-File", ".\scripts\archive_nightly_artifacts.ps1",
        "-ArchiveRoot", "$ArchiveRoot"
    )
    & powershell @archiveCmd
    if ($LASTEXITCODE -ne 0) {
        Add-StepResult "archive_nightly_artifacts" $false
        Add-Failure "archive_nightly_artifacts failed"
        throw "archive_nightly_artifacts failed"
    }
    Add-StepResult "archive_nightly_artifacts" $true @{ archive_root = $ArchiveRoot }
} else {
    Add-StepResult "archive_nightly_artifacts" $true @{ skipped = $true }
}

Step "Nightly summary"
$runSummary.summary_created_utc = (Get-Date).ToUniversalTime().ToString("o")
$runSummary.ab_state_json = $resolvedAbStateJson
$summaryDir = $SummaryOutDir
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
if (-not [System.IO.Path]::IsPathRooted($summaryDir)) {
    $summaryDir = Join-Path $repoRoot $summaryDir
}

$p1SummaryPath = Resolve-RepoPath $repoRoot (Join-Path $P1OutDir "summary.json")
$p2SummaryPath = Resolve-RepoPath $repoRoot "reports/p2_gap/gate_summary.json"
$abFlipSummaryPath = Resolve-RepoPath $repoRoot (Join-Path $AbSlotFlipOutDir "summary.json")
$abRecoverySummaryPath = Resolve-RepoPath $repoRoot "reports/ab_boot_recovery_gate/summary.json"

$p1SummaryPayload = Load-JsonIfExists $p1SummaryPath
$p2SummaryPayload = Load-JsonIfExists $p2SummaryPath
$abFlipSummaryPayload = Load-JsonIfExists $abFlipSummaryPath
$abRecoverySummaryPayload = Load-JsonIfExists $abRecoverySummaryPath

$runSummary.report_paths = [ordered]@{
    p1 = $p1SummaryPath
    p2 = $p2SummaryPath
    ab_slot_flip = $abFlipSummaryPath
    ab_recovery = $abRecoverySummaryPath
}

$verdictReasons = @()
$releaseReady = $true

if (-not $runSummary.ok) {
    $releaseReady = $false
    $verdictReasons += "combined nightly steps include failures"
}

if ($QemuDryRun) {
    $releaseReady = $false
    $verdictReasons += "qemu dry-run enabled"
}

$skippedSteps = @(
    $runSummary.steps | Where-Object {
        $null -ne $_.details -and
        $null -ne $_.details.skipped -and
        [bool]$_.details.skipped
    }
)
if ($skippedSteps.Count -gt 0) {
    $releaseReady = $false
    $skippedNames = ($skippedSteps | ForEach-Object { $_.name }) -join ","
    $verdictReasons += "required steps skipped: $skippedNames"
}

if ($null -eq $p1SummaryPayload -or $null -eq $p1SummaryPayload.summary) {
    $releaseReady = $false
    $verdictReasons += "missing p1 summary"
} elseif (-not [bool]$p1SummaryPayload.summary.ok) {
    $releaseReady = $false
    $verdictReasons += "p1 summary reports failure"
}

if ($null -eq $p2SummaryPayload -or $null -eq $p2SummaryPayload.summary) {
    $releaseReady = $false
    $verdictReasons += "missing p2 gap gate summary"
} elseif (-not [bool]$p2SummaryPayload.summary.ok) {
    $releaseReady = $false
    $verdictReasons += "p2 gap gate reports failure"
}

if ($RunAbSlotFlip -and $null -eq $abFlipSummaryPayload) {
    $releaseReady = $false
    $verdictReasons += "missing A/B slot flip summary"
}

if (-not $QemuDryRun) {
    if ($null -eq $abRecoverySummaryPayload -or $null -eq $abRecoverySummaryPayload.summary) {
        $releaseReady = $false
        $verdictReasons += "missing A/B recovery summary for non-dry-run"
    } else {
        $abRecovery = $abRecoverySummaryPayload.summary
        if (-not [bool]$abRecovery.ok) {
            $releaseReady = $false
            $verdictReasons += "A/B recovery gate reports failure"
        }
        if ($null -ne $abRecovery.pending_slot) {
            $releaseReady = $false
            $verdictReasons += "A/B pending_slot not cleared after recovery"
        }
    }
}

$runSummary.release_ready_verdict = [ordered]@{
    ready = $releaseReady
    reasons = $verdictReasons
}

New-Item -ItemType Directory -Path $summaryDir -Force | Out-Null
$summaryPath = Join-Path $summaryDir "summary.json"
$runSummary | ConvertTo-Json -Depth 8 | Set-Content -Path $summaryPath -Encoding utf8
Write-Host "summary=$summaryPath"
if ($runSummary.ok) {
    Write-Host "P0+P1 nightly sequence completed." -ForegroundColor Green
} else {
    Write-Host "P0+P1 nightly sequence failed." -ForegroundColor Red
}
if ($releaseReady) {
    Write-Host "release_ready_verdict=READY" -ForegroundColor Green
} else {
    Write-Host "release_ready_verdict=NOT_READY" -ForegroundColor Yellow
}

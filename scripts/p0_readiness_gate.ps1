param(
    [switch]$SkipHostTests,
    [switch]$RequireSoakArtifacts,
    [switch]$RunSoakMatrix,
    [switch]$StrictRecoveryGate,
    [ValidateSet("debug","release")]
    [string]$SoakProfile = "release",
    [int]$SoakRounds = 6,
    [int]$SoakMemoryMb = 512,
    [int]$SoakCores = 2,
    [double]$SoakChaosRate = 0.35,
    [int]$SoakRoundTimeoutSec = 45,
    [ValidateSet("direct","iso")]
    [string]$SoakBootMode = "direct",
    [string]$SoakIsoPath = "",
    [switch]$SoakBuildIso,
    [string]$SoakLimineBinDir = "",
    [string]$SoakIsoName = "hypercore.iso",
    [switch]$SoakAutoFetchLimine,
    [string]$SoakLimineVersion = "latest",
    [string]$SoakLimineCacheDir = "artifacts/limine/cache",
    [string]$SoakLimineOutDir = "artifacts/limine/bin",
    [switch]$SoakAllowBuildLimine,
    [switch]$SoakAllowTimeoutSuccess,
    [int]$RecoveryMinSuccessfulBoots = 3,
    [int]$RecoveryWindow = 3,
    [int]$MaxAbiStub = -1,
    [int]$MaxAbiPartial = -1
)

$ErrorActionPreference = "Stop"

function Step($name) {
    Write-Host ""
    Write-Host "==> $name" -ForegroundColor Cyan
}

Step "Release preflight"
$preflightArgs = @("-ExecutionPolicy", "Bypass", "-File", ".\scripts\release_preflight.ps1")
if ($SkipHostTests) {
    $preflightArgs += "-SkipHostTests"
}
& powershell @preflightArgs
if ($LASTEXITCODE -ne 0) { throw "release_preflight failed" }

Step "Linux ABI errno conformance"
& python ".\scripts\linux_abi_errno_conformance.py" --out-dir "reports/errno_conformance"
if ($LASTEXITCODE -ne 0) { throw "linux_abi_errno_conformance failed" }

Step "Linux shim errno conformance"
& python ".\scripts\linux_shim_errno_conformance.py" --out-dir "reports/linux_shim_errno_conformance"
if ($LASTEXITCODE -ne 0) { throw "linux_shim_errno_conformance failed" }

Step "Linux ABI gap inventory"
$abiGapArgs = @(
    ".\scripts\linux_abi_gap_inventory.py",
    "--out-dir", "reports/abi_gap_inventory"
)
if ($MaxAbiStub -ge 0) {
    $abiGapArgs += "--max-stub"
    $abiGapArgs += "$MaxAbiStub"
}
if ($MaxAbiPartial -ge 0) {
    $abiGapArgs += "--max-partial"
    $abiGapArgs += "$MaxAbiPartial"
}
& python @abiGapArgs
if ($LASTEXITCODE -ne 0) { throw "linux_abi_gap_inventory failed" }

Step "Linux syscall coverage"
& python ".\scripts\syscall_coverage_report.py" `
    --linux-compat-enabled `
    --format md `
    --out "reports/syscall_coverage/summary.md" `
    --summary-out "reports/syscall_coverage/summary.json"
if ($LASTEXITCODE -ne 0) { throw "syscall_coverage_report failed" }

Step "Linux ABI readiness score"
& python ".\scripts\linux_abi_readiness_score.py" --out-dir "reports/abi_readiness"
if ($LASTEXITCODE -ne 0) { throw "linux_abi_readiness_score failed" }

$soakSummary = ".\artifacts\qemu_soak\summary.json"
$hasUsableSoakRounds = $false
if (Test-Path $soakSummary) {
    try {
        $soakPayload = Get-Content $soakSummary -Raw | ConvertFrom-Json
        $roundCount = 0
        if ($null -ne $soakPayload.rounds) {
            $roundCount = @($soakPayload.rounds).Count
        }
        $hasUsableSoakRounds = ($roundCount -gt 0)
    } catch {
        $hasUsableSoakRounds = $false
    }
}
if ($RunSoakMatrix) {
    Step "QEMU soak matrix"
    $qemuCmd = @(
        ".\scripts\qemu_soak_matrix.py",
        "--profile", "$SoakProfile",
        "--boot-mode", "$SoakBootMode",
        "--rounds", "$SoakRounds",
        "--memory-mb", "$SoakMemoryMb",
        "--cores", "$SoakCores",
        "--chaos-rate", "$SoakChaosRate",
        "--round-timeout-sec", "$SoakRoundTimeoutSec",
        "--out-dir", "artifacts/qemu_soak"
    )
    if (-not [string]::IsNullOrWhiteSpace($SoakIsoPath)) {
        $qemuCmd += "--iso-path"
        $qemuCmd += "$SoakIsoPath"
    }
    if ($SoakBuildIso) {
        $qemuCmd += "--build-iso"
        if (-not [string]::IsNullOrWhiteSpace($SoakLimineBinDir)) {
            $qemuCmd += "--limine-bin-dir"
            $qemuCmd += "$SoakLimineBinDir"
        }
        if ($SoakAutoFetchLimine) {
            $qemuCmd += "--auto-fetch-limine"
            $qemuCmd += "--limine-version"
            $qemuCmd += "$SoakLimineVersion"
            if (-not [string]::IsNullOrWhiteSpace($SoakLimineCacheDir)) {
                $qemuCmd += "--limine-cache-dir"
                $qemuCmd += "$SoakLimineCacheDir"
            }
            if (-not [string]::IsNullOrWhiteSpace($SoakLimineOutDir)) {
                $qemuCmd += "--limine-out-dir"
                $qemuCmd += "$SoakLimineOutDir"
            }
            if ($SoakAllowBuildLimine) {
                $qemuCmd += "--allow-build-limine"
            }
        }
        if (-not [string]::IsNullOrWhiteSpace($SoakIsoName)) {
            $qemuCmd += "--iso-name"
            $qemuCmd += "$SoakIsoName"
        }
    }
    if ($SoakAllowTimeoutSuccess) {
        $qemuCmd += "--allow-timeout-success"
    }
    & python @qemuCmd
    if ($LASTEXITCODE -ne 0) { throw "qemu_soak_matrix failed" }
}

if ((Test-Path $soakSummary) -and $hasUsableSoakRounds) {
    Step "Reboot recovery gate"
    $minSuccessful = $RecoveryMinSuccessfulBoots
    $recoveryWindow = $RecoveryWindow
    if ($StrictRecoveryGate) {
        $minSuccessful = [Math]::Max($minSuccessful, 8)
        $recoveryWindow = [Math]::Max($recoveryWindow, 5)
    }
    & python ".\scripts\reboot_recovery_gate.py" `
        --soak-summary "artifacts/qemu_soak/summary.json" `
        --min-successful-boots $minSuccessful `
        --recovery-window $recoveryWindow `
        --out-dir "reports/reboot_recovery_gate"
    if ($LASTEXITCODE -ne 0) { throw "reboot_recovery_gate failed" }
} elseif ((Test-Path $soakSummary) -and -not $hasUsableSoakRounds -and -not $RequireSoakArtifacts) {
    Step "Reboot recovery gate"
    Write-Host "Skipped (soak summary has no rounds). Run scripts/qemu_soak_matrix.py without --dry-run, or pass -RequireSoakArtifacts." -ForegroundColor Yellow
} elseif ($RequireSoakArtifacts) {
    throw "missing soak artifacts: $soakSummary"
} else {
    Step "Reboot recovery gate"
    Write-Host "Skipped (missing $soakSummary). Run scripts/qemu_soak_matrix.py first or pass -RequireSoakArtifacts." -ForegroundColor Yellow
}

Step "P0 readiness summary"
Write-Host "P0 readiness gate completed." -ForegroundColor Green

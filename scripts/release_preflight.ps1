param(
    [switch]$SkipHostTests,
    [switch]$SkipBootArtifacts
)

$ErrorActionPreference = "Stop"
$SyscallGateMinImplementedPct = 100
$SyscallGateMaxNoDefault = 0
$SyscallGateMaxNoLinuxCompat = 0
$SyscallGateMaxPartialDefault = 0
$SyscallGateMaxPartialLinuxCompat = 0
$SyscallGateMaxExternal = 0

function Step($name) {
    Write-Host ""
    Write-Host "==> $name" -ForegroundColor Cyan
}

function Run-Cargo {
    param([string[]]$CargoArgs)
    Write-Host "cargo $($CargoArgs -join ' ')" -ForegroundColor DarkGray
    & cargo @CargoArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed: cargo $($CargoArgs -join ' ')"
    }
}

function Run-SyscallCoverageGate {
    param(
        [string]$ReportMd,
        [string]$SummaryJson,
        [int]$MaxNo,
        [int]$MaxPartial,
        [switch]$LinuxCompatEnabled
    )
    $args = @(
        ".\scripts\syscall_coverage_report.py",
        "--format", "md",
        "--out", $ReportMd,
        "--summary-out", $SummaryJson,
        "--min-implemented-pct", "$SyscallGateMinImplementedPct",
        "--max-no", "$MaxNo",
        "--max-partial", "$MaxPartial",
        "--max-external", "$SyscallGateMaxExternal"
    )
    if ($LinuxCompatEnabled) {
        $args = @(".\scripts\syscall_coverage_report.py", "--linux-compat-enabled") + $args[1..($args.Length - 1)]
    }
    & python @args
    if ($LASTEXITCODE -ne 0) { throw "Syscall coverage gate failed" }
}

Step "Rust toolchain info"
& rustc -vV
if ($LASTEXITCODE -ne 0) { throw "rustc not available" }
& cargo -V
if ($LASTEXITCODE -ne 0) { throw "cargo not available" }

Step "Clean check (all targets)"
Run-Cargo -CargoArgs @("check", "--all-targets")

Step "Release build"
Run-Cargo -CargoArgs @("build", "--release")

if (-not $SkipBootArtifacts) {
    Step "Boot artifact build"
    & python .\scripts\build_boot_image.py --profile release
    if ($LASTEXITCODE -ne 0) { throw "Boot artifact build failed" }
}

if (-not $SkipHostTests) {
    Step "Host tests"
    & powershell -ExecutionPolicy Bypass -File ".\scripts\run_tests_host.ps1"
    if ($LASTEXITCODE -ne 0) { throw "Host tests failed" }
}

Step "Linux syscall coverage gate"
Run-SyscallCoverageGate `
    -ReportMd "reports/syscall_coverage.md" `
    -SummaryJson "reports/syscall_coverage_summary.json" `
    -MaxNo $SyscallGateMaxNoDefault `
    -MaxPartial $SyscallGateMaxPartialDefault

Step "linux_compat profile compile + syscall gate"
Run-Cargo -CargoArgs @("check", "--features", "linux_compat,posix_deep_tests")
Run-SyscallCoverageGate `
    -ReportMd "reports/syscall_coverage_linux_compat.md" `
    -SummaryJson "reports/syscall_coverage_linux_compat_summary.json" `
    -MaxNo $SyscallGateMaxNoLinuxCompat `
    -MaxPartial $SyscallGateMaxPartialLinuxCompat `
    -LinuxCompatEnabled

Step "POSIX deep tests compile gate"
Run-Cargo -CargoArgs @("test", "--no-run", "--features", "posix_deep_tests")

Step "Preflight summary"
Write-Host "Release preflight completed successfully." -ForegroundColor Green
Write-Host "Next: boot under emulator with scripts/setup/setup_emulator.ps1 and verify runtime telemetry." -ForegroundColor Green

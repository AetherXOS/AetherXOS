param(
    [string]$TestPath = "tests/powershell/DashboardAgent.Contract.Tests.ps1",
    [string]$OutJson = "reports/tooling/dashboard_agent_contract_summary.json"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $TestPath)) {
    throw "Test file not found: $TestPath"
}

$outDir = Split-Path -Parent $OutJson
if ($outDir) {
    New-Item -ItemType Directory -Path $outDir -Force | Out-Null
}

$result = $null
try {
    $result = Invoke-Pester -Path $TestPath -PassThru
} catch {
    $summary = [ordered]@{
        ok = $false
        error = [string]$_.Exception.Message
        test_path = $TestPath
        generated_utc = [DateTime]::UtcNow.ToString("o")
    }
    ($summary | ConvertTo-Json -Depth 8) | Set-Content -Path $OutJson -Encoding UTF8
    Write-Host "[contract] failed to execute: $($summary.error)" -ForegroundColor Red
    exit 1
}

$failed = 0
$passed = 0
$total = 0
$skipped = 0
if ($result) {
    if ($result.PSObject.Properties.Name.Contains("FailedCount")) { $failed = [int]$result.FailedCount }
    if ($result.PSObject.Properties.Name.Contains("PassedCount")) { $passed = [int]$result.PassedCount }
    if ($result.PSObject.Properties.Name.Contains("TotalCount")) { $total = [int]$result.TotalCount }
    if ($result.PSObject.Properties.Name.Contains("SkippedCount")) { $skipped = [int]$result.SkippedCount }
}

if ($total -eq 0 -and ($passed -gt 0 -or $failed -gt 0)) {
    $total = $passed + $failed + $skipped
}

$ok = ($failed -eq 0 -and $total -gt 0)
$summary = [ordered]@{
    ok = $ok
    total = $total
    passed = $passed
    failed = $failed
    skipped = $skipped
    test_path = $TestPath
    generated_utc = [DateTime]::UtcNow.ToString("o")
}
($summary | ConvertTo-Json -Depth 8) | Set-Content -Path $OutJson -Encoding UTF8

if ($ok) {
    Write-Host "[contract] PASS total=$total passed=$passed failed=$failed skipped=$skipped" -ForegroundColor Green
    exit 0
}

Write-Host "[contract] FAIL total=$total passed=$passed failed=$failed skipped=$skipped" -ForegroundColor Red
exit 1

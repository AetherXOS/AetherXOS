Set-StrictMode -Version Latest

function Write-HcStep {
    param(
        [string]$Scope,
        [string]$Message
    )
    if ($env:HYPERCORE_QUIET -eq "1") { return }
    if ($env:HYPERCORE_JSON_OUTPUT -eq "1") {
        $payload = [ordered]@{
            ts_utc = [DateTime]::UtcNow.ToString("o")
            scope = $Scope
            message = $Message
        }
        Write-Host ($payload | ConvertTo-Json -Compress)
        return
    }
    Write-Host "[$Scope] $Message"
}

function Invoke-HcChecked {
    param(
        [string]$FilePath,
        [string[]]$Arguments,
        [string]$ErrorPrefix = "command failed"
    )
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$ErrorPrefix ($LASTEXITCODE): $FilePath $($Arguments -join ' ')"
    }
}

function Invoke-HcWithRetry {
    param(
        [scriptblock]$Action,
        [int]$RetryCount = 2,
        [int]$DelaySeconds = 2
    )
    $attempt = 0
    while ($true) {
        try {
            & $Action
            return
        } catch {
            if ($attempt -ge $RetryCount) { throw }
            Start-Sleep -Seconds $DelaySeconds
            $attempt += 1
        }
    }
}

function Get-HcRepoRoot {
    param([string]$ScriptRoot)
    return (Resolve-Path (Join-Path $ScriptRoot "..")).Path
}

function Get-HcJsonFile {
    param([string]$Path)
    if (-not (Test-Path $Path)) {
        throw "json file not found: $Path"
    }
    return (Get-Content -Raw -Path $Path | ConvertFrom-Json)
}

function Save-HcJsonFile {
    param(
        [Parameter(Mandatory=$true)]$Object,
        [Parameter(Mandatory=$true)][string]$Path
    )
    $dir = Split-Path -Parent $Path
    if ($dir -and -not (Test-Path $dir)) {
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
    }
    $Object | ConvertTo-Json -Depth 12 | Set-Content -Path $Path -Encoding UTF8
}

function Test-HcCommand {
    param([string]$Name)
    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

function Resolve-HcPython {
    if (Test-HcCommand "python") { return "python" }
    if (Test-HcCommand "py") { return "py" }
    throw "python interpreter not found (python/py)"
}

Export-ModuleMember -Function `
    Write-HcStep, `
    Invoke-HcChecked, `
    Invoke-HcWithRetry, `
    Get-HcRepoRoot, `
    Get-HcJsonFile, `
    Save-HcJsonFile, `
    Test-HcCommand, `
    Resolve-HcPython

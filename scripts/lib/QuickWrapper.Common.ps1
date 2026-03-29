function Resolve-HypercoreScriptsRoot {
    param([Parameter(Mandatory = $true)][string]$ScriptRoot)

    if (Test-Path (Join-Path $ScriptRoot "quick.ps1")) {
        return $ScriptRoot
    }

    $parent = Split-Path -Parent $ScriptRoot
    if ($parent -and (Test-Path (Join-Path $parent "quick.ps1"))) {
        return $parent
    }

    throw "Unable to locate scripts root from: $ScriptRoot"
}

function Resolve-HypercoreWrapperShell {
    if (Get-Command "pwsh" -ErrorAction SilentlyContinue) {
        return "pwsh"
    }
    return "powershell"
}

function Invoke-QuickActionWrapper {
    param(
        [Parameter(Mandatory = $true)][string]$ScriptRoot,
        [Parameter(Mandatory = $true)][string]$Action,
        [ValidateSet("quick", "strict")][string]$Profile = "quick",
        [switch]$NoLock,
        [switch]$SkipConfirm,
        [string[]]$ExtraArgs = @()
    )

    $shell = Resolve-HypercoreWrapperShell
    $scriptsRoot = Resolve-HypercoreScriptsRoot -ScriptRoot $ScriptRoot
    $scriptPath = Join-Path $scriptsRoot "quick.ps1"
    $args = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $scriptPath, "-Action", $Action, "-Profile", $Profile)
    if ($NoLock) { $args += "-NoLock" }
    if ($SkipConfirm) { $args += "-SkipConfirm" }
    if ($ExtraArgs.Count -gt 0) { $args += $ExtraArgs }

    & $shell @args
    exit $LASTEXITCODE
}

function Invoke-HypercoreCommandWrapper {
    param(
        [Parameter(Mandatory = $true)][string]$ScriptRoot,
        [Parameter(Mandatory = $true)][string]$Command,
        [ValidateSet("quick", "strict")][string]$Profile = "quick",
        [string[]]$ExtraArgs = @()
    )

    $shell = Resolve-HypercoreWrapperShell
    $scriptsRoot = Resolve-HypercoreScriptsRoot -ScriptRoot $ScriptRoot
    $scriptPath = Join-Path $scriptsRoot "hypercore.ps1"
    $args = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $scriptPath, "-Command", $Command, "-Profile", $Profile)
    if ($ExtraArgs.Count -gt 0) { $args += $ExtraArgs }

    & $shell @args
    exit $LASTEXITCODE
}

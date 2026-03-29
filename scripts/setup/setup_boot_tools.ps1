<#
.SYNOPSIS
Installs and validates host + boot tooling on Windows.

.DESCRIPTION
- Installs Python, Git, QEMU, MSYS2 (winget fallback: choco)
- Installs Node.js/NPM (winget fallback: choco)
- Installs Deno runtime (winget fallback: choco)
- Installs xorriso and build helper packages via MSYS2 pacman
- Optionally appends MSYS2 and Cargo bins to user PATH
#>

param(
    [switch]$InstallPython = $true,
    [switch]$InstallGit = $true,
    [switch]$InstallQemu = $true,
    [switch]$InstallNode = $true,
    [switch]$InstallDeno = $true,
    [int]$MinNodeMajor = 18,
    [switch]$InstallMsys2 = $true,
    [switch]$InstallXorriso = $true,
    [switch]$InstallMsys2Deps = $true,
    [switch]$AddMsysToUserPath = $true,
    [switch]$AddCargoToUserPath = $true,
    [switch]$Offline
)

$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)
    Write-Host "[setup_boot_tools] $Message"
}

function Invoke-Checked {
    param(
        [string]$FilePath,
        [string[]]$Arguments
    )
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "command failed ($LASTEXITCODE): $FilePath $($Arguments -join ' ')"
    }
}

function Refresh-ProcessPath {
    $machinePath = [Environment]::GetEnvironmentVariable("Path", "Machine")
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $env:Path = "$machinePath;$userPath"
}

function Install-WithWingetOrChoco {
    param(
        [string]$WingetId,
        [string]$ChocoId
    )
    if ($Offline) {
        throw "offline mode enabled; cannot install package: winget=$WingetId choco=$ChocoId"
    }
    $installed = $false
    if ($WingetId -and (Get-Command winget -ErrorAction SilentlyContinue)) {
        Write-Step "Installing $WingetId via winget"
        & winget install --id $WingetId --exact --accept-package-agreements --accept-source-agreements
        if ($LASTEXITCODE -eq 0) { $installed = $true }
    }

    if (-not $installed -and $ChocoId -and (Get-Command choco -ErrorAction SilentlyContinue)) {
        Write-Step "Installing $ChocoId via choco"
        & choco install $ChocoId -y
        if ($LASTEXITCODE -eq 0) { $installed = $true }
    }

    if (-not $installed) {
        throw "Failed to install package (winget/choco): winget=$WingetId choco=$ChocoId"
    }
}

function Invoke-WithRetry {
    param(
        [scriptblock]$Action,
        [int]$Attempts = 2,
        [int]$DelaySec = 2
    )
    for ($i = 1; $i -le $Attempts; $i++) {
        try {
            & $Action
            return
        } catch {
            if ($i -ge $Attempts) { throw }
            Write-Step "retry $i/$Attempts failed: $($_.Exception.Message)"
            Start-Sleep -Seconds $DelaySec
        }
    }
}

function Test-Python {
    if (Get-Command python -ErrorAction SilentlyContinue) { return $true }
    if (Get-Command py -ErrorAction SilentlyContinue) { return $true }
    return $false
}

function Test-Git {
    return [bool](Get-Command git -ErrorAction SilentlyContinue)
}

function Test-Qemu {
    if (Get-Command qemu-system-x86_64 -ErrorAction SilentlyContinue) { return $true }
    $candidate = Join-Path ${env:ProgramFiles} "qemu\qemu-system-x86_64.exe"
    return (Test-Path $candidate)
}

function Test-Xorriso {
    if (Get-Command xorriso -ErrorAction SilentlyContinue) { return $true }
    return (Test-Path "C:\msys64\usr\bin\xorriso.exe")
}

function Test-Npm {
    if (Get-Command npm -ErrorAction SilentlyContinue) { return $true }
    return $false
}

function Test-Node {
    if (Get-Command node -ErrorAction SilentlyContinue) { return $true }
    return $false
}

function Get-NodeMajorVersion {
    if (-not (Test-Node)) { return -1 }
    try {
        $raw = [string](& node --version 2>$null)
        if (-not $raw) { return -1 }
        $v = $raw.Trim().TrimStart("v")
        $major = ($v.Split(".")[0] -as [int])
        if ($null -eq $major) { return -1 }
        return $major
    } catch {
        return -1
    }
}

function Test-Deno {
    return [bool](Get-Command deno -ErrorAction SilentlyContinue)
}

function Ensure-Python {
    if (Test-Python) {
        Write-Step "Python already available"
        return
    }
    Install-WithWingetOrChoco -WingetId "Python.Python.3.12" -ChocoId "python"
    Refresh-ProcessPath
    if (-not (Test-Python)) { throw "Python installation validation failed" }
}

function Ensure-Git {
    if (Test-Git) {
        Write-Step "Git already available"
        return
    }
    Install-WithWingetOrChoco -WingetId "Git.Git" -ChocoId "git"
    Refresh-ProcessPath
    if (-not (Test-Git)) { throw "Git installation validation failed" }
}

function Install-Qemu {
    if (Test-Qemu) {
        Write-Step "QEMU already available"
        return
    }

    Install-WithWingetOrChoco -WingetId "SoftwareFreedomConservancy.QEMU" -ChocoId "qemu"
    Refresh-ProcessPath
    if (-not (Test-Qemu)) { throw "QEMU installation failed. Install manually and retry." }
}

function Ensure-Node {
    $major = Get-NodeMajorVersion
    if ((Test-Npm) -and $major -ge $MinNodeMajor) {
        Write-Step "Node.js/NPM already available (node major=$major)"
        return
    }
    if ($major -gt 0 -and $major -lt $MinNodeMajor) {
        Write-Step "Node.js exists but too old (major=$major, required>=$MinNodeMajor). Upgrading..."
    }

    $installed = $false
    $options = @(
        @{ winget = "OpenJS.NodeJS.LTS"; choco = "nodejs-lts" },
        @{ winget = "OpenJS.NodeJS"; choco = "nodejs" }
    )
    foreach ($opt in $options) {
        if ($installed) { break }
        try {
            Invoke-WithRetry -Attempts 2 -DelaySec 3 -Action {
                Install-WithWingetOrChoco -WingetId $opt.winget -ChocoId $opt.choco
            }
            Refresh-ProcessPath
            $major = Get-NodeMajorVersion
            if ((Test-Npm) -and $major -ge $MinNodeMajor) {
                $installed = $true
            }
        } catch {
            Write-Step "Node install attempt failed for winget=$($opt.winget) choco=$($opt.choco)"
        }
    }

    $major = Get-NodeMajorVersion
    if (-not (Test-Npm) -or -not (Test-Node) -or $major -lt $MinNodeMajor) {
        throw "Node.js/NPM installation validation failed (required node major>=$MinNodeMajor, current=$major)"
    }
}

function Ensure-Deno {
    if (Test-Deno) {
        Write-Step "Deno already available"
        return
    }

    Install-WithWingetOrChoco -WingetId "DenoLand.Deno" -ChocoId "deno"
    Refresh-ProcessPath
    if (-not (Test-Deno)) {
        throw "Deno installation validation failed"
    }
}

function Ensure-Msys2 {
    if (Test-Path "C:\msys64\usr\bin\bash.exe") { return }
    Install-WithWingetOrChoco -WingetId "MSYS2.MSYS2" -ChocoId "msys2"
    Refresh-ProcessPath
    if (-not (Test-Path "C:\msys64\usr\bin\bash.exe")) {
        throw "MSYS2 installation failed"
    }
}

function Install-MsysPackages {
    param([string[]]$Packages)
    $bash = "C:\msys64\usr\bin\bash.exe"
    if (-not (Test-Path $bash)) {
        throw "MSYS2 bash not found at $bash"
    }
    $pkgList = ($Packages -join " ")
    $cmd = "pacman -S --noconfirm --needed $pkgList"
    Invoke-Checked -FilePath $bash -Arguments @("-lc", $cmd)
}

function Add-MsysToPath {
    $paths = @("C:\msys64\usr\bin", "C:\msys64\mingw64\bin")
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $items = @()
    if ($userPath) { $items = $userPath.Split(";") | Where-Object { $_ -ne "" } }

    $changed = $false
    foreach ($p in $paths) {
        if (-not ($items -contains $p)) {
            $items += $p
            $changed = $true
        }
    }
    if ($changed) {
        [Environment]::SetEnvironmentVariable("Path", ($items -join ";"), "User")
        Write-Step "Updated user PATH (new terminals will include MSYS2 tools)"
    }
}

function Add-CargoToPath {
    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if (-not (Test-Path $cargoBin)) { return }
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $items = @()
    if ($userPath) { $items = $userPath.Split(";") | Where-Object { $_ -ne "" } }
    if (-not ($items -contains $cargoBin)) {
        $items += $cargoBin
        [Environment]::SetEnvironmentVariable("Path", ($items -join ";"), "User")
        Write-Step "Updated user PATH with Cargo bin"
    }
}

Write-Step "Starting"

if ($InstallPython) {
    Ensure-Python
}

if ($InstallGit) {
    Ensure-Git
}

if ($InstallQemu) {
    Install-Qemu
}

if ($InstallNode) {
    Ensure-Node
}

if ($InstallDeno) {
	Ensure-Deno
}

if ($InstallMsys2 -and ($InstallXorriso -or $InstallMsys2Deps)) {
    Ensure-Msys2
}

if ($InstallXorriso) {
    Write-Step "Installing xorriso in MSYS2"
    Install-MsysPackages -Packages @("xorriso")
}

if ($InstallMsys2Deps) {
    Write-Step "Installing optional MSYS2 helper deps (nasm/mtools/binutils/llvm)"
    Install-MsysPackages -Packages @(
        "mingw-w64-x86_64-nasm",
        "mingw-w64-x86_64-mtools",
        "binutils",
        "mingw-w64-x86_64-llvm"
    )
}

if ($AddMsysToUserPath) {
    Add-MsysToPath
}

if ($AddCargoToUserPath) {
    Add-CargoToPath
}

Refresh-ProcessPath

if ($InstallPython -and -not (Test-Python)) { throw "python validation failed" }
if ($InstallGit -and -not (Test-Git)) { throw "git validation failed" }
if (-not (Test-Qemu)) { throw "QEMU validation failed" }
if ($InstallNode -and -not (Test-Npm)) { throw "npm validation failed" }
if ($InstallNode -and (Get-NodeMajorVersion) -lt $MinNodeMajor) { throw "node version validation failed" }
if ($InstallDeno -and -not (Test-Deno)) { throw "deno validation failed" }
if (-not (Test-Xorriso)) { throw "xorriso validation failed" }

Write-Step "READY"
Write-Step "python: OK"
Write-Step "git: OK"
Write-Step "qemu: OK"
Write-Step "npm: OK"
Write-Step ("node_major: {0}" -f (Get-NodeMajorVersion))
if ($InstallDeno) { Write-Step "deno: OK" }
Write-Step "xorriso: OK"

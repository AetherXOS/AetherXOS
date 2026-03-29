<#
Emulator helper for QEMU.

This script builds the specified cross target, locates a cross-built ELF artifact and
invokes `qemu-system-x86_64` with sensible defaults. It is NOT a full disk-image
builder — adjust the boot method (`-kernel`, ISO, or disk) according to your project.

Usage examples:
  .\scripts\setup_emulator.ps1                         # build default target and run QEMU
  .\scripts\setup_emulator.ps1 -Target release -Gdb    # build release and open GDB server
  .\scripts\setup_emulator.ps1 -Kernel path\to\img   # run given kernel image
#>

param(
    [string]$Target = "x86_64-unknown-none",
    [ValidateSet('debug','release')]
    [string]$Profile = "debug",
    [string]$Kernel = "",
    [int]$Memory = 1024,
    [int]$Cores = 2,
    [switch]$Gdb
)

function Ensure-Qemu {
    $qemu = Get-Command qemu-system-x86_64 -ErrorAction SilentlyContinue
    if (-not $qemu) {
        Write-Host "qemu-system-x86_64 not found. On Windows, install via Chocolatey: choco install qemu" -ForegroundColor Yellow
        return $false
    }
    return $true
}

if (-not (Ensure-Qemu)) { exit 1 }

Write-Host "Building cross target: $Target ($Profile)" -ForegroundColor Cyan
if ($Profile -eq 'release') {
    & cargo build --target $Target --release
} else {
    & cargo build --target $Target
}
if ($LASTEXITCODE -ne 0) { Write-Error "Cross build failed"; exit $LASTEXITCODE }

# If a kernel image path was provided, use it; otherwise search the target dir for ELF artifacts.
if ($Kernel -ne "") {
    if (-not (Test-Path $Kernel)) { Write-Error "Provided kernel path not found: $Kernel"; exit 1 }
    $kernelPath = (Resolve-Path $Kernel).Path
} else {
    $profileDir = if ($Profile -eq 'release') { 'release' } else { 'debug' }
    $searchDir = Join-Path -Path "target" -ChildPath "$Target\$profileDir"
    if (-not (Test-Path $searchDir)) {
        Write-Error "Target build directory not found: $searchDir"
        exit 1
    }
    $candidates = Get-ChildItem -Path $searchDir -File -ErrorAction SilentlyContinue | Where-Object { $_.Length -gt 1024 }
    # Prefer ELF artifacts: check first 4 bytes for 0x7F 'E' 'L' 'F'
    $elf = $null
    foreach ($f in $candidates) {
        try {
            $hdr = Get-Content -LiteralPath $f.FullName -Encoding Byte -TotalCount 4
            if ($hdr.Length -eq 4 -and $hdr[0] -eq 0x7F -and $hdr[1] -eq 0x45 -and $hdr[2] -eq 0x4C -and $hdr[3] -eq 0x46) {
                $elf = $f.FullName; break
            }
        } catch { }
    }
    if (-not $elf) {
        Write-Warning "No ELF artifacts found in $searchDir. Provide a kernel image via -Kernel or produce a bootable image."
        Write-Host "Candidates:" -ForegroundColor DarkGray
        $candidates | Select-Object -First 10 | ForEach-Object { Write-Host "  $_" }
        exit 0
    }
    $kernelPath = $elf
}

Write-Host "Using kernel artifact: $kernelPath" -ForegroundColor Green

$qemuArgs = @('-nographic', '-m', $Memory.ToString(), '-smp', $Cores.ToString(), '-kernel', $kernelPath)
if ($Gdb) { $qemuArgs += '-S'; $qemuArgs += '-gdb'; $qemuArgs += 'tcp::1234,server,nowait' }

Write-Host "Starting QEMU: qemu-system-x86_64 $($qemuArgs -join ' ')" -ForegroundColor DarkGray
try {
    & qemu-system-x86_64 @qemuArgs
} catch {
    Write-Error "Failed to launch QEMU: $_"
    exit 1
}

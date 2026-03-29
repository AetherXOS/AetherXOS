param(
  [ValidateSet("quick","strict")]
  [string]$Profile = "quick",
  [switch]$NoLock,
  [switch]$SkipConfirm,
  [ValidateSet("auto","en","tr")]
  [string]$Lang = "auto"
)
. (Join-Path $PSScriptRoot "lib/QuickWrapper.Common.ps1")
Invoke-QuickActionWrapper -ScriptRoot $PSScriptRoot -Action "emulator-interactive" -Profile $Profile -NoLock:$NoLock -SkipConfirm:$SkipConfirm -ExtraArgs @("-Lang", $Lang)

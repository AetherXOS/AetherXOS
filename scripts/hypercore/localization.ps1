function Resolve-HostShell {
    if (Test-HcCommand "pwsh") { return "pwsh" }
    if (Test-HcCommand "powershell") { return "powershell" }
    throw "HCERR[dependency_missing] PowerShell executable not found (pwsh/powershell)"
}

function Initialize-LocaleCatalog {
    param($Cfg)
    $preferred = if ($Lang) { $Lang } elseif ($Cfg.tooling.language) { [string]$Cfg.tooling.language } else { "en" }
    $base = Join-Path $PSScriptRoot ".."
    $paths = @(
        (Join-Path $base ("i18n/{0}.json" -f $preferred)),
        (Join-Path $base "i18n/en.json")
    )
    foreach ($p in $paths) {
        if (Test-Path $p) {
            try {
                $obj = Get-Content -Raw -Path $p -Encoding UTF8 | ConvertFrom-Json
                $map = @{}
                foreach ($prop in $obj.PSObject.Properties) {
                    $map[$prop.Name] = [string]$prop.Value
                }
                $script:LocaleCatalog = $map
                return
            } catch {}
        }
    }
    $script:LocaleCatalog = @{}
}

function Get-Msg {
    param(
        [string]$Key,
        [object[]]$FormatArgs = @()
    )
    $template = if ($script:LocaleCatalog.ContainsKey($Key)) { [string]$script:LocaleCatalog[$Key] } else { $Key }
    if ($FormatArgs.Count -eq 0) { return $template }
    return [string]::Format($template, $FormatArgs)
}

function Fail-Hc {
    param(
        [string]$Code,
        [string]$Message
    )
    throw ("HCERR[{0}] {1}" -f $Code, $Message)
}

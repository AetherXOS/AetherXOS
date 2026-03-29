Register-HcPluginCommand -Name "hello" -Description "Sanity plugin example" -Version "1.0.0" -MinApiVersion "1.0" -Action {
    Write-Host "[plugin:hello] plugin system ready"
}

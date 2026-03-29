Describe "HyperCore Tooling" {
    It "loads default config JSON" {
        $cfgPath = Join-Path $PSScriptRoot "..\..\config\hypercore.defaults.cjson"
        Test-Path $cfgPath | Should Be $true
        $cfg = Get-Content -Raw $cfgPath | ConvertFrom-Json
        $cfg.config_version | Should BeGreaterThan 0
        $cfg.profiles.quick.rounds | Should BeGreaterThan 0
        $cfg.paths.limine_bin_dir | Should Not BeNullOrEmpty
        $cfg.paths.plugin_dir | Should Not BeNullOrEmpty
        $cfg.tooling.language | Should Not BeNullOrEmpty
        $cfg.agent.port | Should BeGreaterThan 0
        $cfg.agent.auth_token | Should Not BeNullOrEmpty
        $cfg.agent.tokens.admin | Should Not BeNullOrEmpty
        @($cfg.agent.allowed_origins).Count | Should BeGreaterThan 0
        @($cfg.agent.hosts).Count | Should BeGreaterThan 0
    }

    It "loads task registry JSON" {
        $taskPath = Join-Path $PSScriptRoot "..\..\config\hypercore.tasks.cjson"
        Test-Path $taskPath | Should Be $true
        $tasks = (Get-Content -Raw $taskPath | ConvertFrom-Json).tasks
        ($tasks | Measure-Object).Count | Should BeGreaterThan 0
        (($tasks | Where-Object { $_.name -eq "p0" } | Measure-Object).Count) | Should Be 1
        (($tasks | Where-Object { $_.profile_args.strict } | Measure-Object).Count) | Should BeGreaterThan 0
    }

    It "uses config + plugin manifests" {
        (Test-Path (Join-Path $PSScriptRoot "..\..\config\hypercore.commands.cjson")) | Should Be $true
        (Test-Path (Join-Path $PSScriptRoot "..\..\config\quick.actions.cjson")) | Should Be $true
        (Test-Path (Join-Path $PSScriptRoot "..\..\config\hc_error_playbook.cjson")) | Should Be $true
        (Test-Path (Join-Path $PSScriptRoot "..\..\config\hypercore.policy.cjson")) | Should Be $true
        (Test-Path (Join-Path $PSScriptRoot "..\..\config\plugins\sample.plugin.cjson")) | Should Be $true
        $pm = Get-Content -Raw (Join-Path $PSScriptRoot "..\..\config\plugins\sample.plugin.cjson") | ConvertFrom-Json
        $pm.checksum_sha256.Length | Should BeGreaterThan 10
    }

    It "xtask command surface exists" {
        (Test-Path (Join-Path $PSScriptRoot "..\..\xtask\src\main.rs")) | Should Be $true
        (Test-Path (Join-Path $PSScriptRoot "..\..\xtask\src\cli.rs")) | Should Be $true
        (Test-Path (Join-Path $PSScriptRoot "..\..\xtask\src\commands\validation\test\host.rs")) | Should Be $true
    }

    It "quick action catalog includes risk/category metadata" {
        $path = Join-Path $PSScriptRoot "..\..\config\quick.actions.cjson"
        $actions = Get-Content -Raw -Path $path | ConvertFrom-Json
        @($actions).Count | Should BeGreaterThan 5
        foreach ($a in @($actions)) {
            [string]$a.risk_level | Should Not BeNullOrEmpty
            [string]$a.category | Should Not BeNullOrEmpty
        }
        (@($actions | Where-Object { $_.key -eq "guided-bootstrap" }).Count) | Should Be 1
        (@($actions | Where-Object { $_.key -eq "quality-gate" }).Count) | Should Be 1
        [string](@($actions | Where-Object { $_.key -eq "guided-bootstrap" })[0].aliases[0]) | Should Be "smart-run"
    }

    It "includes detached dashboard agent commands in quick catalog and command registry" {
        $qaPath = Join-Path $PSScriptRoot "..\..\config\quick.actions.cjson"
        $qa = Get-Content -Raw -Path $qaPath | ConvertFrom-Json
        $start = @($qa | Where-Object { $_.key -eq "agent-service-start" -or @($_.aliases) -contains "start-dashboard-agent" })[0]
        $startNoSafe = @($qa | Where-Object { $_.key -eq "agent-service-start-unsafe" -or @($_.aliases) -contains "start-dashboard-agent-nosafe" })[0]
        $start | Should Not BeNullOrEmpty
        $startNoSafe | Should Not BeNullOrEmpty
        [string]$start.cmds[0] | Should Be "dashboard-agent-bg"
        [string]$startNoSafe.cmds[0] | Should Be "dashboard-agent-nosafe-bg"

        $cmdPath = Join-Path $PSScriptRoot "..\..\config\hypercore.commands.cjson"
        $cmds = Get-Content -Raw -Path $cmdPath | ConvertFrom-Json
        (@($cmds | Where-Object { $_.name -eq "dashboard-agent-bg" }).Count) | Should Be 1
        (@($cmds | Where-Object { $_.name -eq "dashboard-agent-nosafe-bg" }).Count) | Should Be 1
    }
}


Describe "HyperCore Tooling" {
    It "loads default config JSON" {
        $cfgPath = Join-Path $PSScriptRoot "..\..\scripts\config\hypercore.defaults.json"
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
        $cfg.agent.max_concurrency | Should BeGreaterThan 0
        $cfg.agent.max_queue | Should BeGreaterThan 0
        $cfg.agent.log_retention_days | Should BeGreaterThan 0
        $cfg.agent.scheduler.enabled | Should BeOfType ([bool])
        @($cfg.agent.scheduler.tasks).Count | Should BeGreaterThan 0
        $cfg.agent.policy.roles.viewer.max_risk | Should Not BeNullOrEmpty
        @($cfg.agent.policy.roles.operator.denied_actions).Count | Should BeGreaterThan 0
        $cfg.agent.policy.roles.admin.max_risk | Should Not BeNullOrEmpty
        ($null -eq $cfg.ui.launcher_token) | Should Be $false
        $cfg.ui.launcher_rate_limit_per_minute | Should BeGreaterThan 0
        $cfg.ui.launcher_audit_retention_days | Should BeGreaterThan 0
    }

    It "loads task registry JSON" {
        $taskPath = Join-Path $PSScriptRoot "..\..\scripts\config\hypercore.tasks.json"
        Test-Path $taskPath | Should Be $true
        $tasks = (Get-Content -Raw $taskPath | ConvertFrom-Json).tasks
        ($tasks | Measure-Object).Count | Should BeGreaterThan 0
        (($tasks | Where-Object { $_.name -eq "p0" } | Measure-Object).Count) | Should Be 1
        (($tasks | Where-Object { $_.profile_args.strict } | Measure-Object).Count) | Should BeGreaterThan 0
    }

    It "has modular script files" {
        $root = Join-Path $PSScriptRoot "..\..\scripts\hypercore"
        (Test-Path (Join-Path $root "localization.ps1")) | Should Be $true
        (Test-Path (Join-Path $root "plugins.ps1")) | Should Be $true
        (Test-Path (Join-Path $root "novice.ps1")) | Should Be $true
        (Test-Path (Join-Path $PSScriptRoot "..\..\scripts\dashboard_ui_server.ps1")) | Should Be $true
        (Test-Path (Join-Path $PSScriptRoot "..\..\scripts\config\hypercore.commands.json")) | Should Be $true
        (Test-Path (Join-Path $PSScriptRoot "..\..\scripts\config\quick.actions.json")) | Should Be $true
        (Test-Path (Join-Path $PSScriptRoot "..\..\scripts\config\hc_error_playbook.json")) | Should Be $true
        (Test-Path (Join-Path $PSScriptRoot "..\..\scripts\config\hypercore.policy.json")) | Should Be $true
        (Test-Path (Join-Path $PSScriptRoot "..\..\scripts\plugins\sample.plugin.json")) | Should Be $true
        $pm = Get-Content -Raw (Join-Path $PSScriptRoot "..\..\scripts\plugins\sample.plugin.json") | ConvertFrom-Json
        $pm.checksum_sha256.Length | Should BeGreaterThan 10
    }

    It "has beginner wrapper scripts" {
        $scriptsRoot = Join-Path $PSScriptRoot "..\..\scripts"
        $launcherRoot = Join-Path $scriptsRoot "launchers"
        (Test-Path (Join-Path $scriptsRoot "quick.ps1")) | Should Be $true
        (Test-Path (Join-Path $scriptsRoot "smart-run.ps1")) | Should Be $true
        (Test-Path (Join-Path $scriptsRoot "start-dashboard-agent.ps1")) | Should Be $true
        (Test-Path (Join-Path $scriptsRoot "start-dashboard-agent-bg.ps1")) | Should Be $true
        (Test-Path (Join-Path $scriptsRoot "start-dashboard-agent-nosafe-bg.ps1")) | Should Be $true
        (Test-Path (Join-Path $scriptsRoot "build-os.ps1")) | Should Be $true
        (Test-Path (Join-Path $scriptsRoot "create-iso.ps1")) | Should Be $true
        (Test-Path (Join-Path $scriptsRoot "run-qemu.ps1")) | Should Be $true
        (Test-Path (Join-Path $scriptsRoot "run-qemu-live.ps1")) | Should Be $true
        (Test-Path (Join-Path $scriptsRoot "create-dashboard.ps1")) | Should Be $true
        (Test-Path (Join-Path $scriptsRoot "full-check.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "guided-bootstrap.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "environment-audit.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "environment-repair.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "dashboard-workspace-setup.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "release-readiness.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "workspace-bootstrap.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "build-and-smoke.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "build-boot-iso.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "run-emulator-smoke.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "run-emulator-interactive.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "dashboard-build.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "dashboard-validation.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "dashboard-open.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "quality-gate.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "agent-contract-verification.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "start-agent-service.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "start-agent-service-background.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "start-agent-service-unsafe.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "start-agent-service-unsafe-background.ps1")) | Should Be $true
        (Test-Path (Join-Path $launcherRoot "install-deno-runtime.ps1")) | Should Be $true
    }

    It "host test runner validates linux_compat feature matrix" {
        $scriptPath = Join-Path $PSScriptRoot "..\..\scripts\run_tests_host.ps1"
        $content = Get-Content -Raw -Path $scriptPath
        $content.Contains('Invoke-CargoCheckVariant -Label "linux_compat feature matrix" -Features "linux_compat telemetry"') | Should Be $true
        $content.Contains('Invoke-CargoCheckVariant -Label "vfs feature matrix" -Features "vfs telemetry"') | Should Be $true
        $content.Contains('Invoke-CargoCheckVariant -Label "posix process feature matrix" -Features "posix_process telemetry"') | Should Be $true
        $content.Contains('Invoke-CargoCheckVariant -Label "posix process/signal minimal matrix" -Features "posix_process posix_signal posix_time telemetry"') | Should Be $true
        $content.Contains('Invoke-CargoCheckVariant -Label "posix net feature matrix" -Features "posix_net telemetry"') | Should Be $true
        $content.Contains('Invoke-CargoCheckVariant -Label "posix fs/net feature matrix" -Features "posix_fs posix_net telemetry"') | Should Be $true
        $content.Contains('Invoke-CargoCheckVariant -Label "vfs fs feature matrix" -Features "vfs posix_fs telemetry"') | Should Be $true
        $content.Contains('Invoke-CargoCheckVariant -Label "posix process/signal feature matrix" -Features "vfs posix_fs posix_process posix_signal posix_time telemetry"') | Should Be $true
        $content.Contains('Invoke-CargoCheckVariant -Label "posix time feature matrix" -Features "posix_time telemetry"') | Should Be $true
        $content.Contains('Invoke-CargoCheckVariant -Label "integrated posix feature matrix" -Features "vfs posix_fs posix_net posix_process posix_signal posix_time telemetry"') | Should Be $true
    }

    It "launcher alias validator script succeeds" {
        $validator = Join-Path $PSScriptRoot "..\..\scripts\tools\validate-launcher-aliases.ps1"
        (Test-Path $validator) | Should Be $true
        & powershell -NoProfile -ExecutionPolicy Bypass -File $validator
        $LASTEXITCODE | Should Be 0
    }

    It "quick action catalog includes risk/category metadata" {
        $path = Join-Path $PSScriptRoot "..\\..\\scripts\\config\\quick.actions.json"
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
        $qaPath = Join-Path $PSScriptRoot "..\\..\\scripts\\config\\quick.actions.json"
        $qa = Get-Content -Raw -Path $qaPath | ConvertFrom-Json
        $start = @($qa | Where-Object { $_.key -eq "agent-service-start" -or @($_.aliases) -contains "start-dashboard-agent" })[0]
        $startNoSafe = @($qa | Where-Object { $_.key -eq "agent-service-start-unsafe" -or @($_.aliases) -contains "start-dashboard-agent-nosafe" })[0]
        $start | Should Not BeNullOrEmpty
        $startNoSafe | Should Not BeNullOrEmpty
        [string]$start.cmds[0] | Should Be "dashboard-agent-bg"
        [string]$startNoSafe.cmds[0] | Should Be "dashboard-agent-nosafe-bg"

        $cmdPath = Join-Path $PSScriptRoot "..\\..\\scripts\\config\\hypercore.commands.json"
        $cmds = Get-Content -Raw -Path $cmdPath | ConvertFrom-Json
        (@($cmds | Where-Object { $_.name -eq "dashboard-agent-bg" }).Count) | Should Be 1
        (@($cmds | Where-Object { $_.name -eq "dashboard-agent-nosafe-bg" }).Count) | Should Be 1
    }

    It "migrates older config versions for non-breaking help command" {
        $tmp = Join-Path $env:TEMP ("hc_cfg_{0}.json" -f [guid]::NewGuid().ToString("N"))
        $old = @{
            config_version = 1
            profiles = @{
                quick = @{ rounds = 1; memory_mb = "512"; cores = "1"; round_timeout_sec = 30; chaos_rate = 0.0; allow_timeout_success = $true }
                strict = @{ rounds = 2; memory_mb = "512"; cores = "1"; round_timeout_sec = 30; chaos_rate = 0.0; allow_timeout_success = $false }
            }
            paths = @{
                limine_bin_dir = "artifacts/limine/bin"
                qemu_default_out_dir = "artifacts/qemu_smoke_easy"
                doctor_report_path = "reports/tooling/doctor_report.json"
                health_report_path = "reports/tooling/health_report.json"
                telemetry_jsonl_path = "reports/tooling/hypercore_telemetry.jsonl"
                tasks_path = "scripts/config/hypercore.tasks.json"
                plugin_dir = "scripts/plugins"
            }
            install = @{ python = $true; git = $true; qemu = $true; msys2 = $true; xorriso = $true; msys2_deps = $true; add_msys_to_user_path = $true; add_cargo_to_user_path = $true }
            cleanup = @{ keep_latest_runs = 10; keep_days = 7; max_artifacts_gb = 2; targets = @("artifacts/nightly_runs") }
            tooling = @{ version = "0.1.0"; language = "en" }
            health = @{ weights = @{ doctor = 40; qemu_smoke = 30; p1_gate = 20; rc_verdict = 10 } }
        }
        $old | ConvertTo-Json -Depth 10 | Set-Content -Path $tmp -Encoding UTF8
        $scriptPath = Join-Path $PSScriptRoot "..\..\scripts\hypercore.ps1"
        & powershell -ExecutionPolicy Bypass -File $scriptPath -Command help -ConfigPath $tmp -NoLock | Out-Null
        $LASTEXITCODE | Should Be 0
        Remove-Item -Force -Path $tmp -ErrorAction SilentlyContinue
    }
}

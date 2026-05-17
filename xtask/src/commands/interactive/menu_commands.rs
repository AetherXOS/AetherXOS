use anyhow::Result;

pub trait MenuCommand {
    fn label(&self) -> &str;
    fn execute(&self) -> Result<bool>; // returns true if we should exit the menu
}

pub struct BuildKernelCommand;
impl MenuCommand for BuildKernelCommand {
    fn label(&self) -> &str { "🚀 Build AetherX Kernel" }
    fn execute(&self) -> Result<bool> {
        crate::commands::infra::build::interactive::run()?;
        Ok(false)
    }
}

pub struct DistroOpsCommand;
impl MenuCommand for DistroOpsCommand {
    fn label(&self) -> &str { "📦 Distro Operations & Supreme Wizard (ISO, Macros, Distro)" }
    fn execute(&self) -> Result<bool> {
        crate::commands::interactive::wizard::launch_supreme_wizard()?;
        Ok(false)
    }
}

pub struct MacroOpsCommand;
impl MenuCommand for MacroOpsCommand {
    fn label(&self) -> &str { "⏺️  Terminal Macro Management (Record/Replay)" }
    fn execute(&self) -> Result<bool> {
        // Just launch the supreme wizard and the user can select Macro Record/Replay
        crate::commands::interactive::wizard::launch_supreme_wizard()?;
        Ok(false)
    }
}

pub struct ManageFeaturesCommand;
impl MenuCommand for ManageFeaturesCommand {
    fn label(&self) -> &str { "🛠️  Kernel Configuration (Features)" }
    fn execute(&self) -> Result<bool> {
        crate::commands::interactive::features::manage_features()?;
        Ok(false)
    }
}

pub struct DashboardCommand;
impl MenuCommand for DashboardCommand {
    fn label(&self) -> &str { "📊 Open Nexus Intelligence Dashboard (TUI)" }
    fn execute(&self) -> Result<bool> {
        crate::utils::ui::dashboard::launch()?;
        Ok(false)
    }
}

pub struct SystemSettingsCommand;
impl MenuCommand for SystemSettingsCommand {
    fn label(&self) -> &str { "⚙️  AetherX Engine Settings" }
    fn execute(&self) -> Result<bool> {
        loop {
            let settings = crate::utils::core::config::get_settings();
            let options = vec![
                format!("📊 TUI HUD during builds: [{}]", if settings.tui_hud_enabled { "ENABLED" } else { "DISABLED" }),
                format!("🎨 TUI HUD Theme: [{}]", settings.hud_theme),
                format!("📢 Webhook Notifications: [{}]", if settings.webhook_enabled { "ENABLED" } else { "DISABLED" }),
                format!("🔊 Voice Feedback: [{}]", if settings.voice_enabled { "ENABLED" } else { "DISABLED" }),
                "↩️  Back to Main Menu".to_string(),
            ];

            let selection = inquire::Select::new("AetherX Engine Settings", options)
                .prompt()?;

            if selection.starts_with("📊") {
                crate::utils::core::config::update_settings(|s| s.tui_hud_enabled = !s.tui_hud_enabled)?;
            } else if selection.starts_with("🎨") {
                let next_theme = match settings.hud_theme {
                    crate::utils::core::config::HudTheme::Cyberpunk => crate::utils::core::config::HudTheme::Matrix,
                    crate::utils::core::config::HudTheme::Matrix => crate::utils::core::config::HudTheme::Steel,
                    crate::utils::core::config::HudTheme::Steel => crate::utils::core::config::HudTheme::Dracula,
                    crate::utils::core::config::HudTheme::Dracula => crate::utils::core::config::HudTheme::Cyberpunk,
                };
                crate::utils::core::config::update_settings(|s| s.hud_theme = next_theme)?;
            } else if selection.starts_with("📢") {
                crate::utils::core::config::update_settings(|s| s.webhook_enabled = !s.webhook_enabled)?;
            } else if selection.starts_with("🔊") {
                crate::utils::core::config::update_settings(|s| s.voice_enabled = !s.voice_enabled)?;
            } else {
                break;
            }
        }
        Ok(false)
    }
}

pub struct ExitCommand;
impl MenuCommand for ExitCommand {
    fn label(&self) -> &str { "🚪 Exit" }
    fn execute(&self) -> Result<bool> { Ok(true) }
}

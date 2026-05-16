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
    fn label(&self) -> &str { "📦 Distro Operations (ISO, Rootfs)" }
    fn execute(&self) -> Result<bool> {
        // Future: Distro wizard
        crate::utils::logging::info("MENU", "Distro wizard is under development", &[]);
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

pub struct ExitCommand;
impl MenuCommand for ExitCommand {
    fn label(&self) -> &str { "🚪 Exit" }
    fn execute(&self) -> Result<bool> { Ok(true) }
}

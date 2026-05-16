use anyhow::Result;
use inquire::{Select, MultiSelect, Text, Confirm};
use crate::engine::{Pipeline, ExecutionContext, BuildProfile};
use crate::utils::logging;

pub fn launch_supreme_wizard() -> Result<()> {
    use crate::constants::workflows::*;
    use crate::constants::ui::prompts::*;
    
    logging::status("WIZARD", "Initializing AetherX OS Supreme Configuration Wizard...");

    // 1. Select Workflow
    let workflows = vec![FULL_ISO, KERNEL_DEV, DOCS, DEBUG, UI_CUSTOM, UI_LOAD_PROFILE];
    let workflow = Select::new(WORKFLOW_SELECT, workflows).prompt()?;

    let mut ctx = ExecutionContext::from_defaults();
    
    if workflow == UI_LOAD_PROFILE {
        load_profile_logic(&mut ctx)?;
    }
    
    // 2. Select Architecture
    ctx.arch = Select::new(ARCH_SELECT, ARCH_LIST.to_vec()).prompt()?.to_string();

    // 3. Dry Run Mode
    ctx.dry_run = Confirm::new(DRY_RUN_CONFIRM).prompt()?;

    // 4. Select Features
    let catalog = crate::utils::features::load_feature_catalog()?;
    ctx.features = MultiSelect::new(FEATURE_SELECT, catalog.kernel_all.clone())
        .prompt()?
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    // 5. Custom Parameters
    if Confirm::new(PARAM_CONFIRM).prompt()? {
        collect_parameters(&mut ctx)?;
    }

    // 6. Save Profile option
    if Confirm::new(SAVE_PROFILE_CONFIRM).prompt()? {
        save_profile_logic(&ctx)?;
    }

    // 7. Dispatch via Controller
    crate::engine::controller::UniversalController::dispatch_workflow(workflow, &ctx)?;
    
    Ok(())
}

fn load_profile_logic(ctx: &mut ExecutionContext) -> Result<()> {
    use crate::constants::ui::prompts::*;
    let repo_root = crate::utils::paths::repo_root();
    let profiles = BuildProfile::list(&repo_root);
    if profiles.is_empty() {
        logging::warn("WIZARD", "No profiles found. Starting fresh...", &[]);
    } else {
        let profile_name = Select::new(PROFILE_SELECT, profiles).prompt()?;
        let profile = BuildProfile::load(&repo_root, &profile_name)?;
        profile.apply_to(ctx);
        logging::success("WIZARD", &format!("Profile '{}' loaded", profile_name), &[]);
    }
    Ok(())
}

fn collect_parameters(ctx: &mut ExecutionContext) -> Result<()> {
    loop {
        let param = Text::new("Parameter (leave empty to finish):").prompt()?;
        if param.is_empty() { break; }
        if let Some((k, v)) = param.split_once('=') {
            ctx.parameters.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    Ok(())
}

fn save_profile_logic(ctx: &ExecutionContext) -> Result<()> {
    use crate::constants::ui::prompts::*;
    let name = Text::new(PROFILE_NAME).prompt()?;
    let profile = BuildProfile {
        name: name.clone(),
        arch: ctx.arch.clone(),
        features: ctx.features.clone(),
        parameters: ctx.parameters.clone(),
    };
    profile.save(&crate::utils::paths::repo_root())?;
    logging::success("WIZARD", &format!("Profile '{}' saved", name), &[]);
    Ok(())
}

// Helper trait to convert Pipeline to Result<Pipeline> if needed, or just return it directly.
// Since I'm using into_result() above, I'll just change the match to return Result directly.
trait PipelineExt {
    fn into_result(self) -> Result<Pipeline>;
}
impl PipelineExt for Pipeline {
    fn into_result(self) -> Result<Pipeline> { Ok(self) }
}

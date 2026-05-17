use anyhow::Result;
use clap::Subcommand;
use crate::utils::executable::Executable;
use crate::engine::{ExecutionContext, controller::UniversalController};
use crate::utils::fs::paths::LAYOUT;

#[derive(Subcommand, Debug)]
pub enum PipelineAction {
    /// Execute a named pipeline workflow directly.
    Run {
        /// The name of the workflow or profile to execute.
        name: String,
        
        /// Optional architecture override.
        #[arg(long)]
        arch: Option<String>,
        
        /// Optional release mode override.
        #[arg(long)]
        release: Option<bool>,

        /// Enable dry-run mode (preview only).
        #[arg(long)]
        dry_run: bool,
        
        /// Add custom parameters (key=value).
        #[arg(long, short = 'P')]
        params: Vec<String>,
    },
    /// List all available workflows and profiles.
    List,
    /// Visualize a named pipeline workflow.
    Visualize {
        /// The name of the workflow or profile to visualize.
        name: String,
        /// The format to export (mermaid, dot).
        #[arg(long, default_value = "mermaid")]
        format: String,
    },
}

impl Executable for PipelineAction {
    fn execute(&self) -> Result<()> {
        match self {
            PipelineAction::Run { name, arch, release, dry_run, params } => {
                let mut ctx = ExecutionContext::from_defaults();
                
                if let Some(a) = arch { ctx.arch = a.clone(); }
                if let Some(r) = release { ctx.is_release = *r; }
                ctx.dry_run = *dry_run;
                
                for p in params {
                    if let Some((k, v)) = p.split_once('=') {
                        ctx.parameters.insert(k.to_string(), v.to_string());
                    }
                }
                
                UniversalController::dispatch_workflow(name, &ctx)
            }
            PipelineAction::Visualize { name, format } => {
                let ctx = ExecutionContext::from_defaults();
                let dag = UniversalController::build_pipeline(name, &ctx)?;
                
                match format.as_str() {
                    "mermaid" => println!("{}", dag.to_mermaid()),
                    "dot" => println!("{}", dag.to_dot()),
                    _ => anyhow::bail!("Unsupported visualization format: {}", format),
                }
                Ok(())
            }
            PipelineAction::List => {
                use crate::constants::workflows::*;
                println!("Available Workflows:");
                println!("  - {}", FULL_ISO);
                println!("  - {}", KERNEL_DEV);
                println!("  - {}", DOCS);
                println!("  - {}", DEBUG);
                
                let profiles = crate::engine::BuildProfile::list(&LAYOUT.root);
                if !profiles.is_empty() {
                    println!("\nAvailable Profiles:");
                    for p in profiles {
                        println!("  - {}", p);
                    }
                }
                Ok(())
            }
        }
    }
}

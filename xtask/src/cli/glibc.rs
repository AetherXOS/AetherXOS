use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum GlibcAction {
    Audit {
        #[arg(long, default_value_t = crate::constants::defaults::glibc::FORMAT_MD.to_string())]
        format: String,
        #[arg(long)]
        out: Option<String>,
        #[arg(long)]
        verbose: bool,
    },
    ClosureGate {
        #[arg(long)]
        quick: bool,
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        family: Option<String>,
        #[arg(long, default_value_t = crate::constants::defaults::glibc::FORMAT_MD.to_string())]
        format: String,
        #[arg(long)]
        out: Option<String>,
    },
    Scorecard {
        #[arg(long, default_value_t = crate::constants::defaults::glibc::FORMAT_JSON.to_string())]
        format: String,
        #[arg(long)]
        out: Option<String>,
    },
    CompatibilitySplit {
        #[arg(long)]
        strict: bool,
    },
}

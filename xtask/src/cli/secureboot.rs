use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum SecurebootAction {
    Sign {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        strict_verify: bool,
    },
    SbatValidate {
        #[arg(long)]
        strict: bool,
    },
    PcrReport,
    MokPlan,
    OvmfMatrix {
        #[arg(long)]
        dry_run: bool,
    },
}

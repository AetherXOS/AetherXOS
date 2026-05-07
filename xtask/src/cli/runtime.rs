use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum AbSlotAction {
    Init,
    Stage { slot: String },
    NightlyFlip,
    RecoveryGate,
}

impl crate::utils::executable::Executable for AbSlotAction {
    fn execute(&self) -> anyhow::Result<()> {
        crate::commands::runtime::ab_slot::execute(self)
    }
}

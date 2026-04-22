use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum AbSlotAction {
    Init,
    Stage { slot: String },
    NightlyFlip,
    RecoveryGate,
}

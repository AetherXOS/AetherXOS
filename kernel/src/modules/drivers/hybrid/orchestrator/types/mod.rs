pub mod requests;
pub mod plans;
pub mod reports;
pub mod audits;
pub mod gates;
pub mod abi;
pub mod virtualization;
pub mod session;

pub use requests::*;
pub use plans::*;
pub use reports::*;
pub use audits::*;
pub use gates::*;
pub use abi::*;
pub use virtualization::*;
pub use session::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendPreference {
    SideCarFirst,
    LibLinuxFirst,
    ReactOsFirst,
    DriverKitFirst,
}

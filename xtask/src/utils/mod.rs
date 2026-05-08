pub mod core;
pub mod fs;
pub mod net;
pub mod sys;
pub mod ui;
pub mod validation;

// 1. Forward modules for backward compatibility (utils::paths::...)
pub use core::config;
pub use core::context;
pub use fs::paths;
pub use fs::registry;
pub use sys::cargo;
pub use sys::executable;
pub use sys::process;
pub use sys::wsl;
pub use ui::help;
pub use ui::logging;
pub use ui::orchestrator as ui_orchestrator;
pub use ui::parser;
pub use validation::elf;
pub use validation::preflight;
pub use validation::report;

// 2. Forward items for backward compatibility (utils::ensure_dir)
pub use fs::hash::*;

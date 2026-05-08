pub mod core;
pub mod fs;
pub mod net;
pub mod sys;
pub mod ui;
pub mod validation;

// 1. Forward modules for backward compatibility (utils::paths::...)
pub use self::core::config;
pub use self::core::context;
pub use self::fs::paths;
pub use self::fs::registry;
pub use self::sys::cargo;
pub use self::sys::executable;
pub use self::sys::process;
pub use self::sys::wsl;
pub use self::ui::help;
pub use self::ui::logging;
pub use self::ui::orchestrator as ui_orchestrator;
pub use self::ui::parser;
pub use self::validation::elf;
pub use self::validation::preflight;
pub use self::validation::report;

// 2. Forward items for backward compatibility (utils::ensure_dir)
pub use self::fs::hash::*;

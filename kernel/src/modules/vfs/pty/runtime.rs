use spin::Mutex;

use crate::modules::vfs::dev_special::WinSize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PtyRuntimeConfig {
    pub default_winsize: WinSize,
    pub default_locked: bool,
    pub auto_sigwinch_on_resize: bool,
    pub allow_control_terminal_attach: bool,
    pub allow_control_terminal_detach: bool,
}

impl Default for PtyRuntimeConfig {
    fn default() -> Self {
        Self {
            default_winsize: WinSize::default(),
            default_locked: true,
            auto_sigwinch_on_resize: true,
            allow_control_terminal_attach: true,
            allow_control_terminal_detach: true,
        }
    }
}

lazy_static::lazy_static! {
    static ref PTY_RUNTIME_CONFIG: Mutex<PtyRuntimeConfig> = Mutex::new(PtyRuntimeConfig::default());
}

pub fn configure_pty_runtime<F>(mutator: F)
where
    F: FnOnce(&mut PtyRuntimeConfig),
{
    let mut config = PTY_RUNTIME_CONFIG.lock();
    mutator(&mut config);
}

pub fn pty_runtime_config() -> PtyRuntimeConfig {
    *PTY_RUNTIME_CONFIG.lock()
}

#[cfg(test)]
pub(crate) fn reset_pty_runtime_config() {
    *PTY_RUNTIME_CONFIG.lock() = PtyRuntimeConfig::default();
}

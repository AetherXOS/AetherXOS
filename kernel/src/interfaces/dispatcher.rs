pub trait Dispatcher {
    fn dispatch(&self, irq: u8);

    // Optional: Allow registering specific handlers at runtime
    // Not all dispatchers support this (e.g. DirectForwarding might be static)
    fn register_handler(&self, _irq: u8, _handler: fn(u8)) {}

    fn register_handler_with_ctx(&self, _irq: u8, _handler: fn(u8, usize) -> bool, _ctx: usize) {
        // Default: Do nothing or panic?
        // Let's do nothing to avoid panics in simple configs.
        // But for Main.rs to work, this must be supported if called.
    }
}

// --- PILLAR 4: IPC ---
pub trait IpcChannel {
    fn send(&self, msg: &[u8]);
    /// Receive a message into the provided buffer. Returns number of bytes read if a message exists.
    fn receive(&self, buffer: &mut [u8]) -> Option<usize>;
}

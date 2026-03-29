use alloc::vec::Vec;

const CORE_TO_LIBNET_BATCH: usize = 64;

pub trait EmbeddedHalNic {
    type Error;

    fn tx_frame(&mut self, frame: Vec<u8>) -> Result<(), Self::Error>;
    fn rx_frame(&mut self) -> Result<Option<Vec<u8>>, Self::Error>;
}

pub fn core_to_libnet_batch_size() -> usize {
    CORE_TO_LIBNET_BATCH
}

pub fn configured_default_pump_budget() -> usize {
    let configured = crate::config::KernelConfig::libnet_fast_path_pump_budget();
    if configured == 0 {
        return CORE_TO_LIBNET_BATCH;
    }
    core::cmp::min(configured, CORE_TO_LIBNET_BATCH)
}

pub fn pump_core_frames_into_libnet_with_budget(max_frames: usize) -> usize {
    if crate::modules::libnet::policy::ensure_l2_enabled().is_err() {
        return 0;
    }

    let budget = if max_frames == 0 {
        configured_default_pump_budget()
    } else {
        core::cmp::min(max_frames, CORE_TO_LIBNET_BATCH)
    };

    let frames = crate::kernel::net_core::drain_rx_frames(budget);
    if frames.is_empty() {
        return 0;
    }

    crate::modules::network::bridge::ingest_raw_ethernet_frames(frames)
}

pub fn pump_core_frames_into_libnet() -> usize {
    pump_core_frames_into_libnet_with_budget(configured_default_pump_budget())
}

mod planning;
mod sidecar;
mod audit;
mod telemetry;
mod linux;
mod support;
mod scoring;
mod session;

use super::*;
use crate::modules::drivers::hybrid::linux::LinuxShimDeviceKind;
use crate::modules::drivers::hybrid::reactos::{NtSymbol, NtSymbolTable};
use crate::modules::drivers::hybrid::liblinux::LibLinuxDispatchSample;
use crate::modules::drivers::hybrid::sidecar::{
    SideCarTelemetrySample, SideCarTelemetryStore,
};
use crate::modules::drivers::hybrid::{
    InMemorySideCarTransport, LinuxBridgeMessage, LinuxBridgeMessageKind, LinuxBridgePayload,
    LinuxSyscall, LinuxSyscallRequest, SideCarBootstrapPhase, SideCarRetryPolicy,
    ZeroCopyIoPolicy,
};

fn sample_pe() -> Vec<u8> {
    let mut image = vec![0u8; 0x400];
    image[0] = 0x4D;
    image[1] = 0x5A;
    image[0x3C..0x40].copy_from_slice(&(0x80u32).to_le_bytes());
    image[0x80..0x84].copy_from_slice(&0x0000_4550u32.to_le_bytes());

    let file_header = 0x84;
    image[file_header..file_header + 2].copy_from_slice(&0x8664u16.to_le_bytes());
    image[file_header + 2..file_header + 4].copy_from_slice(&1u16.to_le_bytes());
    image[file_header + 16..file_header + 18].copy_from_slice(&0xF0u16.to_le_bytes());

    let optional = file_header + 20;
    image[optional..optional + 2].copy_from_slice(&0x20Bu16.to_le_bytes());
    image[optional + 16..optional + 20].copy_from_slice(&0x1000u32.to_le_bytes());
    image[optional + 24..optional + 32].copy_from_slice(&0x140000000u64.to_le_bytes());
    image[optional + 56..optional + 60].copy_from_slice(&0x4000u32.to_le_bytes());
    image[optional + 60..optional + 64].copy_from_slice(&0x400u32.to_le_bytes());

    let section = optional + 0xF0;
    image[section + 8..section + 12].copy_from_slice(&0x200u32.to_le_bytes());
    image[section + 12..section + 16].copy_from_slice(&0x1000u32.to_le_bytes());
    image[section + 16..section + 20].copy_from_slice(&0x200u32.to_le_bytes());
    image[section + 20..section + 24].copy_from_slice(&0x200u32.to_le_bytes());

    image
}

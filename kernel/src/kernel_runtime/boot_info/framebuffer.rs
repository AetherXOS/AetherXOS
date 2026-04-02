use super::BootInfo;

#[cfg(target_arch = "x86_64")]
pub(super) fn collect_framebuffer(info: &mut BootInfo) {
    if let Some(fb) = hypercore::hal::x86_64::framebuffer() {
        info.framebuffer = Some(super::FramebufferInfo {
            phys_addr: fb.address.as_ptr().map(|ptr| ptr as u64).unwrap_or(0),
            width: fb.width,
            height: fb.height,
            pitch: fb.pitch,
            bpp: fb.bpp,
        });
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub(super) fn collect_framebuffer(_info: &mut BootInfo) {}

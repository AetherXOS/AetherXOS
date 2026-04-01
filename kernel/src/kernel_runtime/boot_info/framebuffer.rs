use super::{BootInfo, FramebufferInfo};

pub(super) fn collect_framebuffer(info: &mut BootInfo) {
    #[cfg(target_arch = "x86_64")]
    {
        if let Some(fb) = hypercore::hal::x86_64::framebuffer() {
            info.framebuffer = Some(FramebufferInfo {
                phys_addr: fb.address.as_ptr().map(|ptr| ptr as u64).unwrap_or(0),
                width: fb.width,
                height: fb.height,
                pitch: fb.pitch,
                bpp: fb.bpp,
            });
        }
    }
}

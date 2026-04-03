use core::cell::UnsafeCell;

#[repr(C, align(4096))]
pub(super) struct VmxonRegion {
    pub(super) revision_id: u32,
    pub(super) data: [u8; 4092],
}

#[repr(C, align(4096))]
pub(super) struct VmcsRegion {
    pub(super) revision_id: u32,
    pub(super) abort_indicator: u32,
    pub(super) data: [u8; 4088],
}

#[repr(C, align(4096))]
pub(super) struct VmcbRegion {
    pub(super) data: [u8; 4096],
}

struct StaticVirtRegion<T>(UnsafeCell<T>);

unsafe impl<T> Sync for StaticVirtRegion<T> {}

impl<T> StaticVirtRegion<T> {
    const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    fn ptr(&self) -> *const T {
        self.0.get() as *const T
    }

    unsafe fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        unsafe { f(&mut *self.0.get()) }
    }
}

static VMXON_REGION: StaticVirtRegion<VmxonRegion> = StaticVirtRegion::new(VmxonRegion {
    revision_id: 0,
    data: [0; 4092],
});

static VMCS_REGION: StaticVirtRegion<VmcsRegion> = StaticVirtRegion::new(VmcsRegion {
    revision_id: 0,
    abort_indicator: 0,
    data: [0; 4088],
});

static VMCB_REGION: StaticVirtRegion<VmcbRegion> =
    StaticVirtRegion::new(VmcbRegion { data: [0; 4096] });

#[inline(always)]
pub(super) unsafe fn with_vmcs_region_mut<R>(f: impl FnOnce(&mut VmcsRegion) -> R) -> R {
    unsafe { VMCS_REGION.with_mut(f) }
}

#[inline(always)]
pub(super) unsafe fn with_vmxon_region_mut<R>(f: impl FnOnce(&mut VmxonRegion) -> R) -> R {
    unsafe { VMXON_REGION.with_mut(f) }
}

#[inline(always)]
pub(super) fn vmcs_region_ptr() -> *const VmcsRegion {
    VMCS_REGION.ptr()
}

#[inline(always)]
pub(super) fn vmxon_region_ptr() -> *const VmxonRegion {
    VMXON_REGION.ptr()
}

#[inline(always)]
pub(super) fn vmcb_region_ptr() -> *const VmcbRegion {
    VMCB_REGION.ptr()
}

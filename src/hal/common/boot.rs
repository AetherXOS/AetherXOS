use limine::{DtbRequest, FramebufferRequest, HhdmRequest, MemmapRequest, RsdpRequest};

#[used]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new(0);
#[used]
static MEMORY_MAP_REQUEST: MemmapRequest = MemmapRequest::new(0);
#[used]
static RSDP_REQUEST: RsdpRequest = RsdpRequest::new(0);
#[used]
static DTB_REQUEST: DtbRequest = DtbRequest::new(0);
#[used]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new(0);

pub fn acpi_rsdp_addr() -> Option<u64> {
    let response = RSDP_REQUEST.get_response().get()?;
    let ptr = response.address.as_ptr()?;
    Some(ptr as u64)
}

pub fn dtb_addr() -> Option<u64> {
    let response = DTB_REQUEST.get_response().get()?;
    let ptr = response.dtb_ptr.as_ptr()?;
    Some(ptr as u64)
}

pub fn hhdm_offset() -> Option<u64> {
    let response = HHDM_REQUEST.get_response().get()?;
    Some(response.offset)
}

pub fn mem_map() -> Option<&'static limine::MemmapResponse> {
    MEMORY_MAP_REQUEST.get_response().get()
}

pub fn framebuffer() -> Option<&'static limine::Framebuffer> {
    FRAMEBUFFER_REQUEST
        .get_response()
        .get()
        .and_then(|response| {
            response
                .framebuffers()
                .first()
                .map(|ptr| unsafe { &*ptr.as_ptr() })
        })
}

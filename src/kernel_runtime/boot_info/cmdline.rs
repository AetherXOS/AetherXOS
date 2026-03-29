use super::{BootInfo, MAX_KERNEL_CMDLINE_BYTES};

pub(super) fn collect_kernel_cmdline(info: &mut BootInfo) {
    #[cfg(target_arch = "x86_64")]
    {
        static KERNEL_FILE_REQUEST: limine::KernelFileRequest = limine::KernelFileRequest::new(0);

        if let Some(response_ptr) = KERNEL_FILE_REQUEST.get_response().as_ptr().as_ref() {
            let response = unsafe { &**response_ptr };
            if let Some(file_ptr) = response.kernel_file.as_ptr().as_ref() {
                let file = unsafe { &**file_ptr };
                if let Some(cmdline_ptr) = file.cmdline.as_ptr() {
                    let mut idx = 0usize;
                    loop {
                        if idx >= MAX_KERNEL_CMDLINE_BYTES - 1 {
                            break;
                        }
                        let byte = unsafe { *(cmdline_ptr.add(idx)) } as u8;
                        if byte == 0 {
                            break;
                        }
                        info.kernel_cmdline[idx] = byte;
                        idx += 1;
                    }
                    info.kernel_cmdline[idx] = 0;
                }
            }
        }
    }
}

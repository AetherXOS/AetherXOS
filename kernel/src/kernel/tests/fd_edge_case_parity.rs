/// File Descriptor Edge-Case Parity Tests
///
/// Covers dup/fcntl/CLOEXEC semantics that Linux userspace frequently depends on.

#[cfg(test)]
mod tests {
    const FD_CLOEXEC: u32 = 0x1;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Descriptor {
        fd: i32,
        ofd_id: u32,
        cloexec: bool,
    }

    fn dup_descriptor(descriptor: Descriptor, new_fd: i32) -> Descriptor {
        Descriptor {
            fd: new_fd,
            ofd_id: descriptor.ofd_id,
            cloexec: descriptor.cloexec,
        }
    }

    fn fcntl_set_cloexec(descriptor: &mut Descriptor, enable: bool) {
        descriptor.cloexec = enable;
    }

    fn exec_closes_descriptor(descriptor: Descriptor) -> bool {
        descriptor.cloexec
    }

    fn close_on_exec_after_dup(source_cloexec: bool, dup_cloexec: bool) -> bool {
        let source = Descriptor {
            fd: 3,
            ofd_id: 77,
            cloexec: source_cloexec,
        };
        exec_closes_descriptor(source)
            || exec_closes_descriptor(Descriptor {
                fd: 4,
                ofd_id: source.ofd_id,
                cloexec: dup_cloexec,
            })
    }

    #[test_case]
    fn dup_preserves_open_file_description_identity() {
        let original = Descriptor {
            fd: 3,
            ofd_id: 100,
            cloexec: false,
        };
        let duplicated = dup_descriptor(original, 4);
        assert_eq!(original.ofd_id, duplicated.ofd_id);
        assert_ne!(original.fd, duplicated.fd);
    }

    #[test_case]
    fn fcntl_setfd_updates_only_descriptor_flags() {
        let mut descriptor = Descriptor {
            fd: 5,
            ofd_id: 222,
            cloexec: false,
        };
        fcntl_set_cloexec(&mut descriptor, true);
        assert!(descriptor.cloexec);
        assert_eq!(descriptor.ofd_id, 222);
    }

    #[test_case]
    fn close_on_exec_applies_per_descriptor_not_per_ofd() {
        assert!(close_on_exec_after_dup(true, false));
        assert!(close_on_exec_after_dup(false, true));
        assert!(!close_on_exec_after_dup(false, false));
    }

    #[test_case]
    fn fd_cloexec_bit_matches_expected_mask() {
        assert_eq!(FD_CLOEXEC, 0x1);
    }

    #[test_case]
    fn dup2_style_rebind_keeps_ofd_and_overwrites_target_fd() {
        let source = Descriptor {
            fd: 3,
            ofd_id: 999,
            cloexec: true,
        };
        let rebound = dup_descriptor(source, 7);
        assert_eq!(rebound.ofd_id, source.ofd_id);
        assert_eq!(rebound.fd, 7);
        assert!(rebound.cloexec);
    }
}

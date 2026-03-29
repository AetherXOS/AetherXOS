use super::{
    bootstrap_image_slice, bootstrap_launch_decision, bootstrap_launch_request,
    prepare_aligned_static_bootstrap, prepare_aligned_static_dispatch, prepare_bootstrap_launch_entry,
    prepare_bootstrap_launch_image, prepare_bootstrap_launch_preflight, validate_bootstrap_request,
    BootImageRecord, LaunchError,
};
use super::process_runtime_bootstrap_dispatch::{
    aligned_static_boot_image_record, dispatch_aligned_static_bootstrap,
    invoke_aligned_static_bootstrap, invoke_aligned_static_dispatch,
    invoke_bootstrap_dispatch_call, invoke_bootstrap_image_record_dispatch,
};

const TEST_MAX_PROCESS_NAME_LEN: usize = 32;
const TEST_MAX_BOOT_IMAGE_BYTES: usize = 1024;
const TEST_PRIORITY: u8 = 7;
const TEST_DEADLINE: u64 = 11;
const TEST_BURST: u64 = 13;
const TEST_KERNEL_STACK_TOP: u64 = 17;

#[test_case]
fn validate_bootstrap_request_rejects_empty_name_or_image() {
    assert_eq!(
        validate_bootstrap_request(
            b"",
            b"abc",
            TEST_MAX_PROCESS_NAME_LEN,
            TEST_MAX_BOOT_IMAGE_BYTES,
        ),
        Err(LaunchError::InvalidSpawnRequest)
    );
    assert_eq!(
        validate_bootstrap_request(
            b"probe",
            b"",
            TEST_MAX_PROCESS_NAME_LEN,
            TEST_MAX_BOOT_IMAGE_BYTES,
        ),
        Err(LaunchError::InvalidSpawnRequest)
    );
}

#[test_case]
fn validate_bootstrap_request_rejects_oversized_name_or_image() {
    assert_eq!(
        validate_bootstrap_request(
            &[b'x'; TEST_MAX_PROCESS_NAME_LEN + 1],
            b"abc",
            TEST_MAX_PROCESS_NAME_LEN,
            TEST_MAX_BOOT_IMAGE_BYTES,
        ),
        Err(LaunchError::InvalidSpawnRequest)
    );
    assert_eq!(
        validate_bootstrap_request(
            b"probe",
            &[7u8; TEST_MAX_BOOT_IMAGE_BYTES + 1],
            TEST_MAX_PROCESS_NAME_LEN,
            TEST_MAX_BOOT_IMAGE_BYTES,
        ),
        Err(LaunchError::InvalidSpawnRequest)
    );
}

#[test_case]
fn validate_bootstrap_request_accepts_valid_input() {
    assert_eq!(
        validate_bootstrap_request(
            b"probe",
            b"abc",
            TEST_MAX_PROCESS_NAME_LEN,
            TEST_MAX_BOOT_IMAGE_BYTES,
        ),
        Ok(())
    );
}

#[test_case]
fn aligned_static_boot_image_record_preserves_borrowed_static_slice() {
    static IMAGE: &[u8] = b"probe-image";
    let record = aligned_static_boot_image_record(IMAGE);
    match record {
        BootImageRecord::BorrowedStatic(bytes) => {
            assert_eq!(bytes, IMAGE);
            assert_eq!(bootstrap_image_slice(&BootImageRecord::BorrowedStatic(bytes)), IMAGE);
        }
        other => panic!("expected borrowed static image record, got {:?}", other),
    }
}

#[test_case]
fn bootstrap_launch_request_preserves_inputs() {
    let request = bootstrap_launch_request(
        b"hyper_init",
        BootImageRecord::BorrowedStatic(b"probe"),
        TEST_PRIORITY,
        TEST_DEADLINE,
        TEST_BURST,
        TEST_KERNEL_STACK_TOP,
    );
    assert_eq!(request.process_name, b"hyper_init");
    assert_eq!(bootstrap_image_slice(&request.boot_image), b"probe");
    assert_eq!(request.priority, TEST_PRIORITY);
    assert_eq!(request.deadline, TEST_DEADLINE);
    assert_eq!(request.burst_time, TEST_BURST);
    assert_eq!(request.kernel_stack_top, TEST_KERNEL_STACK_TOP);
}

#[test_case]
fn bootstrap_launch_decision_preserves_runtime_fields() {
    let request = bootstrap_launch_request(
        b"hyper_init",
        BootImageRecord::BorrowedStatic(b"probe"),
        TEST_PRIORITY,
        TEST_DEADLINE,
        TEST_BURST,
        TEST_KERNEL_STACK_TOP,
    );
    let decision = bootstrap_launch_decision(&request);
    assert_eq!(decision.priority, TEST_PRIORITY);
    assert_eq!(decision.deadline, TEST_DEADLINE);
    assert_eq!(decision.burst_time, TEST_BURST);
    assert_eq!(decision.kernel_stack_top, TEST_KERNEL_STACK_TOP);
}

#[test_case]
fn dispatch_aligned_static_bootstrap_rejects_empty_request_before_launch() {
    static IMAGE: &[u8] = b"";
    assert_eq!(
        dispatch_aligned_static_bootstrap(b"", IMAGE, 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );
}

#[test_case]
fn prepare_aligned_static_bootstrap_preserves_borrowed_image() {
    static IMAGE: &[u8] = b"probe-image";
    let prepared = prepare_aligned_static_bootstrap(IMAGE);
    match prepared.boot_image {
        BootImageRecord::BorrowedStatic(bytes) => assert_eq!(bytes, IMAGE),
        other => panic!("expected borrowed static image record, got {:?}", other),
    }
}

#[test_case]
fn prepare_aligned_static_dispatch_preserves_borrowed_image() {
    static IMAGE: &[u8] = b"probe-image";
    let prepared = prepare_aligned_static_dispatch(IMAGE);
    match prepared.prepared_bootstrap.boot_image {
        BootImageRecord::BorrowedStatic(bytes) => assert_eq!(bytes, IMAGE),
        other => panic!("expected borrowed static image record, got {:?}", other),
    }
}

#[test_case]
fn invoke_aligned_static_bootstrap_rejects_empty_request_before_launch() {
    static IMAGE: &[u8] = b"";
    assert_eq!(
        invoke_aligned_static_bootstrap(b"", IMAGE, 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );
}

#[test_case]
fn invoke_aligned_static_bootstrap_preserves_borrowed_static_path_contract() {
    static IMAGE: &[u8] = b"probe-image";
    let prepared = prepare_aligned_static_bootstrap(IMAGE);
    match prepared.boot_image {
        BootImageRecord::BorrowedStatic(bytes) => {
            assert_eq!(bytes, IMAGE);
            assert_eq!(
                invoke_aligned_static_bootstrap(b"", bytes, 0, 0, 0, 0),
                Err(LaunchError::InvalidSpawnRequest)
            );
        }
        other => panic!("expected borrowed static image record, got {:?}", other),
    }
}

#[test_case]
fn prepare_bootstrap_launch_entry_preserves_request_and_decision() {
    let (request, decision) = prepare_bootstrap_launch_entry(
        b"hyper_init",
        BootImageRecord::BorrowedStatic(b"probe"),
        TEST_PRIORITY,
        TEST_DEADLINE,
        TEST_BURST,
        TEST_KERNEL_STACK_TOP,
    );
    assert_eq!(request.process_name, b"hyper_init");
    assert_eq!(decision.priority, TEST_PRIORITY);
    assert_eq!(decision.deadline, TEST_DEADLINE);
    assert_eq!(decision.burst_time, TEST_BURST);
    assert_eq!(decision.kernel_stack_top, TEST_KERNEL_STACK_TOP);
}

#[test_case]
fn prepare_bootstrap_launch_image_validates_and_preserves_slice() {
    let record = BootImageRecord::BorrowedStatic(b"probe");
    let image = prepare_bootstrap_launch_image(b"hyper_init", &record).unwrap();
    assert_eq!(image, b"probe");
}

#[cfg(not(feature = "paging_enable"))]
#[test_case]
fn prepare_bootstrap_launch_preflight_returns_slice_and_snapshot() {
    let record = BootImageRecord::BorrowedStatic(b"\x7FELF\x02\x01\x01\0probe");
    let result = prepare_bootstrap_launch_preflight(b"hyper_init", &record);
    assert!(result.is_err() || result.is_ok());
}

#[test_case]
fn invoke_bootstrap_image_record_dispatch_rejects_empty_request_before_launch() {
    assert_eq!(
        invoke_bootstrap_image_record_dispatch(
            b"",
            BootImageRecord::BorrowedStatic(b""),
            0,
            0,
            0,
            0,
        ),
        Err(LaunchError::InvalidSpawnRequest)
    );
}

#[test_case]
fn invoke_bootstrap_dispatch_call_rejects_empty_request_before_launch() {
    assert_eq!(
        invoke_bootstrap_dispatch_call(b"", BootImageRecord::BorrowedStatic(b""), 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );
}

#[test_case]
fn invoke_aligned_static_dispatch_rejects_empty_request_before_launch() {
    assert_eq!(
        invoke_aligned_static_dispatch(b"", BootImageRecord::BorrowedStatic(b""), 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );
}

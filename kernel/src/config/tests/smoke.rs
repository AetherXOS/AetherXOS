use super::*;

fn assert_batch_error(
    result: Result<usize, ConfigBatchApplyError>,
    expected_index: usize,
    expected_key: &str,
    expected_cause: ConfigSetError,
) {
    let err = result.expect_err("batch operation should fail");
    assert_eq!(err.index, expected_index);
    assert_eq!(err.key, expected_key);
    assert_eq!(err.cause, expected_cause);
}

#[path = "smoke_runtime_limits.rs"]
mod smoke_runtime_limits;
#[path = "smoke_feature_controls.rs"]
mod smoke_feature_controls;
#[path = "smoke_override_batches.rs"]
mod smoke_override_batches;
#[path = "smoke_catalog.rs"]
mod smoke_catalog;

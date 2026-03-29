pub(crate) fn log_module_loader_runtime() {
    let loader = hypercore::kernel::module_loader::stats();
    hypercore::klog_info!(
        "Module loader: preflight={}/{} fp={:#x} parse={}/{} plan={}/{} map={}/{} bootstrap={}/{} segmat={}/{} segbytes={}",
        loader.preflight_success,
        loader.preflight_attempts,
        loader.last_preflight_fingerprint,
        loader.parse_success,
        loader.parse_attempts,
        loader.plan_success,
        loader.plan_attempts,
        loader.mapping_plan_success,
        loader.mapping_plan_attempts,
        loader.bootstrap_task_success,
        loader.bootstrap_task_attempts,
        loader.segment_materialization_success,
        loader.segment_materialization_attempts,
        loader.segment_materialized_bytes
    );
}

pub(crate) fn log_launch_pipeline() {
    let launch = hypercore::kernel::launch::stats();
    hypercore::klog_info!(
        "Launch pipeline: spawn={}/{} failures={} enqueue_failures={} validation_failures={} terminate={}/{} terminate_failures={} terminate_by_task={}/{} terminate_by_task_failures={} claim={}/{} claim_failures={} ack={}/{} ack_failures={} consume={}/{} consume_failures={} execute={}/{} execute_failures={} stale_scans={} stale_recycled={} stale_claim_timeout={} stale_ready_timeout={} processes={} last_tid={}",
        launch.spawn_success,
        launch.spawn_attempts,
        launch.spawn_failures,
        launch.enqueue_failures,
        launch.validation_failures,
        launch.terminate_success,
        launch.terminate_attempts,
        launch.terminate_failures,
        launch.terminate_by_task_success,
        launch.terminate_by_task_attempts,
        launch.terminate_by_task_failures,
        launch.claim_success,
        launch.claim_attempts,
        launch.claim_failures,
        launch.handoff_ack_success,
        launch.handoff_ack_attempts,
        launch.handoff_ack_failures,
        launch.handoff_consume_success,
        launch.handoff_consume_attempts,
        launch.handoff_consume_failures,
        launch.handoff_execute_success,
        launch.handoff_execute_attempts,
        launch.handoff_execute_failures,
        launch.stale_scan_calls,
        launch.stale_recycled_entries,
        launch.stale_claim_timeouts,
        launch.stale_ready_timeouts,
        launch.registered_processes,
        launch.last_task_id
    );
}

#[cfg(feature = "dispatcher")]
pub(crate) fn log_dispatcher_vectored_runtime() {
    let disp = hypercore::modules::dispatcher::vectored::stats();
    hypercore::klog_info!(
        "Dispatcher(vectored): register={} dispatch={} handled={} default={} invocations={} max_fanout={} storm_hints={} throttled={} window_resets={}",
        disp.register_calls,
        disp.dispatch_calls,
        disp.handled_calls,
        disp.default_hits,
        disp.handler_invocations,
        disp.max_fanout,
        disp.storm_hints,
        disp.throttled,
        disp.window_resets
    );
}

#[cfg(feature = "dispatcher")]
pub(crate) fn log_dispatcher_upcall_runtime() {
    let up = hypercore::modules::dispatcher::upcall::stats();
    hypercore::klog_info!(
        "Dispatcher(upcall): register={} overwrites={} unregister={}/{} resolve={}/{} delivered={} enqueued={} queue_drops={} consume={}/{} virq={}/{} pending_processes={} pending_deliveries={}",
        up.register_calls,
        up.register_overwrites,
        up.unregister_hits,
        up.unregister_calls,
        up.resolve_hits,
        up.resolve_calls,
        up.delivery_marks,
        up.delivery_enqueued,
        up.delivery_queue_drops,
        up.consume_hits,
        up.consume_calls,
        up.virq_inject_hits,
        up.virq_inject_calls,
        up.pending_processes,
        up.pending_deliveries
    );
}

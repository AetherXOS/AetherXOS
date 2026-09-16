use super::super::*;

#[cfg(feature = "process_abstraction")]
pub fn process_count() -> usize {
    PROCESS_REGISTRY.lock().len()
}

#[cfg(feature = "process_abstraction")]
pub fn process_ids_snapshot(out: &mut [ProcessId]) -> usize {
    let registry = PROCESS_REGISTRY.lock();
    let mut written = 0usize;
    for entry in registry.iter() {
        if written >= out.len() {
            break;
        }
        out[written] = entry.process_id;
        written += 1;
    }
    written
}

#[cfg(feature = "process_abstraction")]
pub fn launch_registry_snapshot(out: &mut [LaunchRegistrySnapshotEntry]) -> usize {
    let registry = PROCESS_REGISTRY.lock();
    let mut written = 0usize;
    for entry in registry.iter() {
        if written >= out.len() {
            break;
        }
        let (_, image_pages, _, _) = entry.process.image_state();
        let (_, mapped_pages) = entry.process.mapping_state();
        out[written] = LaunchRegistrySnapshotEntry {
            process_id: entry.process_id,
            task_id: entry.task_id,
            stage: entry.stage.as_usize(),
            image_pages,
            mapped_pages,
        };
        written += 1;
    }
    written
}


#[derive(Debug, Clone, Copy)]
pub struct TaskMetadata {
    pub vruntime: u64,
    pub weight: u64,
    pub group_id: u16,
}

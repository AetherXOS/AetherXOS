/// Macro to generate simple queue-based schedulers (DRY)
#[macro_export]
macro_rules! define_simple_scheduler {
    ($name:ident, $queue_type:ty, $add_method:path, $pop_method:path) => {
        pub struct $name {
            queue: $queue_type,
            time_slice_counter: u64,
        }

        impl $name {
            pub fn new() -> Self {
                Self {
                    queue: <$queue_type>::new(),
                    time_slice_counter: 0,
                }
            }
        }

        impl crate::interfaces::Scheduler for $name {
            type TaskItem = crate::interfaces::KernelTask;

            fn init(&mut self) {
                // Initialize queue logic if needed
            }

            fn add_task(&mut self, task: Self::TaskItem) {
                // Method call syntax works if trait is in scope, but for specific type methods we might need strict syntax
                // However, commonly methods like push_back are inherent.
                self.queue.push_back(task.id);
            }

            fn pick_next(&mut self) -> Option<crate::interfaces::TaskId> {
                self.queue.pop_front()
            }

            fn tick(&mut self, _current: crate::interfaces::TaskId) -> crate::interfaces::SchedulerAction {
                self.time_slice_counter += 1;
                // Simple Round Robin Logic default
                if self.time_slice_counter > crate::generated_consts::TIME_SLICE_NS / 1_000_000 { 
                    self.time_slice_counter = 0;
                    return crate::interfaces::SchedulerAction::Reschedule;
                }
                crate::interfaces::SchedulerAction::Continue
            }
        }
    };
}

/// Macro for Priority Based Schedulers
#[macro_export]
macro_rules! define_priority_scheduler {
    ($name:ident) => {
        pub struct $name {
            // Map Priority -> Queue of TaskIds
            queues: alloc::collections::BTreeMap<u8, alloc::vec::Vec<crate::interfaces::TaskId>>, 
        }

        impl $name {
            pub fn new() -> Self {
                Self { queues: alloc::collections::BTreeMap::new() }
            }
        }

        impl crate::interfaces::Scheduler for $name {
            type TaskItem = crate::interfaces::KernelTask;

            fn init(&mut self) {}

            fn add_task(&mut self, task: Self::TaskItem) {
                self.queues.entry(task.priority).or_insert(alloc::vec::Vec::new()).push(task.id);
            }

            fn pick_next(&mut self) -> Option<crate::interfaces::TaskId> {
                // Iterate from highest priority (0) to lowest
                for (_prio, queue) in self.queues.iter_mut() {
                    if !queue.is_empty() {
                        return Some(queue.remove(0)); // Standard Vec remove
                    }
                }
                None
            }

            fn tick(&mut self, _current: crate::interfaces::TaskId) -> crate::interfaces::SchedulerAction {
                crate::interfaces::SchedulerAction::Reschedule // Simple preemption for demo
            }
        }
    };
}

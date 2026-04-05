use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use super::lifecycle::{
    DeviceMatch, DriverBindingRecord, DriverKitClass, DriverKitEvent, DriverKitEventQueue,
    DriverKitHealthSnapshot, DriverKitRecoveryPolicy, DriverLifecycleState, UserModeDriverContext,
};

pub struct DriverKitRegistry {
    classes: Vec<Box<dyn DriverKitClass>>,
    bindings: Vec<DriverBindingRecord>,
    dispatch_success_count: u64,
    dispatch_failure_count: u64,
}

impl DriverKitRegistry {
    pub fn new() -> Self {
        Self {
            classes: Vec::new(),
            bindings: Vec::new(),
            dispatch_success_count: 0,
            dispatch_failure_count: 0,
        }
    }

    pub fn register(&mut self, class: Box<dyn DriverKitClass>) {
        self.classes.push(class);
    }

    pub fn len(&self) -> usize {
        self.classes.len()
    }

    pub fn binding_len(&self) -> usize {
        self.bindings.len()
    }

    pub fn binding_state(&self, binding_index: usize) -> Option<DriverLifecycleState> {
        self.bindings.get(binding_index).map(|binding| binding.state)
    }

    pub fn binding_next_recovery_tick(&self, binding_index: usize) -> Option<u32> {
        self.bindings
            .get(binding_index)
            .map(|binding| binding.next_recovery_tick)
    }

    pub fn select_best(&self, device: &DeviceMatch) -> Option<usize> {
        let mut best_idx = None;
        let mut best_score = 0u32;

        for (idx, class) in self.classes.iter().enumerate() {
            let score = class.score(device);
            if score > best_score {
                best_score = score;
                best_idx = Some(idx);
            }
        }

        best_idx
    }

    pub fn start_selected(
        &mut self,
        selected: usize,
        context: &UserModeDriverContext,
    ) -> Result<(), String> {
        let class = self
            .classes
            .get_mut(selected)
            .ok_or_else(|| "invalid selected class index".to_string())?;
        class.start(context)
    }

    pub fn stop_selected(&mut self, selected: usize) -> Result<(), String> {
        let class = self
            .classes
            .get_mut(selected)
            .ok_or_else(|| "invalid selected class index".to_string())?;
        class.stop()
    }

    pub fn dispatch_selected(
        &mut self,
        selected: usize,
        event: DriverKitEvent,
    ) -> Result<(), String> {
        let class = self
            .classes
            .get_mut(selected)
            .ok_or_else(|| "invalid selected class index".to_string())?;
        match class.on_event(event) {
            Ok(()) => {
                self.dispatch_success_count = self.dispatch_success_count.saturating_add(1);
                Ok(())
            }
            Err(err) => {
                self.dispatch_failure_count = self.dispatch_failure_count.saturating_add(1);
                Err(err)
            }
        }
    }

    pub fn bind_device(&mut self, device: DeviceMatch) -> Option<DriverBindingRecord> {
        self.bind_device_with_policy(device, DriverKitRecoveryPolicy::balanced())
    }

    pub fn bind_device_with_policy(
        &mut self,
        device: DeviceMatch,
        recovery_policy: DriverKitRecoveryPolicy,
    ) -> Option<DriverBindingRecord> {
        let selected = self.select_best(&device)?;
        let record = DriverBindingRecord {
            device,
            selected_index: selected,
            state: DriverLifecycleState::Bound,
            retry_count: 0,
            last_fault_tick: 0,
            next_recovery_tick: 0,
            recovery_policy,
        };
        self.bindings.push(record);
        Some(record)
    }

    pub fn start_binding(
        &mut self,
        binding_index: usize,
        context: &UserModeDriverContext,
    ) -> Result<(), String> {
        let selected = self
            .bindings
            .get(binding_index)
            .ok_or_else(|| "invalid binding index".to_string())?
            .selected_index;
        self.start_selected(selected, context)?;
        if let Some(binding) = self.bindings.get_mut(binding_index) {
            binding.state = DriverLifecycleState::Started;
        }
        Ok(())
    }

    pub fn stop_binding(&mut self, binding_index: usize) -> Result<(), String> {
        let selected = self
            .bindings
            .get(binding_index)
            .ok_or_else(|| "invalid binding index".to_string())?
            .selected_index;
        self.stop_selected(selected)?;
        if let Some(binding) = self.bindings.get_mut(binding_index) {
            binding.state = DriverLifecycleState::Stopped;
        }
        Ok(())
    }

    pub fn pump_events_for_binding(
        &mut self,
        binding_index: usize,
        queue: &mut DriverKitEventQueue,
        budget: usize,
    ) -> Result<usize, String> {
        let selected = self
            .bindings
            .get(binding_index)
            .ok_or_else(|| "invalid binding index".to_string())?
            .selected_index;
        let mut handled = 0usize;
        let mut remaining = budget.max(1);
        while remaining > 0 {
            let Some(event) = queue.pop() else {
                break;
            };
            self.dispatch_selected(selected, event)?;
            handled += 1;
            remaining -= 1;
        }
        Ok(handled)
    }

    pub fn mark_fault(&mut self, binding_index: usize) -> Result<(), String> {
        self.mark_fault_at_tick(binding_index, 0)
    }

    pub fn mark_fault_at_tick(
        &mut self,
        binding_index: usize,
        current_tick: u32,
    ) -> Result<(), String> {
        let binding = self
            .bindings
            .get_mut(binding_index)
            .ok_or_else(|| "invalid binding index".to_string())?;
        binding.retry_count = binding.retry_count.saturating_add(1);
        binding.last_fault_tick = current_tick;
        binding.state = if binding.recovery_policy.quarantine_on_fault
            || binding.retry_count > binding.recovery_policy.max_retries
        {
            binding.next_recovery_tick = 0;
            DriverLifecycleState::Quarantined
        } else {
            let shift = (binding.retry_count.saturating_sub(1) as u32).min(16);
            let factor = 1u32.checked_shl(shift).unwrap_or(u32::MAX);
            let raw_backoff = binding
                .recovery_policy
                .base_recovery_backoff_ticks
                .saturating_mul(factor);
            let backoff = raw_backoff.min(binding.recovery_policy.max_recovery_backoff_ticks);
            binding.next_recovery_tick = current_tick.saturating_add(backoff);
            DriverLifecycleState::Faulted
        };
        Ok(())
    }

    pub fn recover_faulted_binding(
        &mut self,
        binding_index: usize,
        context: &UserModeDriverContext,
    ) -> Result<(), String> {
        self.recover_faulted_binding_at_tick(binding_index, context, u32::MAX)
    }

    pub fn recover_faulted_binding_at_tick(
        &mut self,
        binding_index: usize,
        context: &UserModeDriverContext,
        current_tick: u32,
    ) -> Result<(), String> {
        let selected = self
            .bindings
            .get(binding_index)
            .ok_or_else(|| "invalid binding index".to_string())?
            .selected_index;
        let binding = self
            .bindings
            .get(binding_index)
            .ok_or_else(|| "invalid binding index".to_string())?;
        let state = binding.state;
        let next_recovery_tick = binding.next_recovery_tick;

        match state {
            DriverLifecycleState::Faulted => {
                if current_tick < next_recovery_tick {
                    return Err("binding recovery deferred due to backoff window".to_string());
                }
                self.start_selected(selected, context)?;
                if let Some(binding) = self.bindings.get_mut(binding_index) {
                    binding.state = DriverLifecycleState::Started;
                    binding.next_recovery_tick = 0;
                }
                Ok(())
            }
            DriverLifecycleState::Quarantined => {
                Err("binding is quarantined and cannot be recovered automatically".to_string())
            }
            _ => Ok(()),
        }
    }

    pub fn health_snapshot(&self) -> DriverKitHealthSnapshot {
        let mut started_count = 0usize;
        let mut faulted_count = 0usize;
        let mut quarantined_count = 0usize;
        for binding in &self.bindings {
            match binding.state {
                DriverLifecycleState::Started => started_count += 1,
                DriverLifecycleState::Faulted => faulted_count += 1,
                DriverLifecycleState::Quarantined => quarantined_count += 1,
                _ => {}
            }
        }

        DriverKitHealthSnapshot {
            class_count: self.classes.len(),
            binding_count: self.bindings.len(),
            started_count,
            faulted_count,
            quarantined_count,
            dispatch_success_count: self.dispatch_success_count,
            dispatch_failure_count: self.dispatch_failure_count,
        }
    }

    pub fn health_score(&self) -> u8 {
        Self::health_score_from_snapshot(&self.health_snapshot())
    }

    pub fn health_score_from_snapshot(snapshot: &DriverKitHealthSnapshot) -> u8 {
        let bindings = snapshot.binding_count.max(1) as i32;
        let total_dispatch = snapshot
            .dispatch_success_count
            .saturating_add(snapshot.dispatch_failure_count)
            .max(1) as i32;

        let started_bonus = (snapshot.started_count as i32 * 25) / bindings;
        let fault_penalty = (snapshot.faulted_count as i32 * 20) / bindings;
        let quarantine_penalty = (snapshot.quarantined_count as i32 * 35) / bindings;
        let dispatch_penalty = (snapshot.dispatch_failure_count as i32 * 45) / total_dispatch;

        let score = 70 + started_bonus - fault_penalty - quarantine_penalty - dispatch_penalty;
        score.clamp(0, 100) as u8
    }
}

impl Default for DriverKitRegistry {
    fn default() -> Self {
        Self::new()
    }
}

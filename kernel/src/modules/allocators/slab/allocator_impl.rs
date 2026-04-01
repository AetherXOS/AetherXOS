use super::*;

impl SlabAllocator {
    #[inline(always)]
    pub(super) fn effective_cpu_id() -> usize {
        let cpu_id = crate::hal::cpu::id();
        if cpu_id < MAX_CPUS {
            cpu_id
        } else {
            0
        }
    }

    #[inline(always)]
    pub(super) fn sample_reclaim_queue_depth(&self, idx: usize) -> usize {
        let segments = self.segments.lock();
        let mut depth = 0usize;
        for slot in segments.iter() {
            if slot.active
                && slot.class_idx == idx
                && !slot.reclaiming
                && slot.free_blocks == slot.total_blocks
            {
                depth = depth.saturating_add(1);
            }
        }
        SLAB_RECLAIM_QUEUE_DEPTH_LAST.store(depth, Ordering::Relaxed);
        update_max_usize(&SLAB_RECLAIM_QUEUE_DEPTH_MAX, depth);
        SLAB_RECLAIM_QUEUE_DEPTH_SAMPLES.fetch_add(1, Ordering::Relaxed);
        SLAB_RECLAIM_QUEUE_DEPTH_TOTAL.fetch_add(depth as u64, Ordering::Relaxed);
        record_telemetry_bucket(
            depth as u64,
            [
                &SLAB_RECLAIM_QD_BUCKET_0,
                &SLAB_RECLAIM_QD_BUCKET_1,
                &SLAB_RECLAIM_QD_BUCKET_2_3,
                &SLAB_RECLAIM_QD_BUCKET_4_7,
                &SLAB_RECLAIM_QD_BUCKET_GE8,
            ],
        );
        depth
    }

    pub(super) fn register_segment(
        &self,
        idx: usize,
        base: usize,
        alloc_size: usize,
        alloc_align: usize,
        total_blocks: usize,
    ) -> bool {
        let mut segments = self.segments.lock();
        for slot in segments.iter_mut() {
            if !slot.active {
                *slot = SlabSegment {
                    active: true,
                    reclaiming: false,
                    class_idx: idx,
                    base,
                    alloc_size,
                    alloc_align,
                    total_blocks,
                    free_blocks: total_blocks,
                };
                let active = SLAB_ACTIVE_SEGMENTS.fetch_add(1, Ordering::Relaxed) + 1;
                let mut peak = SLAB_PEAK_ACTIVE_SEGMENTS.load(Ordering::Relaxed);
                while active > peak {
                    match SLAB_PEAK_ACTIVE_SEGMENTS.compare_exchange_weak(
                        peak,
                        active,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => break,
                        Err(observed) => peak = observed,
                    }
                }
                return true;
            }
        }
        false
    }

    pub(super) fn note_segment_alloc(&self, idx: usize, ptr: *mut u8) -> bool {
        let ptr_usize = ptr as usize;
        let mut segments = self.segments.lock();
        for slot in segments.iter_mut() {
            if slot.active && slot.class_idx == idx && slot.contains(ptr_usize) {
                if slot.free_blocks > 0 {
                    slot.free_blocks -= 1;
                }
                return true;
            }
        }
        false
    }

    pub(super) fn note_segment_free_and_maybe_reclaim(
        &self,
        idx: usize,
        ptr: *mut u8,
    ) -> (bool, Option<usize>) {
        let ptr_usize = ptr as usize;
        let mut segments = self.segments.lock();
        for (slot_idx, slot) in segments.iter_mut().enumerate() {
            if slot.active && slot.class_idx == idx && slot.contains(ptr_usize) {
                if slot.free_blocks < slot.total_blocks {
                    slot.free_blocks += 1;
                }
                if slot.free_blocks == slot.total_blocks && !slot.reclaiming {
                    slot.reclaiming = true;
                    return (true, Some(slot_idx));
                }
                return (true, None);
            }
        }
        (false, None)
    }

    pub(super) fn mark_reclaim_candidate_for_class(&self, idx: usize) -> Option<usize> {
        let mut segments = self.segments.lock();
        for (slot_idx, slot) in segments.iter_mut().enumerate() {
            if slot.active
                && slot.class_idx == idx
                && !slot.reclaiming
                && slot.free_blocks == slot.total_blocks
            {
                slot.reclaiming = true;
                return Some(slot_idx);
            }
        }
        None
    }

    pub(super) fn try_reclaim_segment(&self, idx: usize, segment_slot: usize) {
        SLAB_RECLAIM_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
        let start_tick = current_latency_tick();

        let (base, end, total_blocks, layout) = {
            let segments = self.segments.lock();
            if segment_slot >= segments.len() {
                return;
            }
            let segment = segments[segment_slot];
            if !segment.active
                || !segment.reclaiming
                || segment.class_idx != idx
                || segment.free_blocks != segment.total_blocks
                || segment.alloc_size == 0
                || segment.alloc_align == 0
            {
                return;
            }
            let Ok(layout) = Layout::from_size_align(segment.alloc_size, segment.alloc_align)
            else {
                return;
            };
            (segment.base, segment.end(), segment.total_blocks, layout)
        };

        let mut removed = 0usize;
        for cpu in 0..MAX_CPUS {
            let mut cache = self.caches[cpu][idx].lock();
            unsafe {
                removed = removed.saturating_add(cache.drain_range(base, end));
            }
        }

        SLAB_RECLAIM_SWEEPED_BLOCKS.fetch_add(removed as u64, Ordering::Relaxed);

        let can_reclaim = {
            let mut segments = self.segments.lock();
            if segment_slot >= segments.len() {
                false
            } else {
                let segment = &mut segments[segment_slot];
                if segment.active
                    && segment.reclaiming
                    && segment.class_idx == idx
                    && segment.free_blocks == segment.total_blocks
                    && removed == total_blocks
                {
                    *segment = SlabSegment::EMPTY;
                    SLAB_ACTIVE_SEGMENTS.fetch_sub(1, Ordering::Relaxed);
                    true
                } else {
                    segment.reclaiming = false;
                    false
                }
            }
        };

        if can_reclaim {
            unsafe {
                self.fallback_allocator.dealloc(base as *mut u8, layout);
            }
            SLAB_RECLAIM_SUCCESSES.fetch_add(1, Ordering::Relaxed);
        }
        let delta = current_latency_tick().saturating_sub(start_tick);
        SLAB_RECLAIM_LATENCY_TOTAL_TICKS.fetch_add(delta, Ordering::Relaxed);
        update_max_u64(&SLAB_RECLAIM_LATENCY_MAX_TICKS, delta);
        record_telemetry_bucket(
            delta,
            [
                &SLAB_RECLAIM_LAT_BUCKET_0,
                &SLAB_RECLAIM_LAT_BUCKET_1,
                &SLAB_RECLAIM_LAT_BUCKET_2_3,
                &SLAB_RECLAIM_LAT_BUCKET_4_7,
                &SLAB_RECLAIM_LAT_BUCKET_GE8,
            ],
        );
    }

    #[inline(always)]
    pub(super) fn pressure_reclaim_budget(&self) -> usize {
        let release_budget = SLAB_RELEASE_BATCH.load(Ordering::Relaxed).clamp(1, 64);
        let scan_budget = SLAB_PRESSURE_SCAN_BUDGET
            .load(Ordering::Relaxed)
            .clamp(1, 256);
        match slab_reclaim_profile() {
            SlabReclaimProfile::Conservative => 1,
            SlabReclaimProfile::Balanced => release_budget.min(scan_budget),
            SlabReclaimProfile::Aggressive => release_budget.saturating_mul(2).min(scan_budget),
        }
    }

    pub(super) fn maybe_run_pressure_reclaim(&self, idx: usize, pressure_reclaim: bool) {
        let reclaim_profile = slab_reclaim_profile();
        let pressure_budget = self.pressure_reclaim_budget();
        if pressure_reclaim {
            let _ = self.sample_reclaim_queue_depth(idx);
            for _ in 0..pressure_budget {
                if let Some(slot) = self.mark_reclaim_candidate_for_class(idx) {
                    SLAB_PRESSURE_RECLAIM_PASSES.fetch_add(1, Ordering::Relaxed);
                    self.try_reclaim_segment(idx, slot);
                } else {
                    break;
                }
            }
        } else if reclaim_profile == SlabReclaimProfile::Aggressive {
            let _ = self.sample_reclaim_queue_depth(idx);
            if let Some(slot) = self.mark_reclaim_candidate_for_class(idx) {
                SLAB_PRESSURE_RECLAIM_PASSES.fetch_add(1, Ordering::Relaxed);
                self.try_reclaim_segment(idx, slot);
            }
        }
    }
}

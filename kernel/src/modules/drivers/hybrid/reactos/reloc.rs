use alloc::vec::Vec;

use super::{PeRelocationBlock, RelocationPatch};

pub fn plan_relocation_patches(
    blocks: &[PeRelocationBlock],
    old_image_base: u64,
    new_image_base: u64,
) -> Vec<RelocationPatch> {
    let delta = new_image_base.wrapping_sub(old_image_base);
    let mut patches = Vec::new();

    for block in blocks {
        let entries = block.entry_count as u32;
        let mut i = 0u32;
        while i < entries {
            let target_rva = block.page_rva.saturating_add(i.saturating_mul(8));
            let old_value = old_image_base.wrapping_add(target_rva as u64);
            let new_value = old_value.wrapping_add(delta);
            patches.push(RelocationPatch {
                target_rva,
                old_value,
                new_value,
            });
            i += 1;
        }
    }

    patches
}

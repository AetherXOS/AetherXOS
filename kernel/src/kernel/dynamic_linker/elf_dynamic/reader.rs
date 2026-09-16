pub fn parse_sysv_hash_nchain(image: &[u8], hash_off: usize) -> Option<usize> {
    if hash_off.checked_add(8)? > image.len() {
        return None;
    }
    let nchain_bytes: [u8; 4] = image.get(hash_off + 4..hash_off + 8)?.try_into().ok()?;
    Some(u32::from_le_bytes(nchain_bytes) as usize)
}

pub fn parse_gnu_hash_symbol_count(image: &[u8], hash_off: usize) -> Option<usize> {
    if hash_off.checked_add(16)? > image.len() {
        return None;
    }
    let nbuckets = u32::from_le_bytes(image.get(hash_off..hash_off + 4)?.try_into().ok()?) as usize;
    let symoffset =
        u32::from_le_bytes(image.get(hash_off + 4..hash_off + 8)?.try_into().ok()?) as usize;
    let bloom_size =
        u32::from_le_bytes(image.get(hash_off + 8..hash_off + 12)?.try_into().ok()?) as usize;
    let buckets_off = hash_off
        .checked_add(16)?
        .checked_add(bloom_size.checked_mul(8)?)?;
    let chains_off = buckets_off.checked_add(nbuckets.checked_mul(4)?)?;
    if chains_off > image.len() {
        return None;
    }

    let mut max_bucket = 0usize;
    for i in 0..nbuckets {
        let off = buckets_off.checked_add(i.checked_mul(4)?)?;
        let raw: [u8; 4] = image.get(off..off + 4)?.try_into().ok()?;
        max_bucket = max_bucket.max(u32::from_le_bytes(raw) as usize);
    }
    if max_bucket < symoffset {
        return Some(symoffset);
    }

    let mut sym_index = max_bucket;
    loop {
        let chain_index = sym_index.checked_sub(symoffset)?;
        let off = chains_off.checked_add(chain_index.checked_mul(4)?)?;
        let raw: [u8; 4] = image.get(off..off + 4)?.try_into().ok()?;
        let hash = u32::from_le_bytes(raw);
        sym_index = sym_index.checked_add(1)?;
        if (hash & 1) != 0 {
            return Some(sym_index);
        }
    }
}

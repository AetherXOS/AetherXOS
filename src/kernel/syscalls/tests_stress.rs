use super::*;

#[test_case]
fn mapped_unmapped_stress_rejects_crossing_unmapped_gap() {
    let mapped_pages = [true, true, true, false, true, true, true, true];
    let writable_pages = [true; 8];
    let first = USER_SPACE_BOTTOM_INCLUSIVE;

    for page_idx in 0..mapped_pages.len() {
        let page_base = first + page_idx * PAGE_SIZE;
        let ptr = page_base + (PAGE_SIZE - 64);
        let len = 128;
        let valid = user_access_range_valid_with(ptr, len, UserAccessMode::Read, |page, mode| {
            access_ok_with_windows(page, mode, &mapped_pages, &writable_pages)
        });

        let next_idx = page_idx + 1;
        let expected = if next_idx < mapped_pages.len() {
            mapped_pages[page_idx] && mapped_pages[next_idx]
        } else {
            false
        };
        assert_eq!(valid, expected);
    }
}

#[test_case]
fn mapped_unmapped_stress_write_requires_writable_for_all_pages() {
    let mapped_pages = [true, true, true, true, true, true];
    let writable_pages = [true, false, true, true, false, true];
    let first = USER_SPACE_BOTTOM_INCLUSIVE;

    for page_idx in 0..(mapped_pages.len() - 1) {
        let ptr = first + page_idx * PAGE_SIZE + (PAGE_SIZE - 32);
        let len = 64;
        let write_valid = user_access_range_valid_with(ptr, len, UserAccessMode::Write, |page, mode| {
            access_ok_with_windows(page, mode, &mapped_pages, &writable_pages)
        });
        let read_valid = user_access_range_valid_with(ptr, len, UserAccessMode::Read, |page, mode| {
            access_ok_with_windows(page, mode, &mapped_pages, &writable_pages)
        });

        let expected_write = writable_pages[page_idx] && writable_pages[page_idx + 1];
        assert!(read_valid);
        assert_eq!(write_valid, expected_write);
    }
}

#[test_case]
fn mapped_unmapped_stress_detects_sparse_mapping_across_long_ranges() {
    let mapped_pages = [
        true, true, false, true, true, true, false, true, true, true, true, false,
    ];
    let writable_pages = [true; 12];
    let first = USER_SPACE_BOTTOM_INCLUSIVE;

    for start_idx in 0..mapped_pages.len() {
        let ptr = first + start_idx * PAGE_SIZE + 16;
        let len = PAGE_SIZE * 3;
        let valid = user_access_range_valid_with(ptr, len, UserAccessMode::Read, |page, mode| {
            access_ok_with_windows(page, mode, &mapped_pages, &writable_pages)
        });

        let mut expected = true;
        for idx in start_idx..core::cmp::min(start_idx + 4, mapped_pages.len()) {
            expected &= mapped_pages[idx];
        }
        if start_idx + 3 >= mapped_pages.len() {
            expected = false;
        }
        assert_eq!(valid, expected);
    }
}

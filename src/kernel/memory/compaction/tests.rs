use super::*;
use crate::hal::common::virt::{
    GOVERNOR_BIAS_AGGRESSIVE, GOVERNOR_BIAS_BALANCED, GOVERNOR_BIAS_RELAXED,
};

#[test_case]
fn compaction_max_migrations_tracks_latency_bias() {
    assert_eq!(
        governor_adjusted_max_migrations(8, GOVERNOR_BIAS_BALANCED),
        8
    );
    assert_eq!(
        governor_adjusted_max_migrations(8, GOVERNOR_BIAS_AGGRESSIVE),
        10
    );
    assert_eq!(
        governor_adjusted_max_migrations(8, GOVERNOR_BIAS_RELAXED),
        6
    );
}

#[test_case]
fn compaction_max_migrations_stays_nonzero() {
    assert_eq!(
        governor_adjusted_max_migrations(0, GOVERNOR_BIAS_BALANCED),
        1
    );
    assert_eq!(
        governor_adjusted_max_migrations(1, GOVERNOR_BIAS_RELAXED),
        1
    );
}

#[test_case]
fn compact_zone_respects_adjusted_migration_budget_bounds() {
    let mut zone = Zone::new(0, 1024, PageMobility::Movable);
    zone.free_list.push(0);
    zone.free_list.push(4096);

    let movable_pages = [
        MovablePage {
            phys_addr: 4096 * 700,
            owner_id: 1,
            virt_addr: 0x1000,
        },
        MovablePage {
            phys_addr: 4096 * 701,
            owner_id: 1,
            virt_addr: 0x2000,
        },
    ];

    let mut migrations = 0usize;
    let mut migrate = |_: usize, _: usize, _: usize, _: usize| {
        migrations += 1;
        true
    };

    let result = compact_zone(&mut zone, &movable_pages, &mut migrate, 1);
    assert!(result.pages_migrated >= 1);
}

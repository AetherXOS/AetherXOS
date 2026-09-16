# Core Pressure Snapshot ABI

Bu dokuman `GET_CORE_PRESSURE_SNAPSHOT` syscall sozlesmesini sabitler.

## Core Syscall Numarasi

- `GET_CORE_PRESSURE_SNAPSHOT = 56`
- `GET_LOTTERY_REPLAY_LATEST = 57`

## GET_CORE_PRESSURE_SNAPSHOT (nr=56)

Kullanici alani buffer'i `usize[18]` olmalidir.

Sozlesme (`CORE_PRESSURE_SNAPSHOT_WORDS = 18`):

1. `schema_version`
2. `online_cpus`
3. `runqueue_total`
4. `runqueue_max`
5. `runqueue_avg_milli`
6. `rt_starvation_alert` (0/1)
7. `rt_forced_reschedules`
8. `watchdog_stall_detections`
9. `net_queue_limit`
10. `net_rx_depth`
11. `net_tx_depth`
12. `net_saturation_percent`
13. `lb_imbalance_p50`
14. `lb_imbalance_p90`
15. `lb_imbalance_p99`
16. `lb_prefer_local_forced_moves`
17. `core_pressure_class` (0..3)
18. `scheduler_pressure_class` (0..3)

Sinif kodlari:

- `0`: Nominal
- `1`: Elevated
- `2`: High
- `3`: Critical

## GET_LOTTERY_REPLAY_LATEST (nr=57)

Kullanici alani buffer'i `usize[5]` olmalidir.

Sozlesme (`LOTTERY_REPLAY_LATEST_WORDS = 5`):

1. `seq`
2. `task_id`
3. `winner_ticket`
4. `total_tickets`
5. `rng_state`

`sched_lottery` kapaliysa veya replay olayi yoksa syscall `0` donebilir.

## Raporlama Araci

`scripts/core_pressure_report.py` raw word dump'u okunabilir rapora cevirir.

Ornek:

```bash
python scripts/core_pressure_report.py --words "2,8,10,4,1250,0,12,0,1024,40,20,3,2,5,8,0,1,1"
python scripts/core_pressure_report.py --words "2,8,10,4,1250,0,12,0,1024,40,20,3,2,5,8,0,1,1" --format json --out docs/reports/core_pressure_snapshot.json
```

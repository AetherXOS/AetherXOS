# Crash Artifact ABI

Bu dokuman P0 crash artifact sozlesmesini sabitler.

## Core Syscall Numaralari

- `GET_CRASH_REPORT = 54`
- `LIST_CRASH_EVENTS = 55`

## GET_CRASH_REPORT (nr=54)

Kullanici alani buffer'i `usize[10]` olmalidir.

Sozlesme (`CRASH_REPORT_WORDS = 10`):

1. `panic_count`
2. `last_panic_tick`
3. `last_reason_hash`
4. `watchdog_tick`
5. `watchdog_stalls`
6. `watchdog_hard_panics`
7. `startup_stage_transitions`
8. `startup_order_violations`
9. `crash_log_latest_seq`
10. `crash_log_latest_kind`

## LIST_CRASH_EVENTS (nr=55)

Kullanici alani buffer'i event bazli `usize[8 * N]` olmalidir.

Sozlesme (`CRASH_EVENT_WORDS = 8`), event layout:

1. `seq`
2. `kind`
3. `tick`
4. `cpu_id`
5. `task_id`
6. `reason_hash`
7. `aux0`
8. `aux1`

`kind` degerleri:

- `1`: panic
- `2`: soft watchdog stall
- `3`: hard watchdog stall
- `4`: driver quarantine

## Log Ciktisi

Kernel dump satirlari:

- `[KERNEL DUMP] panic_count=... last_panic_tick=... last_reason_hash=...`
- `[KERNEL DUMP] crash_event seq=... kind=... tick=... cpu=... task=... reason_hash=... aux0=... aux1=...`

Panic raporu:

- `PANIC report: count=... reason=... hash=...`

## Raporlama Araci

`scripts/crash_artifacts_report.py` log ve/veya raw syscall word dump'larini tek rapora cevirir.

Ornek:

```bash
python scripts/crash_artifacts_report.py --log kernel.log --format md
python scripts/crash_artifacts_report.py --log kernel.log --format json --out docs/reports/crash_report.json
python scripts/crash_artifacts_report.py --crash-report-words "1,100,0xabc,200,2,1,4,0,5,3"
python scripts/crash_artifacts_report.py --crash-events-words "5,3,200,0,42,0xabc,0,0,6,4,210,0,42,0xdef,7,1"
```

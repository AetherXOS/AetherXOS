# Linux Syscall Coverage Report Tool

`scripts/syscall_coverage_report.py` Linux syscall kapsama raporu uretir.

Kaynaklar:
- `src/modules/linux_compat/sys_dispatcher/**/*.rs`: `linux_nr::* -> handler` eslemeleri
- `src/modules/linux_compat/**/*.rs`: handler tanimlari/govdeleri

Siniflandirma:
- `implemented`: unsupported marker bulunmadi
- `partial`: `EOPNOTSUPP/ENOSYS` veya `TODO/mock/stub/no-op` marker bulundu
- `no`: `linux_nosys()` bulundu
- `external`: handler linux_compat disinda veya tanim bulunamadi

## Kullanim

```bash
python scripts/syscall_coverage_report.py --format md
python scripts/syscall_coverage_report.py --format json
python scripts/syscall_coverage_report.py --format md --out docs/reports/linux_syscall_coverage.md
```


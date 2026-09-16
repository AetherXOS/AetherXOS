# Config Refactor Plan (src/config.rs)

`src/config.rs` tek dosyada çok büyüdüğü için hedeflenen parçalama planı:

## Faz 1 (güvenli, düşük risk) - Durum: Tamamlandi

1. `src/config/profiles.rs` [x]
   - `NetworkRuntimeProfile`, `SchedulerRuntimeProfile`, `DriverNetworkRuntimeProfile`,
     `TelemetryRuntimeProfile`, `VfsRuntimeProfile`, `DevFsRuntimeProfile`
2. `src/config/parsers.rs` [x]
   - string -> enum parserları (`TlsPolicyProfile`, `BoundaryMode`, `DevFsPolicyProfile` vb.)
3. `src/config/overrides.rs` [x]
   - ortak clamp/override yardımcıları (`normalize_u16_override`, `normalize_u32_override`, bool/profile encode/decode)
4. `src/config/tests.rs` [x]
   - test modülünün ana dosyadan ayrılması

## Faz 2 (orta risk) - Durum: Tamamlandi

1. `src/config/vfs_devfs.rs` [x]
   - VFS + DevFS getter/setter/profile + reset blokları
2. `src/config/network.rs` [x]
   - network/libnet override ve profil blokları
3. `src/config/drivers.rs` [x]
   - driver wait/slo/budget override blokları

## Faz 3 (yüksek risk, kapsamlı test ister) - Durum: Devam Ediyor

1. reset/runtime profile bloklarını modül bazında bölmek [x]
   - `src/config/reset.rs`
   - `src/config/runtime_tuning.rs`
   - `src/config/scheduler.rs`
2. policy/telemetry/library feature bloklarını ayırmak [x]
   - `src/config/policy.rs`
3. testleri dosya bazında ayırmak [x]:
   - `src/config/tests/smoke.rs`
   - `src/config/tests/parsers.rs`
   - `src/config/tests/profiles.rs`
4. test alt modüllerini daha da domain-bazlı bölmek [ ]:
   - `src/config/tests/vfs_devfs.rs`
   - `src/config/tests/network.rs`
   - `src/config/tests/drivers.rs`

## Hedef

- `src/config.rs` sadece orchestrator + `KernelConfig` ana `impl` giriş noktası olsun.
- Modül sınırları netleşsin, magic-value ve tekrar alanları küçülsün.

# Kernel Test Harness Revizyon Plani (2026-05-14)

Kapsam: Kernel test harness'inin host ve kernel hedefleri arasinda tutarli calismasi, custom test framework uyumsuzluklarinin kapatilmasi ve test calistirma yolunun CI'ya alinarak kararlastirilmasi.

## 1. Bu turda tamamlanan kritik duzeltmeler

1. Custom test framework hizalandi
- `kernel/src/lib.rs`
- `kernel/src/main.rs`
- `#![feature(custom_test_frameworks)]`, `#![test_runner(...)]` ve `#![reexport_test_harness_main = "test_main"]` kullanimi korunup kernel test akisi ile uyumlu hale getirildi.

2. Plain `#[test]` testleri tek stile cekildi
- `kernel/src/core/log.rs`
- `kernel/src/core/log_filter.rs`
- `kernel/src/core/time.rs`
- `kernel/src/hal/abstractions.rs`
- `kernel/src/hal/acpi_parser.rs`
- Kalan `#[test]` kullanan kernel testleri `#[test_case]` stiline cevrildi; custom harness tek tip test koleksiyonu ile calisir hale getirildi.

3. Host test derlemesi ve calistirma dogrulandi
- Komut: `cargo test --lib --quiet`
- Sonuc: `EXIT:0`
- Host tarafinda kernel library testleri basariyla derlendi ve calisti.

## 2. Su anki mimari durum

- Test harness artik iki ayrik test stili uretmiyor; kernel kaynak agacinda tek uyumlu yol olarak custom framework + `#[test_case]` kullaniliyor.
- Host compile yolu kilitlenmiyor; test derlemesi artık `TestDescAndFn` / `Fn()` uyusmazligi uretmiyor.
- Calisan test seti simdilik host tarafinda dogrulandi; kernel hedefli boot testleri icin ayrica feature/boot akisi gerekir.

## 3. Guncel aciklar

### P0 - Kernel hedefi boot/test entegrasyonu

1. `x86_64-unknown-none` test calistirma yolu otomatik degil
- Kernel testleri host uzerinde compile oluyor, ama target-none boot ve execution yolu ayrica korunmali.
- Eylem: `kernel_test_mode` ile boot image uretimini ve QEMU/benzeri calistirmayi tek komut akisi altinda sertlestir.

2. Kernel test yuzeyi ile host test yuzeyi arasinda ayrim netlesmeli
- Eylem: host unit testleri ile target-none kernel testleri icin beklenen davranis dokumante edilmeli; boot-time testlerin hangi ortamda calistigi aciklanmali.

3. Test harness regresyonu icin CI kapisi eksik
- Eylem: `cargo test --lib --quiet` host dogrulamasi ve kernel hedefli no-run/boot check'ler CI'da farkli job'lar halinde tanimlanmali.

### P1 - Bakim ve gozlenebilirlik

1. Test kodu uyari gürultusu yuksek
- Eylem: mevcut unused import / unused variable uyari alani ayrica temizlenmeli veya ilgili test modullerinde bilincli olarak daraltilmali.

2. Kernel test dokumantasyonu daginik
- Eylem: tek bir hizli referans dosyasi ile host compile, target-none build ve boot execution adimlari birlestirilmeli.

## 4. Fazli uygulama plani

## Faz A (hemen): Host/kernel test ayriminin kalici hale getirilmesi

- `kernel_test_mode` ile kernel hedefli boot-test akisini netlestir.
- Host unit testleri icin `cargo test --lib --quiet` dogrulamasini standart kabul et.
- Kernel hedefi testleri icin `--target x86_64-unknown-none` no-run veya boot tabanli kapi belirle.

Exit kriteri:
- Host test derlemesi geciyor.
- Kernel target test akisi icin tek komutlu yol dokumante.

## Faz B (kisa vadede): Kernel test execution automation

- Boot image uretimi ve test calistirma adimlarini xtask altinda toplu komutlara bagla.
- Test sonucunu serial/JUnit benzeri cikisla raporla.
- Kernel test mode ile normal boot mode arasinda yapi farkini minimumda tut.

Exit kriteri:
- Host test, kernel target no-run ve boot smoke akisi birbirini bozmadan calisiyor.

## Faz C (orta vadede): CI sertlestirme

- Host compile + test job
- Kernel target compile/no-run job
- Kernel boot smoke job
- Gerekiyorsa feature matrisi ile test kapsam ayri ayrik tanimlanmali.

Exit kriteri:
- Test harness degisikligi bir sonraki CI turunda yakalanabiliyor.
- Test calistirma yolu tek seferde ve dokumante.

## 5. Not

Bu plan, runtime semantik revizyonlardan bagimsiz olarak kernel test harness'in calisir ve tekrar edilebilir hale gelmesini hedefler. Bugun icin kritik kisim, custom test framework ile test attribute stilinin tek duzene alinmasi ve host testlerinin basariyla gecmis olmasidir.

## 6. Yapılanlar (kisa not) ve sonraki adımlar

- **Bugün yapıldı:** `cargo fix --workspace` ile host/kitaplık uyarılarının otomatik onarılabilen kısmı uygulandı. (Örnek dosyalar: `kernel/src/modules/posix/process/pidfd.rs`, `kernel/src/kernel_runtime/service_integration.rs`, `kernel/src/hal/x86_64/paging.rs` vb.)
- **Deneme:** `--target x86_64-unknown-none` ile `--no-run` build denemeleri yapıldı; derleme aşamasında bağımlılık/core öğelerinin bulunmadığına dair hatalar alındı (ör. `serde_core` kaynaklı `Ok`/`Err`/`Result` referans eksiklikleri). Bu, hedef platform konfigürasyonunun (target spec / sysroot / linker / feature set) düzgün ayarlanmamış olmasından kaynaklanıyor.

Önerilen sonraki adımlar (kısa):

1. Hedef sistem yapılandırmasını doğrula: proje kökünde özel bir target JSON veya `.cargo/config.toml` içinde gerekli target tanımları olduğundan emin ol.
2. Eğer hedef bir "bare-metal" triple ise, build için kullanılan feature set'i kısıtlayarak bağımlılıkların `no_std` dostu olduğunu doğrula; örnek denemeler:

```powershell
cargo test --lib --no-run --target x86_64-unknown-none --features "kernel_test_mode"
cargo test --lib --no-run --target x86_64-unknown-none --no-default-features --features "kernel_test_mode"
```

3. `xtask` veya proje içi scriptlerle boot-image üretimini kullanarak QEMU ile smoke testi çalıştır; örnek (proje özelinde değişebilir):

```powershell
cargo run -p xtask -- build-boot --features "kernel_test_mode,linux_compat"
# sonra: qemu-system-x86_64 -drive file=out/boot.img,format=raw ...
```

4. CI entegrasyonu: host-derleme job'u ile target-no-run/boot job'larını ayrı görevler olarak tanımla; cross-build başarısız olduğunda jobları daha açıklayıcı hata ile geri döndürecek şekilde yapılandır.

Eğer isterseniz, ben şimdi devam edip `ARCH_REVISION_PLAN_2026_05_14_KERNEL_TEST_HARNESS.md` dosyasını bu maddelerle zenginleştirdim; sonraki adım olarak P0 cross-build blokajını çözmeye odaklanırım (önerilen eylem: target spec ve xtask tabanlı boot-image akışını kullanmak). 

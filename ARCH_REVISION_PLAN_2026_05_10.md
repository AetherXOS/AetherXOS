# AetherXOS Mimari Revizyon Master Plani

Tarih: 2026-05-10
Kapsam: Eksik ozellikler, eksik moduller, yarim kalmis yapilar, mock/stub kalan yollar, test ve release kapilari

## 1) Amac ve Basari Kriteri

Bu planin amaci, repodaki kismi/placeholder implementasyonlari production yolundan kaldirip sistemin dogrulugunu ve bakim kabiliyetini artirmaktir.

Bu plan tamamlandiginda:
- Runtime yolunda placeholder/stub/mock davranis kalmayacak.
- Feature-gated fallback yollari acikca ayrisacak ve raporlanacak.
- Mimari dokumanlar kodla birebir senkron olacak.
- Build/test/release kapilari otomatik ve olculebilir hale gelecek.

## 2) Gap Matrisi (Kanitli Alanlar)

### P0 - Runtime Dogruluk Aciklari
1. IPC syscall stublari
- Dosya: kernel/src/services/compat/ipc.rs
- Durum: sem/msg/shm/futex yuzeyinde sabit veya bos donusler.
- Etki: Linux uyumluluk semantigi ve process senkronizasyonunda dogruluk riski.

2. MMU page table walk mock
- Dosya: kernel/src/services/memory/memory/paging/table_walk.rs
- Durum: walk_to_pte() mock deger donuyor.
- Etki: bellek cevirimi, fault analizi ve koruma mekanizmalari acisindan yuksek risk.

3. VFS native path mock baglama
- Dosya: kernel/src/services/vfs/aether_fs.rs
- Durum: open() path cozumu yerine root inode donuslu mock yol.
- Etki: dosya izolasyonu, path semantics, guvenlik kontrollerinde tutarsizlik.

4. VFS syscall permission hook placeholder baglam
- Dosya: kernel/src/services/compat/syscalls/vfs/io_ops.rs
- Durum: inode/uid/gid placeholder degerlerle policy hook cagri.
- Etki: izin kontrolu semantiginde yanlis pozitif/negatif kararlar.

5. Socket lifecycle placeholder adresleme
- Dosya: kernel/src/services/compat/syscalls/linux_shim/net/socket/lifecycle.rs
- Durum: connect/accept policy kontrolunde placeholder remote adres.
- Etki: ag policy enforcement dogrulugu dusuk.

### P1 - HAL/Platform Gercekcilik Aciklari
1. CPU id placeholder
- Dosya: kernel/src/hal/platforms/x86_64_platform.rs
- Durum: get_cpu_id() placeholder donuyor.
- Etki: SMP tanilama ve telemetri dogrulugu zayif.

2. Firmware null provider fallback
- Dosya: kernel/src/hal/common/firmware_abstraction.rs
- Durum: null provider makul fallback olsa da hangi modda aktif oldugu net raporlanmiyor.
- Etki: platform kabiliyeti algisi ile runtime gercekligi arasinda fark.

### P2 - Bilincli Fallback/Feture-Gate Ayrimi
1. VFS stubs yalnizca feature kapali iken
- Dosya: kernel/src/services/vfs/vfs_control/stubs.rs
- Not: Bu teknik olarak dogru fallback. Ancak raporlamada runtime eksigi gibi gorunuyor.
- Ihtiyac: fallback sinifi, etiketleme ve CI raporlama.

## 3) Revizyon Fazlari

## Faz 0 - Freeze ve Envanter (2 gun)
Cikti:
- ACTIVE_PATHS.md
- FALLBACK_PATHS.md
- TEST_ONLY_PATHS.md
- FEATURE_MATRIX.md

Isler:
1. Tüm feature kombinasyonlari icin aktif kod yolunu cikart.
2. Fallback ve test-only modulleri siniflandir.
3. Placeholder/mocked runtime yollarini P0-P2 onceligiyle etiketle.

DoD:
- Her kritik modulin aktif/fallback/test-only statusu tek tabloda gorunur.

## Faz 1 - P0 Runtime Dogruluk Kapatma (1 hafta)
Cikti:
- Gercek IPC syscall semantigi (minimum uyumluluk seti)
- Gercek page table walk
- Gercek path resolution ve metadata temelli VFS policy baglami
- Socket policy kontrollerinde gercek peer adres/port baglami

Isler:
1. kernel/src/services/compat/ipc.rs
- sem/msg/shm/futex cagri yuzeyini cekirdek servislerine bagla.
- errno donuslerini Linux semantigine yaklastir.

2. kernel/src/services/memory/memory/paging/table_walk.rs
- mimariye uygun 4-seviye walk + valid bit ve access flag kontrolu.
- fault nedenini ayristiran hata modeli ekle.

3. kernel/src/services/vfs/aether_fs.rs
- open/stat/create/remove/mkdir akislarinda gercek inode path cozumu.

4. kernel/src/services/compat/syscalls/vfs/io_ops.rs
- placeholder inode/uid/gid kaldir.
- process credential + inode metadata ile policy hook baglantisi yap.

5. kernel/src/services/compat/syscalls/linux_shim/net/socket/lifecycle.rs
- sockaddr parse sonucu gercek remote endpoint ile policy kontrolleri.

DoD:
- Production yolunda placeholder return kalmaz.
- Unit + integration testleri P0 alanlarinda yesil.

## Faz 2 - HAL/Platform Guvenilirlik (1 hafta)
Cikti:
- CPU id dogru tespit yolu
- Firmware provider aktivasyon raporu

Isler:
1. kernel/src/hal/platforms/x86_64_platform.rs
- APIC/CPUID tabanli cpu index/id tespiti.

2. kernel/src/hal/common/firmware_abstraction.rs
- aktif provider turu (acpi/device-tree/null) runtime telemetriye yazilsin.

DoD:
- SMP test senaryolarinda cpu id tutarli.
- Firmware kaynak secimi logs/metrics uzerinden izlenebilir.

## Faz 3 - Fallback Kontrati ve CI Kapilari (3-4 gun)
Cikti:
- FALLBACK_POLICY.md
- CI placeholder scan gate
- Feature matrix CI artifact

Isler:
1. Fallback modulleri acik isimlendirme ve belgeleme.
2. Production build yolunda placeholder/mock/stub tarama gate.
3. Feature kombinasyonlari icin artifact raporlama.

DoD:
- Yanlis pozitifler whitelist disinda sifir.
- CI artifact her PR'da uretiliyor.

## Faz 4 - Domain Parity ve Sertlestirme (2 hafta)
Cikti:
- Syscall parity raporu
- VFS/net/process regression suite

Isler:
1. Linux compat syscall parity tablosu (ipc, fs, net odakli).
2. VFS permission + ownership + path traversal negatif testleri.
3. Socket policy ve lifecycle stres testleri.

DoD:
- Kritik syscall setinde stub path sifir.
- Negatif guvenlik testleri yesil.

## Faz 5 - Dokuman Senkronu ve Release Gate (3 gun)
Cikti:
- ARCHITECTURE.md guncel
- Domain README guncel
- RELEASE_READINESS_CHECKLIST.md

Isler:
1. Mimari iddialari kod tabanli rapora bagla.
2. Dokuman drift kontrolunu CI adimi yap.

DoD:
- Dokuman iddialari ile code scan uyumsuzlugu yok.

## 4) Validation ve Exit Kriterleri (Zorunlu)

Her merge icin minimum set:
1. cargo build
2. cargo test
3. Cargo feature matrisinde secili kombinasyonlar:
- default
- --features vfs
- --features posix_net
- --features "vfs posix_net"

4. Placeholder gate:
- Production aktif dosyalarda asagidaki kaliplar yasak:
  - "Mock"
  - "Placeholder"
  - "stub"
  - "todo!"
  - "unimplemented!"

5. Syscall parity gate:
- P0 syscall listesinde "always return 0" veya "Ok(()) sabit" yolu kalmayacak.

6. Security regression:
- VFS izin/ownership testleri
- socket policy deny/allow testleri

7. Architecture drift gate:
- Mimari rapor hash ve dokuman hash eslesmesi.

## 5) Riskler ve Azaltim

1. Risk: Faz 1'de degisiklikler genis etki olusturur.
- Azaltim: P0 dosyalari adim adim, her adimdan sonra test.

2. Risk: Feature kombinasyon patlamasi.
- Azaltim: Kritik kombinasyon matrisi + gece calisan genis matris.

3. Risk: False positive placeholder scan.
- Azaltim: test-only ve fallback whitelist politikasi.

## 6) Uygulama Sirasi (Operasyonel)

Sprint 1:
- Faz 0 + Faz 1'in IPC ve table_walk bolumu

Sprint 2:
- Faz 1'in VFS ve socket policy bolumu + Faz 2

Sprint 3:
- Faz 3 + Faz 4

Sprint 4:
- Faz 5 + release hardening

## 7) Hemen Baslanacak Task Paketi (Ilk 10 Gorev)

1. table_walk icin gercek traversal taslagi ve test fixture
2. ipc.rs syscall mapping tablosu
3. futex wait/wake cekirdek senkronizasyon baglantisi
4. aether_fs open path resolution
5. io_ops permission hook context gercekleme
6. socket lifecycle peer extraction
7. x86_64 cpu id implementation
8. firmware provider telemetry eventi
9. placeholder CI scanner script
10. feature matrix raporlayici

## 8) Kapanis

Bu planin hedefi sadece "derleniyor" seviyesini degil, runtime semantigi ve mimari tutarliligi da garanti etmektir. Basari olcumu build/test yesili ile sinirli degil; placeholder-free production yolu, parity raporu ve dokuman-code senkronu birlikte saglanmalidir.

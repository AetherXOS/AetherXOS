Kernel Durum Değerlendirmesi ve Yol Haritası
=========================================

Özet
-----
Bu döküman, çalışma alanındaki kernel kod tabanının mevcut durumu, eksikleri, öncelikli geliştirmeler, refactor önerileri ve kısa/orta vadeli uygulama adımlarını içerir. Hedef: daha modüler, test edilebilir, güvenli ve sürdürülebilir bir kernel kod tabanı sağlamak.

Bulunan Ana Dosyalar (örnek)
----------------------------
- src/kernel/api.rs
- src/kernel/cpu_local.rs
- src/kernel/dynamic_linker.rs
- src/kernel/elf_dynamic.rs (çok büyük, ~800+ satır tespit edildi)
- src/kernel/interrupt_guard.rs
- src/kernel/launch.rs (büyük; launch ile init mantığı yoğun)
- src/kernel/load_balance.rs
- src/kernel/log.rs
- src/kernel/memory/manager.rs

1) Durum Analizi
-----------------
- Kod tabanı Rust ile yazılmış; Cargo temelli build ve xtask yardımcı araçları mevcut.
- Bazı dosyalar çok büyük ve tek sorumluluk ilkesini (SRP) zayıflatıyor: özellikle `elf_dynamic.rs` ve `launch.rs`.
- Test altyapısı ve otomatik CI konfigürasyonuna yönelik net bir görünürlük yok (workspace kökünde Cargo.toml var ancak repo içindeki CI dosyaları açıkça listelenmedi).
- Bellek yönetimi ve unsafe blokları muhtemel kritik noktalar; `memory/manager.rs` incelenmeli.
- Dinamik bağlayıcı/ELF dinamik yüklemeye dair karmaşık mantık var — modülerleştirme ve birim testleri için parçalanmalı.
- Logging ve observability temel seviyede; daha merkezi ve konfigüre edilebilir log katmanı faydalı olur.

2) Eksikler ve Öneriler (Yüksek Seviyeli)
---------------------------------------
- Modülerleştirme: Büyük dosyalar parçalanmalı (parser, resolver, loader gibi alt-modüller).
- Test kapsamı: Birim testleri ve entegrasyon testleri eklenmeli. ELF parsing ve küçük memory manager fonksiyonları için unit testler yazılmalı.
- CI: `cargo fmt`, `cargo clippy`, `cargo test` otomatik koşulsun (GitHub Actions / Azure Pipelines / GitLab CI önerilir).
- Güvenlik ve Bellek Denetimi: Unsafe kullanım yerleri listelenip kapsüllenerek güvenlik kontrolleri eklenmeli.
- Sürücü soyutlama: driver interface'leri standartlaştırılmalı ve mockable olsun.
- Observability: Log seviyeleri, structured logging, ve kernel başlangıç sırasında debug/trace toggles eklenmeli.
- Performans: kritik path'lerde mikro-benchmark'lar planlanmalı (ör. context switch, interrupt path).

3) Refactor Önerileri (Dosya Bazlı)
----------------------------------
- `src/kernel/elf_dynamic.rs`:
  - Ayır: `elf_dynamic/reader.rs`, `elf_dynamic/resolver.rs`, `elf_dynamic/bindings.rs`.
  - Fonksiyonları küçük, test edilebilir parçalara ayır.
- `src/kernel/launch.rs`:
  - Boot/init mantığını `init.rs` ve `service_start.rs` gibi parçalara böl.
  - Platform bağımlı kodlar için `arch/` alt modüllerine daha net sınırlar koy.
- `src/kernel/dynamic_linker.rs`:
  - API yüzeyi sadeleştirilsin; yükleme/runtıme ayırımı net olsun.
- `src/kernel/memory/manager.rs`:
  - Güvenli sarmalayıcılar oluştur (RAII benzeri) ve unsafe blokları minimize et.

4) Önceliklendirme (Kısa/Orta/Uzun Vadeli)
------------------------------------------
- Kısa (1-2 hafta): Kod tabanı analiz raporu, `cargo fmt` ve `clippy` entegrasyonu, küçük refactorlar (dosya bölümleri), temel birim testleri.
- Orta (2-8 hafta): `elf_dynamic` modülerleştirmesi, `launch.rs`'in parçalanması, test kapsamının artırılması, CI boru hattı.
- Uzun (2-6 ay): Sürücü framework refactoru, bellek güvenliği sertifikasyonına hazırlık, performans optimizasyonu ve geniş entegrasyon testleri.

5) İlk Adımlar (Yapılacaklar — somut)
------------------------------------
1. `Kod tabanı analizi`: tüm `src/kernel` dosyalarının boyut/complexity belirlenmesi ve unsafe kullanımlarının listelenmesi.
2. `Dokümantasyon`: `src/kernel/README.md` güncellensin; modüller ve sorumluluklar belirlensin.
3. `Style & Lint`: Repo'ya `cargo fmt` ve `clippy` workflow ekle (CI).
4. `Büyük dosyaları böl`: `elf_dynamic.rs` ve `launch.rs`'in ana hatlarını çıkarıp küçük modüllere böl.
5. `Test altyapısı`: ELF parser ve memory manager için unit test iskeleti ekle.

6) Riskler ve Dikkat Edilmesi Gerekenler
---------------------------------------
- Kernel kodunu değiştirirken boot zincirini bozmamaya dikkat edin; cross-derleme hedefleri ve toolchain gereksinimleri kritik.
- Unsafe ve bellek yolları önce analiz edilmeli; refactor sırasında davranış değişikliği yapmamak için kapsamlı testler gerekir.

7) Kaynak Dosya Referansları (öncelikli inceleme)
-----------------------------------------------
- [src/kernel/api.rs](src/kernel/api.rs)
- [src/kernel/launch.rs](src/kernel/launch.rs)
- [src/kernel/elf_dynamic.rs](src/kernel/elf_dynamic.rs)
- [src/kernel/dynamic_linker.rs](src/kernel/dynamic_linker.rs)
- [src/kernel/memory/manager.rs](src/kernel/memory/manager.rs)

8) Önerilen iş bölümü (örnek)
-----------------------------
- Adım A (analiz): dosya boyut/unsafe taraması — 1 gün
- Adım B (stil/CI): fmt+clippy+test workflow — 1 gün
- Adım C (modülerizasyon): `elf_dynamic` parçalama — 3-5 gün
- Adım D (testler): birim testleri ve küçük entegrasyon — 3 gün

Sonraki adım için onay isteği
----------------------------
Bu planı dosyaya kaydettim. Onay verirseniz önce "Kod tabanı analizi" adımını başlatıp unsafe ve büyük dosya taraması yapacağım; ardından `elf_dynamic` için parçalara ayırma planını uygulamaya başlayacağım.

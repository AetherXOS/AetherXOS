<!-- Auto-generated detailed 4-year roadmap for AetherCore. Written in Turkish. -->
# AetherCore — 4 Yıllık Gelişim Yol Haritası (Detaylı)

Son güncelleme: 2026-02-28

Bu belge AetherCore projesinin mevcut durumu, öncelikleri, zaman çizelgesi ve 4 yıllık (Yıl 1–4) gelişim yol haritasını kapsamlı biçimde açıklar. İçerik, depo `README.md`, `src/hal/README.md` ve `ci/README.md` gibi ana belgeleri harmanlayarak hazırlanmıştır.

**Kısa Özet (1 cümle):** AetherCore, mekanizma-odaklı (Exokernel) mimariyle yüksek modülerlik, performans ve güvenlik hedefleyen bir işletim sistemi çekirdeğidir; bu yol haritası projeyi üretim düzeyine taşımak için gereken adımları, kilometre taşlarını, riskleri ve ölçülebilir başarı kriterlerini detaylandırır.

**İçindekiler**
- **Genel bakış ve vizyon**
- **Mevcut durum (özet)**
- **Yıllara göre hedefler ve kilometre taşları**
- **Modül-bazlı görev dağılımı**
- **Sprint & sürüm stratejisi**
- **Test/CI/Fuzz/Benchmark planı**
- **Dokümantasyon ve geliştirici deneyimi (DX)**
- **Riskler, bağımlılıklar ve azaltma planı**
- **KPI'lar ve kabul kriterleri**
- **İlk 30/90/180 günlük yol haritası**

**1. Genel Bakış ve Vizyon**

Amaç: AetherCore'u 4 yıl içinde kararlı, modüler ve güvenli bir Exokernel + LibraryOS platformuna dönüştürmek. Hedefler:
- Mekanizmaları güvenli, minimal, izole tutmak (Core/Library sınırını sıkı koru).
- Performans: düşük latency scheduling ve yüksek I/O throughput (NVMe/NIC) hedefleri.
- Modülerlik: Scheduler, Allocator, VFS, Net, Security vs. replaceable library olarak sunulacak.
- Test edilebilirlik: tam CI, QEMU entegrasyonları, fuzzing hedefleri ve benchmarklar.

**2. Mevcut Durum (konsolidasyon)**
- Core/Kernel: Boot, GDT/IDT, context-switch, temel scheduler ve slab allocator: stabil.
- HAL: `x86_64` hatasız; `AArch64` scaffolding aşamasında.
- VFS: temel VFS ve birden fazla backend (RAM/FAT/Little/Ext4-view read-only) mevcut.
- Network: kontrol-plane, smoltcp entegrasyonu; dataplane bazı donanım yolculuklarında eksiklikler.
- Sürücüler: NVMe, VirtIO, E1000 yolu eklenmiş ama dataplane parity & performans çalışmaları devam ediyor.
- Güvenlik: syscall boundary hardening, IOMMU/VT-d discovery ve bazı formal doğrulamalar var.
- CI: `ci/` notları ve scriptler mevcut; QEMU emülasyon betikleri var fakat disk-image otomasyonu eksik.

Kaynaklar & referanslar: kök `README.md`, `src/hal/README.md`, `ci/README.md`, her modülün README'leri (ör: `src/modules/*/README.md`).

**3. 4 Yıllık Yol Haritası (Yıl bazında, çeyreklere bölünmüş)**

Not: Her çeyrek iki haftalık sprintler halinde uygulanacak; her sprint sonunda otomatik CI + entegre testler çalışır.

Yıl 1 — Stabilizasyon & Foundation
- Q1 (Başlangıç—3 ay)
  - Hedef: Tam yol haritası onayı, mimari dokümanlar (`ARCHITECTURE.md`), CI temel hattı.
  - Teslimatlar: `ROADMAP/ROADMAP.md` (bu dosya), `ARCHITECTURE.md` taslağı, `CONTRIBUTING.md` güncellemesi.
  - Teknik iş: QEMU disk-image builder eklenecek; `scripts/setup/setup_agent_env.sh` genişletilecek.

- Q2
  - Hedef: Kernel MVP (preemptive scheduler stabilizasyonu), IPC doğru testleri.
  - Teslimatlar: Kernel-upgrade test harness, scheduler RT/Batch politikleri.
  - Teknik iş: syscall helper consolidation ve mapping-aware copyin/copyout genişletmeleri.

- Q3
  - Hedef: Bellek yönetimi ileri (buddy/slab refinements), per-CPU allocator refactor.
  - Teslimatlar: Slab refill + refill-fallback testleri, MMU integration smoke tests.

- Q4
  - Hedef: Sürücü lifecycle ve NVMe baseline dataplane (QEMU NVMe testleri).
  - Teslimatlar: NVMe integration tests, driver trait unification (`probe/init/io/teardown/health`).

Yıl 2 — Dataplane & Library Productization
- Q1
  - Hedef: Network dataplane parity (VirtIO/E1000 RX/TX) ve libnet service templates.
  - Teslimatlar: transport facade, UDP/TCP baseline, LibNet service examples.

- Q2
  - Hedef: VFS productization: write-paths, journaling experiment, ext4 read-write pilot.
  - Teslimatlar: VFS caching + metadata journaling prototype, mount/umount policy tests.

- Q3
  - Hedef: Security hardening: capability revocation, ACL lifecycles, threat-modeling.
  - Teslimatlar: Threat model doc, fuzz targets for syscall/VFS/IPC.

- Q4
  - Hedef: Multi-arch parity push (AArch64 baseline runnable on QEMU virt + smoke tests).
  - Teslimatlar: AArch64 interrupt/mmu/timer baseline, cross-arch smoke suite.

Yıl 3 — Scaling & Optimization
- Q1–Q2
  - Hedef: Performance engineering — scheduler latency reduction, allocator reclaim classes, bridge fast-path tuning.
  - Teslimatlar: microbench harnesses (no_std-friendly), production telemetry dashboards.

- Q3–Q4
  - Hedef: Reliability — long-running stability tests, watchdog improvements, live-upgrade experiments.
  - Teslimatlar: 7-day uptime target tests, crash-free commit metrics.

Yıl 4 — Production Readiness & Ecosystem
- Q1–Q2
  - Hedef: Production features gap closure — full POSIX compatibility surface, TLS policy profiles, ops playbooks.
  - Teslimatlar: Tuning playbooks, runbooks, packaged images for common platforms.

- Q3–Q4
  - Hedef: GA (v1.0) hazırlığı, dış denetimler, genişleme planı (container hosting, unikernel images).
  - Teslimatlar: GA release, upgrade notes, third-party audit raporu.

**4. Modül-Bazlı Görev Dağılımı (Detaylı)**

- Kernel (`src/kernel`)
  - Görevler: scheduler policy çeşitliliği (CFS/MLFQ/RT), preemptive path testleri, task lifecycle. Tahmin: 6–10 sprint.
  - KPI: context-switch latency p50/p95 hedefleri.

- HAL (`src/hal`)
  - Görevler: ACPI/IOAPIC derinleştirme, IOMMU tam kapsam, AArch64 parity.
  - KPI: cross-arch smoke suite geçiş oranı.

- Sürücüler (`src/modules/drivers`)
  - Görevler: NVMe dataplane optimizasyonu, NIC RX/TX parity, driver lifecycle trait tamamlanması.
  - Test: QEMU NVMe emülasyonu + gerekirse fiziksel test matrix.

- VFS (`src/modules/vfs`)
  - Görevler: journaling prototipi, writeback stratejileri, cache invalidation, fsck araçları.

- Ağ & LibNet (`src/modules/network`, `src/modules/libnet`)
  - Görevler: transport facade, adaptive poll profiles, libnet templates (UDP/TCP/HTTP), TLS profiles.

- Güvenlik (`src/modules/security`)
  - Görevler: capability/ACL revocation, sandboxing/namespace-lite, secure boot recommendations.

**5. Sprint & Sürüm Stratejisi**

- Sprint: 2 hafta.
- Release wave: 3 sprints (6 hafta) per wave; aylık stabilite kapısı.
- Done definition: feature flag wired + telemetry + tests + README update.

Örnek sprint backlog maddesi (NVMe dataplane sprint):
1. Review `nvme.rs` code paths for error handling (2d)
2. Add submission/completion queue unit tests (3d)
3. Add QEMU-based NVMe integration test harness (2d)
4. Benchmark basic sequential/random IO and record baseline (3d)
5. Merge & CI run (1d)

**6. Test / CI / Fuzz / Benchmark Planı (ölçülebilir)**

- CI matrix: Host unit tests, cross-compile, QEMU boot & integration, hardware smoke (optional runners).
- Fuzzing targets (priority): syscall parsers, VFS path handling, IPC ring buffer, driver command parsers.
- Benchmarks:
  - Scheduling latency harness (context switch microbench)
  - NVMe sequential/random throughput
  - Network throughput/latency (1G baseline)
- Acceptance: `cargo check --tests` + QEMU boot + selected integration tests green on PR.

**7. Dokümantasyon & Geliştirici Onboarding**

- Dokümanlar toplanacak/güncellenecek:
  - `ARCHITECTURE.md` (modül sorumlulukları, public traitler, concurrency model)
  - `DEVELOPMENT.md` (local build, QEMU runs, debugging)
  - `RELEASE.md` (wave çıkış kriterleri, playbook)
- Onboarding: 1-day recorded workshop, example `hello-world` userland, driver dev walkthrough.

**8. Riskler & Azaltma Planları (Önceliklendirilmiş)**

- Donanım uyumluluğu (yüksek)
  - Azaltma: QEMU geniş matrix, erken gerçek-hardware doğrulama, sürücü adaptasyon kılavuzu.
- Scope creep / takvim gecikmeleri (yüksek)
  - Azaltma: MVP tanımı, sprint bazlı gözden geçirme, roadmap freeze per wave.
- Kernel güvenlik açıkları (kritik)
  - Azaltma: fuzzing, dış denetim (yıl 3 planlanmış), formal model hedefli kritik fonksiyonlar.

**9. KPI'lar ve Başarı Ölçütleri**

- Stabilite: Alpha → Beta → RC → GA aşamalarında uptime hedefleri (örn. RC için 7 gün, GA için 30 gün hedefi).
- Performans: Context-switch latency p50/p95; NVMe throughput hedefleri (eşikler sprint bazında belirlenir).
- Test kapsama: PR başına CI geçme oranı %95+; kritik fuzz hedefleri için minimum issue eşik.

**10. Operasyonel Öneriler ve Araçlar**

- CI runner önerisi: Linux self-hosted runner + QEMU ve hardware runners (NVMe/NIC) için özel etiket.
- Telemetry: runtime summary + per-module counters (vfs, net, scheduler, drivers) varsayılan.
- Release packaging: ISO/EFI stub + minimal initramfs + example library OS images.

**11. 30 / 90 / 180 Günlük Öncelikler (Başlangıç için)**

- İlk 30 gün
  - `ARCHITECTURE.md` taslağını tamamla.
  - CI: QEMU image builder & basic integration job ekle.
  - `nvme.rs` için öncelikli hata-path incelemesi ve küçük entegrasyon testi ekle.

- İlk 90 gün
  - Kernel MVP stabilizasyonu (scheduler + MMU hardening).
  - NVMe dataplane baseline testleri (QEMU).
  - Fuzz harness: syscall/VFS/IPC scaffolding.

- İlk 180 gün
  - Network dataplane parity (VirtIO/E1000) tamamla.
  - VFS write-path + journaling POC.
  - AArch64 smoke baseline oluştur.

**12. Sonuç ve Sonraki Adımlar**

Bu belge proje için yol haritası, öncelikler ve açık görevlerin kapsamlı listesini sunar. Önerilen hemen yapılacaklar:
1. `ARCHITECTURE.md` oluşturulması (öncelik: 1 hafta).
2. CI’ye QEMU disk-image builder ekleme (öncelik: 2 hafta).
3. `nvme.rs` detaylı statik inceleme ve küçük entegrasyon testi ekleme (öncelik: 2 hafta).

İstersen bu dosyayı sprint backlog’larına bölüp her görev için tahmini insan-gün ataması yapabilirim; veya her modül için 2 haftalık sprint backlog'larını otomatik oluştururum.

---

Bu dosya otomatik olarak repo içinde `ROADMAP/ROADMAP.md` olarak eklendi. Değişiklik istersen hangi formatta görmek istediğini (Gantt, CSV, Issues listesi vs.) söyle; ona göre dönüştürüp eklerim.

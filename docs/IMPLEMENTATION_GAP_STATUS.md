# AetherCore Gap Status (2026-03-06, sprint 2 update)

Bu tablo, gönderdiğin Linux/Windows-benzeri tam çalışma listesine göre yaklaşık tamamlanma oranını verir.
Yüzdeler kod tabanı ve mevcut pipeline/gate durumuna göre mühendislik tahminidir.

## Sprint İlerlemesi (Son Oturum)

| Tamamlanan | Önceki | Sonraki |
|---|---:|---:|
| VirtIO block descriptor ring — gerçek I/O okuma/yazma | 45% | 75% |
| VFS Journal WAL (JBD2-style, commit, recover) | 30% | 80% |
| Priority-Inheritance Mutex (boost + restore, RAII guard) | 0% | 100% |
| Power MSR frekans denetimi (IA32_PERF_CTL wrmsr, P-state geçişleri) | 0% | 100% |
| Init/Service Manager (5 kernel service, TID 0x8000+, idempotent) | 45% | 85% |
| VirtIO net + E1000 — gerçek TX/RX halka doğrulandı | 50% | 100% |
| HTTP/1.1 outbound client (builder, parse, chunked decode) | 0% | 100% |
| Namespace unshare wiring (Linux unshare(2) + clone NEW* propagation) | 10% | 70% |

## Özet

- Sprint öncesi: **%52**
- **Güncel: ~%76**
- Kalan: **~%28**

## Kalem Bazlı Durum

| Alan | Durum | Yaklaşık Tamamlanma | Kalan |
|---|---|---:|---:|
| Minimal bootable PoC (boot image + initramfs + qemu smoke) | aktif pipeline var, ISO path opsiyonel | 75% | 25% |
| Userspace runtime (libc/loader/POSIX runtime) | POSIX katmanı var, libc/loader bütünlüğü eksik | 35% | 65% |
| Linux syscall ABI uyumluluğu | coverage/gate çok güçlü, semantik derinlik eksik | 70% | 30% |
| Block + filesystem (writable production rootfs) | **VirtIO block + WAL tamamlandı**, üst-katman FS eksik | 75% | 25% |
| Devfs/udev benzeri dinamik node + hotplug | gerçek devfs mount + event queue + policy config var | 70% | 30% |
| Driver dataplane (virtio/e1000/nvme/ahci production depth) | **VirtIO net + E1000 tam dataplane doğrulandı** | 85% | 15% |
| Userland paketleri + init + installer | erken initramfs var, dağıtım/installer yok | 20% | 80% |
| System services/policy tooling | **service manager + journal + watchdog tamamlandı** | 80% | 20% |
| Security namespaces/cgroups/MAC entegrasyonu | **PI Mutex + unshare namespace wiring eklendi**, setns/cgroup enforcement eksik | 60% | 40% |
| Power / CPU frekans yönetimi | **IA32_PERF_CTL wrmsr + P-state tamamlandı** | 90% | 10% |
| Network stack derinliği | **HTTP client + driver dataplane doğrulandı** | 85% | 15% |
| Toolchain + reproducible build | preflight/pipelines iyi, tam reproducibility/signed release eksik | 60% | 40% |
| Test coverage/regression (soak/stress/chaos/boot matrix/fuzz) | güçlü gate ve matrix var, syscall fuzz + uzun soak eksik | 65% | 35% |

## P0 / P1 / P2 Görünümü

- P0 (kritik boot + build + syscall gate + temel recovery): **%88**
- P1 (operasyonel olgunluk + POSIX/driver derinliği): **%75**
- P2 (ürünleşme: installer, namespace/cgroup, signing): **%35**

## Kalan Yüksek Öncelikli İşler

1. Writable production rootfs (ext4/journaling veya eşdeğeri) + güvenli mount/umount lifecycle.
2. Userspace runtime bütünlüğü: libc/loader zinciri ve process model semantiklerinin sertleştirilmesi.
3. Namespace/cgroup temelinin çekirdeğe alınması (özellikle process/fs görünümü izolasyonu).
4. Uzun süreli soak + fault injection + syscall fuzz’ın CI zorunlu kapısı haline getirilmesi.
5. Release artifacts için reproducible + imzalı paket/ISO hattı.

## Değişiklik Geçmişi

| Tarih | Konu |
|---|---|
| 2026-03-06 (sprint 1) | İlk tablo oluşturuldu (~%52) |
| 2026-03-06 (sprint 2) | VirtIO I/O, WAL, PI Mutex, MSR, service mgr, HTTP client — ~%72 |
| 2026-03-12 (sprint 3) | Linux unshare(2) namespace registry wiring + clone NEW* propagation — ~%76 |

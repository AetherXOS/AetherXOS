# AetherCore Production OS TODO (2026-03-30)

Bu liste, sistemi RAMFS demo modundan cikip production benzeri Linux/desktop workload calistirabilen seviyeye getirmek icin olusturuldu.

## A. Rootfs ve Kalici Dosya Sistemi

- [x] Diskfs bootstrap scripti (`aethercore-diskfs-setup`) eklendi.
- [x] Kernel tarafinda diskfs mount syscall ABI (`VFS_MOUNT_DISKFS`) eklendi.
- [x] `pivot_root`/`switch_root` benzeri initramfs->diskfs gecisi implement et.
- [x] Ext4 icin crash-recovery e2e testi (dirty journal + reboot + fsck benzeri dogrulama) ekle.
- [ ] Mount policy: read-only fallback, degraded mode ve telemetry alanlarini netlestir.
- [ ] Overlay upper/lower lifecycle (diskfs lower + tmpfs upper) senaryolarini gate'e bagla.

## B. Package Manager Runtime (APT/Pacman)

- [x] Userspace ABI preflight script ve contract dosyasi eklendi.
- [x] Seeded package manager capability manifest (`apt-seed-capability.json`) eklendi.
- [x] Non-Unix build host limiti icin acik not dosyasi eklendi.
- [ ] Linux host tabanli apt closure seeding pipeline (apt-get + dpkg + ld.so + gerekli .so) zorunlu hale getir. (pipeline scripti eklendi, CI zorunlulugu beklemede)
- [x] Offline cache mirror modu (air-gapped) ve imza dogrulama raporunu CI artifact'i yap.
- [ ] Package install transaction rollback semantigini bozuk paket senaryolariyla test et.

## C. ELF/.so Loader ve Userspace ABI

- [x] Dynamic linker / shared object kaynak kontrat probe'u validator'a eklendi.
- [x] DT_RUNPATH/DT_RPATH precedence davranisini Linux ile birebir uyumlu hale getir.
- [x] SONAME + symbol versioning (VERDEF/VERNEED) mismatch hata semantigini sertlestir.
- [ ] Lazy binding / PLT relocation davranisi ve audit hook altyapisini tanimla.
- [ ] `ld-linux` path ve multi-arch lib arama politikasini config-surface'e ac.
- [ ] `dlopen/dlsym/dlclose` hata kodlari ve edge-case semantikleri icin regression seti yaz.

## D. Syscall Semantics ve ABI Guclendirme

- [ ] Poll/select/epoll timeout, wakeup ve signal interruption semantiklerini Linux parity testine bagla.
- [ ] `clone/fork/execve/wait*` kombinasyonlari icin process lifecycle matrix testleri ekle.
- [ ] `mmap/mprotect/munmap` + file-backed mapping + COW davranislarini dogrula.
- [ ] `ioctl` coverage envanteri cikar, Flutter/graphics kritik ioctl'leri P0 listesine al.
- [ ] Syscall contract drift denetimini release gate'e bagla.

## E. Flutter ve Desktop Stack

- [x] Flutter runtime dependency closure envanteri olustur (`libflutter.so` + transitif .so listesi).
- [ ] Wayland ve X11 object lifecycle implementasyonu (handshake disinda) tamamla.
- [ ] Software renderer fallback'i production policy ile finalize et.
- [ ] GPU driver health probe'u runtime smoke testine bagla.
- [ ] Input/audio/windowing event pathlerini Flutter app smoke testleriyle dogrula.

## F. Driver ve Donanim Katmani

- [x] VirtIO-GPU / DRM benzeri userspace API surface haritasini cikart.
- [ ] NVMe/AHCI hata enjeksiyonlu soak testleriyle veri tutarliligini dogrula.
- [ ] Network dataplane uzun sureli soak + packet loss/jitter profillerini CI'ya bagla.
- [ ] MSI/MSI-X, interrupt affinity ve NUMA-aware scheduling etkisini olc.

## G. CI/CD ve Release

- [ ] Linux-host zorunlu nightly hattinda `apt-iso` + qemu userspace install proof calistir. (pipeline scripti eklendi, scheduler/CI entegrasyonu beklemede)
- [x] QEMU log parser'a `hyper_init`, `aethercore-apt-seed`, `abi-preflight` marker zorunlulugu ekle.
- [x] Signed/reproducible ISO artifact dogrulamasini release kriteri yap.
- [x] Production readiness scorecard: fs stack + package stack + desktop stack + syscall parity tek raporda birlestir.

## Baslanan Isler (Bu Oturum)

- [x] Uzun vadeli production TODO listesi olusturuldu.
- [x] APT seed capability manifest kodlandi.
- [x] Linux app compat validator'a .so runtime kontrat probe'u eklendi.
- [x] Derleme ve validator calistirma sonucu raporlandi.
- [x] Diskfs pivot-root setup scripti ve hyper_init entegrasyonu eklendi.
- [x] Filesystem stack gate'ine ext4 journal recovery evidence denetimi eklendi.
- [x] Package stack gate'ine mirror/retry/signature trust-chain denetimleri eklendi.
- [x] Flutter runtime closure audit manifesti eklendi.
- [x] Production release acceptance scorecard raporu eklendi.
- [x] Linux-host e2e proof pipeline scripti ve dokumantasyonu eklendi.
- [x] GPU ioctl P0 coverage envanteri release-gate girdisi olarak eklendi.

# AetherCore OS Detailed Roadmap (2026-03-14)

Bu dokuman, kod tabaninin mevcut durumunu "tamamlilik" acisindan netlestirir ve bundan sonra neyin, hangi sirayla, hangi kabul kriteriyle yapilacagini listeler.

## 1) Guncel Teknik Snapshot

- Build durumu: `cargo check -q` geciyor.
- Syscall coverage (default): 258/258 implemented (%100) — kaynak: `reports/syscall_coverage_summary.json`.
- Syscall coverage (`linux_compat`): 257/257 implemented (%100) — kaynak: `reports/syscall_coverage_linux_compat_summary.json`.
- Not: Coverage %100 olmasi semantik parity'nin %100 oldugu anlamina gelmez; bircok syscall "minimum dogru davranis" ile implemented olabilir.

## 2) Bu Oturumda Eklenen Duzeltmeler

### 2.1 Linux poll/select/epoll semantik sertlestirme

Dosya: `src/modules/linux_compat/net/poll.rs`

- `ppoll` timeout argumani `LinuxTimespec` olarak dogrulandi.
- `ppoll`, `epoll_pwait`, `pselect6` icin `sigsetsize`/descriptor dogrulama eklendi.
- Gecici signal mask apply/restore semantigi eklendi.
- `pselect6` hata path'inde eski sigmask restore edilmeden cikma riski giderildi.
- `poll` timeout->retry cevirimi daha tutarli hale getirildi.

### 2.2 Guvenli warning temizligi

- `build_cfg/validators/mod.rs`: kullanilmayan import kaldirildi.
- `src/modules/schedulers/cfs.rs`: kullanilmayan import kaldirildi, bilincli kullanilmayan alan `_id` yapildi.
- `src/kernel/memory/paging.rs`: kullanilmayan degisken warning'i kaldirildi.
- `src/kernel/pi_mutex.rs`: kullanilmayan import ve anlamsiz atama temizlendi.

## 3) Alan Bazli Durum ve Gap Listesi

Asagidaki tabloda durum etiketleri:
- `Done`: Kod + build + temel akis tamam.
- `Partial`: Temel yol var, edge-case/test/semantik derinlik eksik.
- `Missing`: Uretim seviyesi uygulanmamis.

### 3.1 Linux ABI / Syscall Semantics

- Durum: `Partial`
- Tamamlanan:
  - poll/select/epoll ailesinde timeout/sigmask semantigi belirgin sekilde iyilesti.
  - Unknown syscall fallback yolunda ENOSYS davranisi netlestirildi (linux_shim tarafi).
- Eksikler:
  - Tum signal-etkili bekleme syscall'lari icin ayni rigor seviyesinde maske/restore denetimi.
  - errno uyumlulugunun Linux corner-case seviyesinde tek tek dogrulanmasi.
  - ABI-level regression test matrisi (null ptr, bad ptr, size mismatch, timeout edge, interrupted wait).

### 3.2 Process/Signal Model

- Durum: `Partial`
- Tamamlanan:
  - clone/exec/fork wiring mevcut.
  - rt_sig* temel akislar mevcut.
- Eksikler:
  - signal frame push/restorer semantigi ve SA_* edge-case uyumu.
  - process lifecycle + namespace etkilesiminde daha fazla integration test.

### 3.3 FS + Storage

- Durum: `Partial`
- Tamamlanan:
  - VFS katmani ve bircok metadata/io route mevcut.
  - Journal/WAL ilerlemesi dokumante edilmis.
- Eksikler:
  - Writable production rootfs kabul kriterlerini tamamlayacak e2e dogrulama.
  - mount lifecycle ve crash-recovery senaryolarinda uzun sureli test.

### 3.4 Scheduler + Sync

- Durum: `Partial`
- Tamamlanan:
  - CFS/RT altyapi + PI mutex altyapisi mevcut.
- Eksikler:
  - scheduler fairness/latency davranisina iliskin uzun soak + threshold tabanli gate.
  - lock contention ve inversion senaryolari icin stress test paketleri.

### 3.5 Networking Dataplane

- Durum: `Partial` (high maturity)
- Tamamlanan:
  - VirtIO net + E1000 dataplane dogrulanmis.
  - recvmmsg/sendmmsg gibi batch yol var.
- Eksikler:
  - protocol/timeout corner-case testleri ve fault injection kapsam artisi.
  - long-run packet loss/reorder chaos testleri.

### 3.6 Release Engineering / CI

- Durum: `Partial`
- Tamamlanan:
  - P0/P1 gate scriptleri ve nightly wrapper'lar mevcut.
- Eksikler:
  - non-dry-run ortamlarda duzenli artifact retention + trend governance.
  - signed/reproducible release zinciri.

### 3.7 Security Isolation (Namespace/Cgroup/MAC)

- Durum: `Partial` -> `Missing` arasi
- Tamamlanan:
  - namespace wiring baslangici ve clone NEW* propagation ilerlemis.
- Eksikler:
  - cgroup enforcement (cpu/mem/io) kernel-level policy uygulamasi.
  - setns/unshare semantics ve capability policy audit'i.

## 4) Onceliklendirilmis Backlog (Actionable)

## P0 (1-2 hafta, release confidence)

1. Linux ABI semantic regression suite (poll/select/epoll + signal).
2. Error-code conformance tests (EINVAL/EFAULT/ENOSYS/EINTR).
3. Nightly non-dry-run artifact zorunlulugu ve rapor trend gate.

Kabul kriteri:
- ABI test paketleri CI'da kirmizi/yesil net sonuc verir.
- Son 7 gun nightly trendinde regressionsiz stabilite.

## P1 (2-4 hafta, operasyonel olgunluk)

1. Signal frame/restorer semantigini Linux davranisina yaklastir.
2. Namespace + process integration test seti (unshare/setns/clone kombinasyonlari).
3. FS crash-recovery ve mount lifecycle soak testleri.

Kabul kriteri:
- Ilgili test setleri en az 3 ardiskik gece kosusunda stabil.

## P2 (4-8 hafta, urunlesme)

1. cgroup enforcement temel policy (cpu+memeory en az).
2. Release reproducibility + signing pipeline.
3. Installer/artifact packaging standartlastirma.

Kabul kriteri:
- Signed artifact zinciri ve yeniden uretilen build hash esitligi.

## 5) Teknik Borc Listesi (Somut)

1. `linux_compat` ve `kernel/syscalls/linux_shim` arasinda semantik drift riskini azaltmak icin ortak helper katmani.
2. Timeout donusumlerinde birlesik utility (timespec/timeval/ms -> retry).
3. Sigmask descriptor parsing kodu tek yerde canonical hale getirilmeli.
4. High-value syscall'lar icin property/fuzz testleri eklenmeli.

## 6) Risk Kayitlari

1. Coverage metrikleri semantik bosluklari gizleyebilir.
2. Feature-gated kod yollarinda test matrisi yetersiz kalabilir.
3. Uzun sureli soak olmadan scheduler/network regressions gec fark edilir.

## 7) Onerilen 14 Gunluk Uygulama Plani

## Gun 1-3

1. ABI semantic test skeleton'u: poll/select/epoll + signal.
2. Hata kodu matrisi testleri.

## Gun 4-7

1. Signal frame/restorer gap'leri.
2. Namespace process integration testleri.

## Gun 8-10

1. FS crash-recovery scenario testleri.
2. Nightly rapor trend enforcement sertlestirme.

## Gun 11-14

1. cgroup policy MVP scope freeze.
2. Signed/reproducible release pipeline tasarim + PoC.

## 8) Cikis Kriterleri (Roadmap Exit)

- Build green + nightly green + ABI semantic test green.
- En az 2 hafta regressionsiz trend.
- P0/P1 maddelerinin CI gate'lere baglanmasi.

---

Not: Bu roadmap, mevcut coverage raporlarini ve koddaki son semantik duzeltmeleri baz alir. Her sprint sonunda bu dosya guncellenmeli ve sadece "yuzde" degil "kabul kriteri" bazli takip edilmelidir.

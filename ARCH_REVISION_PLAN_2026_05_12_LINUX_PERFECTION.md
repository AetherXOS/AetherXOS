# Linux Destek Mimari Revizyon Plani (2026-05-12)

Kapsam: Linux uyumluluk yuzeyinin production kalitesine alinmasi, yarim implementasyonlarin kapatilmasi, CI ve release kapilarinin Linux odakli sertlestirilmesi.

## 1. Bu turda tamamlanan kritik duzeltmeler

1. QEMU smoke pipeline kararliligi
- `xtask/src/commands/ops/qemu/mod.rs`
- ISO boot yolu one alindi, direct-kernel sadece fallback olarak birakildi.
- `-kernel` PVH uyumsuzlugu kaynakli smoke false-negative riski azaltildi.

2. QEMU smoke timeout sertlestirmesi
- `xtask/src/constants.rs`
- QEMU smoke timeout `20s -> 120s` yapildi.

3. Linux CI komut uyumsuzlugu duzeltildi
- `.github/workflows/linux-host-e2e.yml`
- Eskimis `ops qemu smoke` yerine `run smoke` kullanildi.

4. Linux uyumluluk kaynak yolu drift duzeltildi
- `xtask/src/config.rs`
- Legacy `modules/linux_compat` path'leri guncel `services/compat` hiyerarsisine alinmistir.

5. Network/POSIX yarim implementasyon kapatma (kritik)
- `kernel/src/services/network/libnet.rs`
- `socket()` domain/type/protocol dogrulamasi eklendi.
- `fcntl(F_GETFL/F_SETFL)` icin fd flag state eklendi.
- `dup/dup2` davranisi fs_table ile baglandi.
- `shutdown()` icin fd validasyonlu minimum semantik eklendi.

6. Deadlock riski kapatma
- `kernel/src/services/compat/posix/fs_table.rs`
- `dup()` icindeki cift lock (aynı mutex'e yeniden giris) kaldirildi.

## 2. Guncel aciklar (P0/P1 oncelikli)

### P0 - Linux semantik eksikleri (run-time)

1. IPC SysV fallback NoSys
- `kernel/src/services/compat/ipc.rs`
- `semop/semget/semctl`, `msgsnd/msgrcv/msgget`, `shm_*` feature kapali durumda dogrudan `NoSys`.
- Eylem: minimum working SysV object table + namespace + permission check + lifecycle.

2. VFS syscall yuzeyinde NoSys kalanlar
- `kernel/src/services/compat/posix/fs_table.rs`
- `resolve_at_path`, `fchmod`, `fchown`, `mknod`, `inotify_*`.
- Eylem: in-kernel VFS API baglantilari + capability/permission enforcement.

3. Process wait/fork kapsam bosluklari
- `kernel/src/services/compat/posix/process.rs`
- `waitid` belirli yollarda `NoSys`, `fork()` feature disinda `NoSys`.
- Eylem: process abstraction ile tek yol, feature disi davranisi explicit unsupported profile'a tasima.

4. Signal wait fallback NoSys
- `kernel/src/services/compat/posix/signal/wait.rs`
- signalfd/wait varyantlarinda feature-disi `NoSys`.
- Eylem: least-common-denominator emulasyonu (queue tabanli) veya hard gate + dokumante profile.

5. Poll/select yüzeyi NoSys
- `kernel/src/services/network/libnet.rs`
- `posix_poll_errno`, `posix_select_errno`.
- Eylem: epoll/poll backend bridge ile minimum Linux davranis uyumu.

### P1 - CI ve release guvenilirligi

1. Linux-host E2E su an stable toolchain kullaniyor
- `.github/workflows/linux-host-e2e.yml`
- Eylem: test job matrix (stable + nightly) ve nightly-required pathlerin explicit ayrimi.

2. Linux ABI skoru ile gercek runtime semantik senkronu
- `README.md` badge + report pipeline
- Eylem: badge'i sadece strict gate pass oldugunda update eden hard gate.

## 3. Fazli uygulama plani

## Faz A (1 hafta): Linux run-time semantik kapatma

- SysV IPC minimum implementation seti (sem/msg/shm)
- `poll/select` bridge implementation
- `fchmod/fchown/mknod/inotify_*` baglantilari

Exit kriteri:
- `Err(PosixErrno::NoSys)` sayisi Linux profile icin P0 yuzeyde sifir.

## Faz B (1 hafta): Process/Signal semantics

- `waitid/wait*` davranis uyumu (WNOHANG, child selection semantics)
- signalfd/wait profile netlestirme (emulation veya strict unsupported)

Exit kriteri:
- POSIX deep tests + Linux app compat strict profile'da process/signal regressions sifir.

## Faz C (3-4 gun): CI/release gate sertlestirme

- Linux-host E2E command drift check
- badge/report update hard gate
- unsupported syscall dokumani otomatik guncelleme

Exit kriteri:
- Linux quality gate + integration tier + linux-host-e2e workflow yesil.

## 4. Olcum ve kapilar

Zorunlu komutlar:
1. `cargo run -p xtask -- test quality-gate`
2. `cargo run -p xtask -- test tier integration --ci`
3. `cargo run -p xtask -- run smoke`

Kalite olcutleri:
- Linux profile'da P0 yolunda `NoSys` yok.
- Smoke JUnit: `failures=0`, `errors=0`.
- Process/IPC/signal regresyon testleri zorunlu.

## 5. Not

Bu planda feature kapali fallback'ler tamamen silinmiyor; production Linux profile icin kritik run-time yollarinda `NoSys` ve placeholder davranislarin kaldirilmasi hedefleniyor.

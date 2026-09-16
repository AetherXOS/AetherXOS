# P0/P1/P2 Execution Board (2026-03-14)

Bu dosya, roadmap'i eyleme donusturen canli uygulama panosudur.

## Overall

- Build health: green (`cargo check -q`)
- Compiler warnings: 0
- Linux compat semantic hardening: aktif

## P0 - Semantic Correctness (ABI-First)

Status: in progress

### Tamamlandi (bu oturum)

1. poll/select/epoll semantik sertlestirme uygulandi.
2. `ppoll` timespec timeout parse + sigset size validation aktif.
3. `epoll_pwait` sigmask parse + temporary apply/restore aktif.
4. `pselect6` descriptor parse + restore-on-error davranisi duzeltildi.
5. `retries_from_total_ns` icin tasma/overflow edge-case hatasi giderildi.
6. ABI semantik unit testleri eklendi (`poll.rs` test modulu).
7. poll/select/epoll icin syscall-level negatif path testleri eklendi.
8. `scripts/linux_abi_errno_conformance.py` eklendi ve P0 gate'e baglandi (`scripts/p0_readiness_gate.ps1`).
9. linux_shim net+util user-pointer errno parity EFAULT'a cekildi ve `scripts/linux_shim_errno_conformance.py` ile P0 gate'e baglandi.
10. `src/kernel/syscalls/linux_shim/util.rs` icin EFAULT davranisini kilitleyen unit testleri eklendi.
11. linux_shim parity sweep genisletildi (fs, fd_process_identity, signal, task_time, process::exec, socket lifecycle) ve static conformance kapsamı bu dosyalari da kapsayacak sekilde buyutuldu.

### Acik P0 maddeleri

1. poll/select/epoll syscall-level integration regression testleri (invalid ptr, null ptr, timeout edge).
2. errno conformance matrisi static kontrolden runtime test dogrulamasina tasinmali (ozellikle EFAULT/EINTR yolları).
3. nightly non-dry-run artefact zorunlulugu CI gate olarak devreye alinmali.

### Kabul kriteri

1. ABI semantik test paketi CI'da zorunlu ve stabil.
2. Son 7 gun gecmisinde semantic-regression alarmi yok.

## P1 - Operational Hardening

Status: started

### Tamamlandi (bu oturum)

1. Linux net/poll tarafinda helper seviyesinde regression test zemini kuruldu.
2. Kod tabanindaki semantik riskli warning/teknik borc noktalarinin bir kismi temizlendi.

### Acik P1 maddeleri

1. signal frame/restorer edge-case hardening.
2. namespace + process interaction integration test paketi.
3. fs crash-recovery + mount lifecycle soak testi.

### Kabul kriteri

1. En az 3 ardiskik nightly kosusunda P1 testleri regressionsuz.

## P2 - Productization & Governance

Status: started (planning + partial infra)

### Tamamlandi (bu oturum)

1. P0/P1/P2 eylem panosu olusturuldu (bu dosya).
2. Kisa vadeli uygulama adimlari dogrudan teknik kabul kriterine baglandi.

### Acik P2 maddeleri

1. cgroup enforcement MVP (cpu + memory limit path).
2. signed + reproducible release pipeline.
3. installer/artifact packaging standardizasyonu.

### Kabul kriteri

1. signed artefact zinciri ve reproducible hash dogrulamasi.
2. rollout/recovery playbook'larinin otomasyon entegrasyonu.

## Next 72h (Execution)

1. poll/select/epoll icin syscall-level negative test seti.
2. errno conformance tablo uretimi ve CI gate'e baglama.
3. namespace+signal interaction regression senaryolari.

## Not

Bu dosya sprint sonunda guncellenir; yuzde degil, kabul kriteri bazli izleme yapilir.

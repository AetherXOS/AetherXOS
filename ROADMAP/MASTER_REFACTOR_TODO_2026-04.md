# AetherCore Refactor Master TODO (2026-04)

Bu dokuman, kernel + xtask + shared katmanlarinda tekrar, okunabilirlik, bakim maliyeti ve dogrulama kalitesini yukseltecek uzun vadeli uygulama planidir.

## North Star Hedefleri

- Kod tekrarini sistematik olarak azaltmak (minimum %35 hedef)
- Moduller arasi ortak soyutlamalari paylasmak
- "magic value" kullanimini sabit/policy merkezli hale getirmek
- Hata yuzeyini netlestirmek (typed error + context zinciri)
- Kernel ve xtask tarafinda test/CI guvenini arttirmak
- Kod inceleme maliyetini dusurmek (okunabilirlik + naming + structure)

## Basari Metrikleri

- Fonksiyon uzunlugu ortalama: <= 40 satir (hot path disi)
- Bir moduldaki tekrarli command arg bloklari: <= 1 ortak builder
- Magic literal sayisi: her alt sistemde en az %50 azalma
- compile-check + clippy + secili test suiti green
- Refactor sonrasi davranis regresyonu: 0 kritik bug

## Faz 0 - Envanter ve Guvenlik Korkuluklari

- [ ] Tekrar haritasi cikar: command invocations, telemetry counters, parse ladders
- [ ] Magic value envanteri cikar: timeout, page size, buffer, id seedleri
- [ ] "risk map" olustur: hot path / boot path / security path / test-only
- [ ] Refactor checklist sablonu olustur (pre-check, patch, post-check)
- [ ] Refactor PR template olustur
- [ ] Degisiklik oncelik matrisi olustur (impact x effort x risk)
- [ ] Refactor rollback kriterleri belirle
- [ ] Baseline perf ve binary size raporu al
- [ ] Lint warning envanteri cikar ve kategori bazli grupla

Cikti:
- tekrar_raporu.md
- magic_value_raporu.md
- risk_map.md

## Faz 1 - Shared Library Expansion (Foundation)

### 1.1 Moduler Yapi
- [x] shared::units -> boyut/sure yardimcilari
- [x] shared::result -> ortak Result/Error catisi
- [x] shared::prelude -> sik kullanilan tip/makro re-export
- [x] shared::identifiers -> typed-id newtype patternleri
- [x] shared::telemetry -> sayac adlandirma/desen yardimcilari

### 1.2 Makro Paketi
- [x] define_enum! gelistirmesi: richer error + parse context
- [x] define_flags! gelistirmesi: from_bits_strict + display helper
- [x] counter macro ailesi: declarative static atomic counter setleri
- [ ] cfg-gated tracing macro wrappers
- [x] compile-time assertion macro seti (size/align/range)
- [ ] macro icin compile-fail test dosyalari ekle
- [ ] macro kullanimi icin kisa docs/examples seti ekle
- [ ] ortak macro naming guide olustur

### 1.3 API Stabilite
- [x] public API policy dokumani
- [x] deprecation policy + migration notes
- [x] no_std/alloc/std feature matrix testi

## Faz 2 - XTask Refactor Programi

### 2.1 Process Runtime Katmani
- [x] process helperlerini tek cekirdekte birlestir
- [x] owned/borrowed arg farkini generic tasarimla sadele
- [x] cwd/env/timeout/capture seceneklerini option struct ile yonet
- [x] error mesajlarini standardize et
- [x] binary probing stratejisini tek noktada topla

### 2.2 Command Builders
- [x] qemu arg builder setini ortak modula tasi
- [x] secureboot komut dizilimlerini reusable helperlara ayir
- [x] setup/install script adimlarini tablo-gudumlu hale getir
- [x] apt/flutter seed adimlarinda tekrar eden download/extract akisini teklestir

### 2.3 Logging ve Raporlama
- [x] event schema standardizasyonu
- [x] log key naming standardi (snake_case)
- [x] CI-friendly structured output katmani
- [x] text + junit + json rapor ciktilari icin ortak emitter

### 2.4 XTask Test Stratejisi
- [x] process util unit testleri
- [x] command builder golden testleri
- [x] OS-branch davranislari icin mock testleri
- [x] failure message snapshot testleri
- [x] dry-run modu icin deterministic fixture seti ekle
- [x] report emitter icin golden output testleri ekle
- [x] command execution error path coverage arttir

## Faz 3 - Kernel Refactor Programi

### 3.1 Telemetry Counter Standardi
- [ ] tekrarli Atomic counter declaration patternlerini makrolastir
- [x] stats snapshot olusturma patternlerini ortak helper ile birlestir
- [x] naming policy: CALLS/HITS/DENIED vb standard set
- [x] race-safe reset/report patterni
- [x] config control-plane telemetry counter standardization
- [ ] telemetry counter reset semantics dokumante et
- [x] interval bazli stats toplama helperi ekle

### 3.2 Magic Value Elimination
- [ ] memory page/align sabitlerini merkezi kaynaga tasi
- [ ] timeout/spin limitleri policy modullerine tasi
- [ ] id seed/base degerlerini config tabanli hale getir
- [ ] hash mix sabitleri adlandirma standardi
- [ ] sabitler icin canonical constants modulu olustur
- [ ] hot path literal audit scripti ekle

### 3.3 Security ve Capability Okunabilirligi
- [ ] resource-id gruplarini domain bazli alt modullere ayir
- [ ] capability token saklama/desen kodunu tablo-gudumlu hale getir
- [ ] access decision path'lerini step helperlarla sadele
- [ ] karar telemetrisi ve deny reason kodlarini normalize et
- [ ] security path icin karar agaci diyagrami cikar
- [ ] capability log formatini stable alanlarla netlestir

### 3.4 Scheduler/VFS/Ipc Sadeleme
- [x] scheduler config parse tekrarlarini azalt
- [x] VFS mount/policy/helper tekrarlarini ayikla
- [x] IPC queue/wait path utility extraction
- [ ] syscall edge-case map + ortak guard helperlari
- [x] ipc futex telemetry standardization
- [x] scheduler config validation helperi ayir
- [x] VFS error pathleri icin ortak context helperi ekle
- [ ] IPC timeout davranisi icin tek policy noktasi tanimla

## Faz 4 - Quality Gates

- [x] cargo check (root + xtask)
- [x] clippy hedefli (secili moduller)
- [x] test smoke + kritik moduller
- [x] compile-time feature matrix dogrulamasi
- [ ] benchmark smoke (on/off)
- [ ] doc test smoke (shared + xtask)
- [x] refactor sonrası regression checklist calistir
- [ ] CI matrixte host/target varyantlarini netlestir

## Faz 5 - Dokumantasyon ve Operasyonel Donusum

- [ ] migration guide (eski API -> yeni API)
- [ ] katkici rehberi: naming, structure, testing
- [ ] architecture decision records (ADR)
- [ ] ornek refactor playbook
- [ ] shared API kullanım rehberi yaz
- [ ] xtask command builder rehberi yaz
- [ ] kernel telemetry naming guide yaz

## Faz 6 - Gelecek Backlog

- [ ] shared crate icin public facade audit yap
- [ ] kernel modullerinde cfg boundary audit yap
- [ ] xtask icin command trace export ozelligi tasarla
- [ ] otomatik refactor candidate detector scripti dusun
- [ ] benchmark sonuclarini tarihsel olarak karsilastiran rapor olustur

## Faz 7 - Operasyonel Otomasyon

- [ ] otomatik TODO uretim kurali tanimla
- [ ] roadmap ilerleme ozeti icin generator script yaz
- [ ] refactor checklist icin dokuman bazli validation ekle
- [ ] test ve clippy sonucunu tek raporda toplayan wrapper ekle
- [ ] kod kalite kurallari icin tersine kontrol listesi olustur
- [ ] backlog item etiketleme standardi belirle

## Sprintlenmis Uygulama Listesi (Oncelik)

### Sprint A (hemen)
- [x] shared::units ve shared::prelude ekle
- [x] xtask process util duplication azalt (phase-1)
- [x] kernel allocator/security secili magic value isimlendirme
- [x] compile checks

### Sprint B
- [ ] xtask command builder module extraction
- [x] kernel telemetry counter macro adoption (pilot)
- [x] secureboot/setup tekrar kirilimi

### Sprint C
- [x] capability/resource map table-driven migration
- [x] scheduler/vfs tekrar azaltma
- [ ] comprehensive tests and docs

### Sprint D
- [ ] shared API policy maddelerini kod yorumlariyla eslestir
- [x] xtask report emitter icin ikinci bir kullanim alani ekle
- [x] kernel telemetry icin yeni bir pilot modul sec
- [ ] cfg-gated macro wrapper iskeletini olustur

## Kod Kalitesi Kurallari (Yeni Standart)

- Fonksiyon birden fazla sorumluluk tasiyorsa bol
- Her tekrar patterni icin en az bir ortak abstraction dusun
- literal gorunce once "neden policy degil?" sorusunu sor
- hata mesajinda "ne oldu + nerede oldu + ne denendi" bulunsun
- compile green olmadan refactor adimi kapanmis sayilmaz

## Izleme Panosu

- Durum: IN_PROGRESS
- Baslangic: 2026-04-02
- Sonraki Milestone: Sprint A tamamlanmasi
- Sorumlu alanlar: shared, xtask, kernel

## Bugunden Baslanan Isler

- [x] Bu master TODO dokumani olusturuldu
- [x] shared genisletme patch-1
- [x] xtask process refactor patch-1
- [x] kernel magic cleanup patch-1

# Dashboard Control Center Roadmap

Bu roadmap, dashboard uzerinden build/test/qemu/install operasyonlarini guvenli ve production-grade seviyeye tasimak icin hazirlandi.

## Hedef
- Dashboard UI, sadece rapor gosteren ekran olmaktan cikacak.
- Local Agent uzerinden kontrollu operasyon paneline donusecek.
- Acik ve denetlenebilir is akislari, role-based guvenlik, queue/scheduler ve audit ile calisacak.

## Mevcut Durum (2026-03-08)
- Control Center sekmesi var.
- Local Agent var (`scripts/dashboard_agent.ps1`).
- Async run ve job output polling var (`/run_async`, `/job`, `/status`, `/jobs`).
- Build/test/qemu/install benzeri temel aksiyonlar UI'dan tetiklenebiliyor.

### Ilerleme Ozeti (2026-03-08)
- P0: %92
  - Tamam: token auth, CORS hardening, action whitelist, cancel, audit log, stable API, risk labels, what-next action tetikleme.
  - Kalan: adim-bazli ayrintili progress model (`finalizing`) ve arg policy schema hardening.
- P1: %84
  - Tamam: queue+priority, dependency inspector, artifact explorer, analytics trend+p50/p95/p99+flaky, fallback no-data model.
  - Kalan: failure clustering/runbook deep-links’in tam otomasyonu.
- P2: %68
  - Tamam: role-based auth (viewer/operator/admin), two-step confirm, scheduler UI/agent, multi-host registry hazirligi + host matrix.
  - Kalan: federation (hostlar arasi merkezi orchestrator), crash recovery console, plugin marketplace hardening, SSE/WebSocket live stream.

## P0 (Kritik - 1-2 hafta)
1. Agent Auth Token
- Endpointlere `X-AetherCore-Token` dogrulamasi eklenecek.
- Token `scripts/config/aethercore.defaults.json` uzerinden ayarlanacak.
- UI tarafinda token giris alani + local storage secure scope.

2. Komut Whitelist ve Arg Policy
- Aksiyonlar sadece katalogdaki komutlarla calisacak (tamam),
- Ek olarak arg policy json ile sadece izinli arg setleri kabul edilecek.
- Komut enjeksiyonu riskini azaltmak icin tum dynamic arglar schema dogrulamadan gececek.

3. Job Cancellation
- `POST /job/cancel` endpointi eklenecek.
- UI'da her aktif job icin `Cancel` butonu.
- Cancel edilen job'larin output ve final status'u saklanacak.

4. Structured Logs
- Agent run loglari `reports/tooling/agent_runs/*.json` altina yazilacak.
- Alanlar: action, cmd, args, start/end, exit_code, duration, user/session id.

5. UI Progress Model
- Job state icin asama bazli progress (queued/running/finalizing/done).
- Her aksiyon icin beklenen adimlar (doctor -> build -> qemu -> dashboard) gosterilecek.

6. One-click Blueprints
- `Quick Validate`, `Release Dry Run`, `Dashboard Refresh` gibi composable blueprintler.
- UI'da tek tikla calisir ve alt adimlari gorunur.

## P1 (Yuksek - 2-4 hafta)
1. Queue + Concurrency Controller
- Tek job yerine kuyruk modeli (max concurrency configurable).
- Islem onceligi: high/normal/low.

2. Dependency Inspector Panel
- Host tool surumleri + min required + drift/uyumluluk skoru.
- `Fix` aksiyonu ile ilgili setup scriptine yonlendirme.

3. Artifact Explorer
- Build sonucu ISO, rapor, test output, checksum dosyalari tek panelde.
- Boyut, son degisiklik, hash ve indir/open aksiyonlari.

4. Failure Analytics 2.0
- Job bazli success/fail trend grafikleri.
- P95 duration by action, flaky action detector.
- Failure clustering (lock_conflict/test_failed/config_invalid vb.).

5. Smart Recommendations
- What-should-I-do-next motorunu action-aware hale getirme.
- Her oneride direkt calistirilabilir buton + risk seviye etiketi.

6. Runbook Links
- Her hata kodu icin playbook linki + mini adim kartlari.

## P2 (Orta/Uzun Vade - 4-8 hafta)
1. Multi-Host Agent Federation
- Birden fazla host agent'a baglanma (lab farm).
- Host secici + host bazli run status.

2. Role-based UI
- Viewer / Operator / Admin rollerine gore aksiyon gorunurlugu.
- Kritik aksiyonlarda "2-step confirm".

3. Temporal Schedulers
- Gece smoke run, haftalik full gate, otomatik dashboard refresh.
- Cron benzeri scheduler UI.

4. Crash Recovery Console
- Son panic dump/diagnostics zip ve one-click triage.
- Recovery pipeline trigger (A/B boot gibi) kontrol paneli.

5. Plugin Marketplace (Local)
- Plugin action kartlari (json manifest + signature).
- Enable/disable + compatibility checks.

6. Rich Live Stream
- Polling yerine SSE/WebSocket ile canli output stream.
- Backpressure ve reconnect destegi.

## Guvenlik Notlari
- Localhost agent bile olsa token zorunlu olmali.
- Hassas komutlar icin ikinci onay adimi olmali.
- Tum runlar audit log'a yazilmali.
- Output masking: token/path/secret filtreleme katmani eklenmeli.

## Basari Kriterleri
- Dashboard uzerinden en az 10 ana operasyon tek tikla calisiyor.
- Run success/fail, duration, output gercek zamanli izleniyor.
- Hatalarin %80'i UI icinden actionable onerilerle cozuluyor.
- Yeni kullanici 10 dakika icinde "agent baslat -> build -> qemu -> rapor" akisini tamamlayabiliyor.

## Hemen Sonraki 7 Gun Plani
1. Token auth + CORS hardening
2. Job cancel endpoint + UI cancel
3. Agent structured logs + run history panel
4. Blueprint actions (3 adet)
5. Failure analytics chart seti (action success trend + p95)
6. Dependency inspector (read-only)
7. Docs + walkthrough gif/video

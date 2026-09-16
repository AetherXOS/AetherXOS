# LibNet Runtime Playbook

Bu dokuman, uzun sureli calismada LibNet profil ayarlari ve batch sizing icin operasyonel baseline sunar.

## 1) PollProfile Secimi

- `LowLatency`: tail latency odakli is yukleri (RPC, control-plane).
- `Balanced`: genel amacli varsayilan profil.
- `Throughput`: buyuk paket/bulk transfer.
- `PowerSave`: arka plan veya dusuk trafik.

## 2) Adaptive Mod

- `Adaptive`, core basinc sinyallerine gore profil degistirir.
- Beklenen davranis:
  - kuyruk derinligi artisinda `Throughput` tarafina kayma,
  - bos/nominal durumda `Balanced` veya `PowerSave`.

## 3) Fast-Path Pump Budget

- Baslangic: `libnet_fast_path_pump_budget = 64`
- Tuning:
  - yuksek p95/p99 gecikmede budget dusur (`32` civari),
  - kuyruk birikiminde budget arttir (`96-128` arasi).
- Degisiklikleri her adimda soak/stress ile dogrula.

## 4) Isletim Pratigi

- Her release adayi icin:
  1. `python scripts/libnet_feature_bench.py`
  2. `python scripts/soak_stress_chaos.py --rounds 200 --timeout-sec 300`
  3. `python scripts/reboot_recovery_gate.py --soak-summary artifacts/qemu_soak/summary.json`

- Alarm durumlari:
  - artan drop/backpressure sayaçlari,
  - poll error artisi,
  - recovery gate fail.

## 5) Rollback Kurali

- Yeni profil/budget degisikligi sonrası 2 ardışık gate fail olursa:
  - son bilinen stabil profile geri don,
  - fail anindaki telemetri snapshotini sakla,
  - degisikligi kademeli tekrar dene.

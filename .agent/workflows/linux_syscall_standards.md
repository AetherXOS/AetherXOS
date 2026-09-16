---
description: Production-Grade Linux Syscall Implementation Standards
---

# Linux Syscall Geliştirme Standartları

Bu döküman, Antigravity (AI) tarafından Linux uyumluluk katmanını geliştirirken uyulması gereken KESİN kuralları içerir.

## 1. Kod Kalitesi ve Profesyonellik
- **Simülasyon Yasaktır**: Syscall'lar "mış gibi" (mock/dummy) yapılmamalıdır. Eğer bir donanım veya çekirdek özelliği eksikse, o özellik çekirdeğe eklenmeli veya uygun bir hata (`ENOSYS`, `EOPNOTSUPP`) döndürülmelidir.
- **Production-Ready**: Kod sadece "çalışıyor" olmamalı; yarış durumlarına (race conditions), geçersiz kullanıcı pointer'larına (UserPtr validation) ve memory leak'lere karşı taş gibi sağlam olmalıdır.
- **Sabitlerin Düzeni**: Yeni eklenen sabitler (`linux_nr`, `fcntl`, `errno` vb.) sayısal sıraya veya mantıksal gruplarına göre yazılmalıdır. Dağınık ekleme yapılmamalıdır.

## 2. Implementasyon Öncelikleri
- **Memory Safety**: `UserPtr` her zaman doğrulanmalı, kernel adresleri kullanıcıya sızdırılmamalıdır.
- **Alignment**: Stack ve TLS yapıları işlemci mimarisinin (x86_64: 16-byte alignment) gereksinimlerini tam karşılamalıdır.
- **Error Codes**: Linux'un resmi `errno` değerleri (`-EFAULT`, `-EINVAL` vb.) tam olarak eşleşmelidir. POSIX hataları doğrudan döndürülmemeli, Linux'un beklediği negatif değer formatına çevrilmelidir.

## 3. Eksiklerin Tespiti ve Raporlama
- Her büyük adımdan sonra `missing` ve `incomplete` syscall'lar raporlanmalıdır.
- Implemente edilen her syscall için "Neden production-grade?" sorusuna cevap verilecek seviyede yorum satırı eklenmelidir.

## 4. Test ve Doğrulama
- Karmaşık syscall'lar (futex, clone, epoll) için kernel-land testleri veya kullanıcı alanı simülasyonları düşünülmelidir.
- `cargo check` her zaman temiz (sıfır warning) olmalıdır.

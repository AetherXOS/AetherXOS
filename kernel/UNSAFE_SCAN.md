Kernel Unsafe Usage Scan
=========================

Bu dosya, hızlı bir grep taramasından elde edilen `unsafe {` kullanım örneklerini listeler. Amaç: bellek güvenliği incelemesi için öncelikli dosyaları belirlemek.

Bulunan örnekler (örnek, eksiksiz değil):
- `kernel/src/hal/dtb_parser.rs` — birkaç `unsafe` kullanımı (string/byte parsing)
- `kernel/src/hal/acpi_parser.rs` — DTB/ACPI parsing sırasında `unsafe` blokları
- `kernel/src/hal/devices/uart.rs` — doğrudan donanım erişimleri
- `kernel/src/hal/devices/timer.rs` — init/write_config fonksiyonlarında `unsafe`
- `kernel/src/hal/devices/interrupts.rs` — IRQ yönetiminde `unsafe`
- `kernel/src/hal/devices/i2c_spi.rs` — donanım init ve select_slave içinde `unsafe`
- `kernel/src/kernel/dynamic_linker/elf_dynamic/access.rs` — pointer read/write helper'ları (merkezileştirildi)

İnceleme önerisi:
1. Her `unsafe` bloğu için kısa açıklama yaz: neden gerekli, olası güvenlik riskleri, alternatif (safe wrapper mümkün mü?).
2. Donanım erişimleri (`hal/devices/*`) için güvenli sarmalayıcı (safe API) uygulayın ve `unsafe`'ı yalnızca bu sarmalayıcıların içinde bırakın.
3. `unsafe` fonksiyon imzalarında belgelendirme ekleyin (gerekli önkoşullar, garanti edilen invariants).
4. Automatize edilecek testler: memory-safety unit testleri (mangled test images), fuzzing ile bytes-parsing fonksiyonları.

Sonraki adım: `unsafe` kullanım noktalarının tam listesi ve her biri için refactor önerilerini içeren ayrıntılı raporu hazırlayayım.

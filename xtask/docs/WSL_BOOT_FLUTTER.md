<!--
  WSL Boot + Flutter Quickstart
  Path: xtask/docs/WSL_BOOT_FLUTTER.md
  Purpose: step-by-step commands to produce a Debian rootfs with Flutter installed
           using WSL on Windows, then boot it with QEMU on the Windows host GUI.
  IMPORTANT: This file contains the full commands; follow them from WSL and PowerShell.
-->

*** Begin Patch
*** Update File: c:\Users\oyunm\Desktop\OS\xtask\docs\WSL_BOOT_FLUTTER.md
@@
# WSL: xtask ile Build & Boot — Flutter-ready VM (Windows host)

Bu belge, doğrudan `xtask` komutunu kullanarak WSL (Ubuntu) içinde Debian rootfs oluşturma, Flutter SDK'yı rootfs içine kurma ve sonrasında Windows üzerinde QEMU ile GUI olarak VM'yi başlatma adımlarını içerir. Shell scriptleri doğrudan çalıştırmayın — `xtask` aynı işi yapacak şekilde paketlendi.

## Önkoşullar
- Windows üzerinde WSL2 ve bir Ubuntu dağıtımı yüklü olmalı.
- Windows tarafında QEMU (`qemu-system-x86_64`) yüklü olmalı.
- Repo kökü Windows yolunda (ör: `C:\Users\oyunm\Desktop\OS`) ve WSL içinde `/mnt/c/Users/oyunm/Desktop/OS` olarak erişilebilir olmalı.
- `kernel.x` dosyası repo kökünde mevcut olmalı (yoksa önce kernel derleyin).

## Neden `xtask`?
`xtask` tek noktadan tüm ortam kurulumlarını, paketlemeyi ve platform-farklı yürütme yollarını yönetir. Burada `cargo run -p xtask -- setup bootstrap-flutter` komutu, daha önceki `xtask/scripts/bootstrap_flutter.sh` betiğini güvenli, ortam-duyarlı bir şekilde çağırır ve Windows/WSL/Docker durumlarını doğru şekilde ele alır.

## Adımlar — WSL (Ubuntu) içinde

1. WSL (Ubuntu) terminalini açın.

2. Gerekli yardımcı paketleri kurun (bu, debootstrap ve loop-mount araçlarını sağlar):

```bash
sudo apt update
sudo apt install -y debootstrap rsync fakeroot dpkg-dev e2fsprogs util-linux curl
```

3. Repo köküne gidin (WSL yolunu kullanın):

```bash
cd /mnt/c/Users/oyunm/Desktop/OS
```

4. (Opsiyonel) `xtask`'i derleyin (gerekli değil; `cargo run` otomatik derler):

```bash
cargo build -p xtask
```

5. `xtask` ile rootfs oluşturma ve Flutter yükleme — QEMU bootunu atlamak (Windows'ta GUI ile başlatmak için):

```bash
SKIP_BOOT=1 cargo run -p xtask -- setup bootstrap-flutter --outdir /mnt/c/Users/oyunm/Desktop/OS/outdir --flutter-url 'https://storage.googleapis.com/flutter_infra_release/releases/stable/linux/flutter_linux_3.41.6-stable.tar.xz' --kernel-image /mnt/c/Users/oyunm/Desktop/OS/kernel.x
```

SKIP_BOOT=1 cargo run -p xtask -- setup bootstrap-flutter \
    --outdir ./outdir \
    --flutter-url 'https://storage.googleapis.com/flutter_infra_release/releases/stable/linux/flutter_linux_3.41.6-stable.tar.xz' \
    --kernel-image ./kernel.x

SKIP_BOOT=1 cargo run -p xtask -- setup bootstrap-flutter \
    --outdir ./outdir \
    --flutter-url 'https://storage.googleapis.com/flutter_infra_release/releases/stable/linux/flutter_linux_3.41.6-stable.tar.xz' \
    --kernel-image ./kernel.x \
    -Z build-std=core,alloc

SKIP_BOOT=1 cargo run -p xtask --target x86_64-unknown-linux-gnu -- setup bootstrap-flutter \
    --outdir ./outdir \
    --flutter-url 'https://storage.googleapis.com/flutter_infra_release/releases/stable/linux/flutter_linux_3.41.6-stable.tar.xz' \
    --kernel-image ./kernel.x \
    -Z build-std=core,alloc

- Notlar:
  - `SKIP_BOOT=1` ortam değişkeni, `xtask` içindeki pipeline'ın QEMU'yu otomatik başlatmasını engeller (WSL içinde GUI başlatmak sorunlu olabilir). Bu şekilde yalnızca disk görüntüsü oluşturulur.
  - `--outdir` mutlaka host erişimli bir yol olmalı (ör. `/mnt/c/...`), böylece Windows tarafında aynı dosyaya erişebilirsiniz.

6. `xtask` komutu tamamlandığında disk imajı şu konumda olacaktır (Windows tarafında):

```
C:\Users\oyunm\Desktop\OS\outdir\work\rootfs.img
```

> Eğer WSL içinde `losetup` veya mount ile izin sorunları görürseniz: `outdir`'ı WSL yerelinde (ör. `/home/<user>/work/outdir`) oluşturup iş bittikten sonra `cp` ile Windows `C:\` altına kopyalayabilirsiniz.

## Adımlar — Windows (PowerShell) ile GUI başlatma

1. PowerShell açın ve repo köküne gidin (veya `kernel.x` ve `outdir\work\rootfs.img` yollarını tam belirtin).

2. Aşağıdaki QEMU komutunu çalıştırın (pencere açılır, grafik destekli):

```powershell
qemu-system-x86_64 -m 4096 -kernel kernel.x -append "root=/dev/vda rw console=tty0 rootwait" -drive file=outdir\work\rootfs.img,if=virtio,format=raw -display sdl -vga virtio -device virtio-gpu-pci
```

& "C:\Program Files\qemu\qemu-system-x86_64.exe" -m 4096 -kernel kernel.x -append "root=/dev/vda rw console=tty0 rootwait" -drive file=outdir\work\rootfs.img,if=virtio,format=raw -display sdl -vga virtio -device virtio-gpu-pci

& "C:\Program Files\qemu\qemu-system-x86_64.exe" -m 4096 -kernel kernel.elf -append "root=/dev/vda rw console=tty0 rootwait" -drive file=outdir\work\rootfs.img,if=virtio,format=raw -display sdl -vga virtio -device virtio-gpu-pci -machine pc

& "C:\Program Files\qemu\qemu-system-x86_64.exe" -m 4096 -cdrom aetherxos.iso -serial stdio -vga std -display sdl -d cpu,int

3. VM açıldıktan sonra guest içinde Flutter uygulamalarını çalıştırmak için (guest içinde):

```bash
sudo apt update
sudo apt install -y libgtk-3-0 libglu1-mesa clang cmake ninja-build pkg-config
/opt/flutter/bin/flutter doctor
cd /opt/flutter/examples/hello_world
/opt/flutter/bin/flutter run -d linux --enable-software-rendering
```

## Hızlı hata giderme
- `SKIP_BOOT=1` kullanmayıp WSL içinden QEMU başlatmak istiyorsanız: WSL ortamınızın X11/Wayland/GUI yönlendirmesinin doğru yapılandırıldığından ve `DISPLAY` değişkeninin ayarlı olduğundan emin olun. Genelde Windows tarafında GUI için doğrudan Windows QEMU daha güvenilirdir.
- `losetup`/`mount` hatası: `outdir`'ı WSL yerelinde oluşturup (`/home/<user>/...`), iş bittikten sonra `cp` ile `C:\...` altına taşıyın.
- `kernel.x` yoksa önce kernel derleyin: `cargo run -p xtask -- build` ya da repo özel build adımlarını çalıştırın.

---
Bu dosya artık doğrudan `xtask` kullanımını gösterir; betiği elle çağırmayın. Eğer isterseniz, bu belgeyi daha kısa bir 'one-liner' rehber haline getireyim.


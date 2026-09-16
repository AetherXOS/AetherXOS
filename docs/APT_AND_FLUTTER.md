Apt + Flutter integration plan for this OS
=========================================

Amaç
-----
- Hedef: Hedef işletim sisteminin kullanıcısına `apt` ile paket yükleme imkanı sağlamak ve `flutter` paketini `apt` üzerinden kurup çalıştırabilmek.
- Kapsam: Bu doküman, hedef rootfs oluşturma, Flutter SDK'yı `.deb` paketine dönüştürme, yerel apt deposu oluşturma ve kernel/yan-kütüphane gereksinimleri için kontrol listesi sunar.

Kısıtlar ve gerçekçi beklentiler
-------------------------------
- Bu repo bir kernel + userspace runtime içeriyor. "apt" çalıştırmak için full Debian/Ubuntu userspace (glibc, dpkg, apt) gerekir; en pratik yol debootstrap ile hazır bir rootfs oluşturmak veya resmi Debian tarball'ını kullanmaktır.
- Flutter genellikle büyük bağımlılıklar (GTK/OpenGL/Wayland/X11, libfontconfig, vs.) gerektirir; tam masaüstü desteği için kernel'de GPU (virtio-gpu / PCI passthrough) ve Wayland/X11 destekleri ile kullanıcı alanında ilgili kütüphaneler gereklidir.

Yüksek seviye yaklaşım
-----------------------
1. Host (geliştirici makinasi) üzerinde Debian rootfs oluştur (debootstrap) veya indir.
2. Rootfs içinde `apt` ve temel araçları kur; repository ve mirror yapılandır.
3. Flutter SDK'yı indir, `/opt/flutter` altına aç, bir `.deb` paket oluştur ve yerel apt deposuna ekle.
4. Hedef işletim sisteminde rootfs'i mount/boot et veya emulator/QEMU ile çalıştır; `apt update` -> `apt install flutter-sdk` test et.

Adım adım (Linux host üzerinde hızlı yol)
-----------------------------------------
1) Gereksinimler (host): `debootstrap`, `dpkg-deb`, `dpkg-scanpackages` (debian-utils), `curl`/`wget`, `fakeroot`.

2) Debian rootfs oluştur (örnek):

```bash
OUTDIR=./debian-rootfs
sudo debootstrap --variant=minbase --arch=amd64 stable "$OUTDIR" http://deb.debian.org/debian/
```

3) Chroot içine kısa yapılandırma:

```bash
sudo mount -t proc /proc "$OUTDIR/proc"
sudo mount -t sysfs /sys "$OUTDIR/sys"
sudo mount --bind /dev "$OUTDIR/dev"
sudo cp /etc/resolv.conf "$OUTDIR/etc/"
sudo chroot "$OUTDIR" /bin/bash -c "apt-get update && apt-get install -y apt-transport-https ca-certificates curl gnupg"
sudo umount "$OUTDIR/proc" || true
sudo umount "$OUTDIR/sys" || true
sudo umount "$OUTDIR/dev" || true
```

4) Flutter'ı `.deb` yapmak (yerel repo için):

- Flutter tarball URL'sini (resmi) indir. Örnek:

```bash
FLUTTER_URL=https://storage.googleapis.com/flutter_infra_release/releases/stable/linux/flutter_linux_<version>-stable.tar.xz
curl -L -o flutter.tar.xz "$FLUTTER_URL"
```

- Paket kökünü oluşturup `opt/flutter` altına açın, `usr/bin/flutter` için küçük bir wrapper ekleyin ve `DEBIAN/control` dosyasını hazırlayın, ardından `dpkg-deb --build` ile `.deb` oluşturun.

5) Yerel apt deposu oluşturma

```bash
mkdir -p repo
cp flutter-sdk.deb repo/
cd repo
dpkg-scanpackages . /dev/null | gzip -9c > Packages.gz
```

Debian sunucusu olarak kullanmak için hedef rootfs içinde `/etc/apt/sources.list.d/local-flutter.list` içine:

```text
deb [trusted=yes] file:/path/to/repo ./
```

ve sonra `apt-get update && apt-get install flutter-sdk`.

Kernel & runtime kontrol listesi (minimum)
-----------------------------------------
- `execve` + `PT_INTERP` handling — bu repo'da çekirdek tarafı için esas işlevsellik mevcut ([src/modules/posix/process/exec_runtime.rs](src/modules/posix/process/exec_runtime.rs)).
- Dinamik bağlayıcı/relokasyon — [src/kernel/dynamic_linker.rs](src/kernel/dynamic_linker.rs) ve `so_loader`.
- Dosya sistemi: ext4 veya başka disk backend (diskfs) + overlay desteği (copy-up) — `mount_overlay` ve `WritableOverlayFs`.
- Ağ (TCP/UDP/DNS) — `apt` paketleri indirmek için host ağ stack gereklidir; test için rootfs içinde `curl`/`apt` ile doğrulayın.
- Gerekli syscalls: `mmap`, `munmap`, `mprotect`, `open`, `read`, `write`, `socket`, `connect`, `sendto`, `recvfrom`, `getaddrinfo`, `poll/epoll`, `fork/clone/execve`, `prctl`, `setpgid` ve benzerleri.
- Grafik/GPU (Flutter desktop için): EGL/OpenGL (mesa), Wayland veya X11, libdrm / GPU driver (virtio-gpu veya gerçek GPU passthrough).

Özet / Öneriler
----------------
- Hızlı yol: Host üzerinde `debootstrap` ile rootfs oluşturup kernel ile birlikte boot edin; sonra `apt` içinde Flutter paketini test edin.
- Üretim şeklinde "apt ile doğrudan kur" desteği istiyorsanız iki yol var:
  - Hedef rootfs'e direkt Debian paket deposu ekleyip `apt` kullanmak (en basit), veya
  - Flutter SDK'yı bir `.deb` haline getirip dahili repo'ya eklemek (kontrollü).
- GPU/Wayland/X11 desteği büyük ek iş; önce `flutter --version` ve `flutter doctor` çalıştırılana kadar ilerleyin, ardından grafik hızlandırma ve oturumu devreye alın.

Dosyalar ve yardımcı scriptler repo içinde `scripts/` ve bu dokümanda listelenmiştir.

İleri adımlar (ben yapabilirim)
------------------------------
1. `scripts/prepare_debian_rootfs.sh` ekleyip test edilmesini sağlamak (debootstrap gerektirir).
2. `scripts/package_flutter_deb.sh` ile Flutter SDK'yı `.deb` paketine çevirme scripti.
3. `xtask` içine doğrulama adımı eklemek: rootfs oluştur, boot/emu ile başlat, `apt install flutter-sdk`, `flutter --version`.
4. GPU/Wayland adımlarını planlayıp kernel tarafındaki eksikleri tespit etmek.

Eklenen yardımcı scriptler
-------------------------
- `scripts/mk_rootfs_image.sh` — debootstrap ile oluşturulmuş bir rootfs klasörünü ext4 disk görüntüsüne dönüştürür (qemu ile mount edilebilir). Kullanım:

```bash
bash scripts/mk_rootfs_image.sh /path/to/debian-rootfs out-rootfs.img 2048
```

- `scripts/run_qemu_rootfs.sh` — kernel imajı (`kernel.x`) ile oluşturulmuş ext4 görüntüsünü kullanarak QEMU başlatır. Kullanım:

```bash
bash scripts/run_qemu_rootfs.sh kernel.x out-rootfs.img 2048
```

Notlar: Bu komutlar Linux host üzerinde çalışmak üzere tasarlanmıştır; Windows geliştiricileri WSL veya bir Linux VM kullanmalıdır. QEMU boot komutu `-kernel` argümanı ile çekirdek görüntüsünü önyükler; çekirdeğinizin `bzImage` uyumlu olduğundan emin olun veya alternatif boot yöntemleri kullanın.

Ek opsiyon: tam otomasyon
---------------------------------
`scripts/bootstrap_and_boot_with_flutter.sh` betiği tüm adımları birbirine zincirler: debootstrap rootfs, Flutter `.deb` oluşturma, chroot ile `.deb` kurulumu (rootfs'e önceden yükleme), ext4 disk görüntüsü oluşturma ve QEMU ile boot. Örnek:

```bash
bash scripts/bootstrap_and_boot_with_flutter.sh /tmp/debian-rootfs 'https://storage.googleapis.com/flutter_infra_release/releases/stable/linux/flutter_linux_<ver>-stable.tar.xz' kernel.x
```

Bu betik Linux host üzerinde çalışır ve root yetkisi gerektiren adımlar içerir (`mount`, `losetup`, `chroot`).

Not: Windows host için WSL veya Linux VM/Konteyner kullanmanız en hızlı, en stabil yoldur.

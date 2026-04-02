#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 OUTDIR FLUTTER_TAR_XZ_URL"
  echo "Example: $0 ./out 'https://storage.googleapis.com/flutter_infra_release/releases/stable/linux/flutter_linux_3.7.0-stable.tar.xz'"
  exit 1
fi

OUTDIR=$1
FLUTTER_URL=$2
PKGNAME=${3:-flutter-sdk}
VERSION=${4:-0.0.1}

mkdir -p "$OUTDIR/work"
TMP="$OUTDIR/work"

echo "Downloading Flutter from: $FLUTTER_URL"
curl -L -o "$TMP/flutter.tar.xz" "$FLUTTER_URL"

echo "Building .deb layout"
DEB_ROOT="$OUTDIR/debroot"
rm -rf "$DEB_ROOT"
mkdir -p "$DEB_ROOT/opt"
tar -xf "$TMP/flutter.tar.xz" -C "$DEB_ROOT/opt"

mkdir -p "$DEB_ROOT/usr/bin"
cat > "$DEB_ROOT/usr/bin/flutter" <<'EOF'
#!/usr/bin/env bash
exec /opt/flutter/bin/flutter "$@"
EOF
chmod +x "$DEB_ROOT/usr/bin/flutter"

mkdir -p "$DEB_ROOT/DEBIAN"
cat > "$DEB_ROOT/DEBIAN/control" <<EOF
Package: $PKGNAME
Version: $VERSION
Section: utils
Priority: optional
Architecture: amd64
Maintainer: aethercore <dev@local>
Description: Flutter SDK packaged for local apt install
EOF

echo "Building deb package"
fakeroot dpkg-deb --build "$DEB_ROOT" "$OUTDIR/${PKGNAME}_${VERSION}_amd64.deb"

echo "Created: $OUTDIR/${PKGNAME}_${VERSION}_amd64.deb"

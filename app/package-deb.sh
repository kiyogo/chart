#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TARGET_BIN="$ROOT_DIR/target/release/chatw-app"

if [ ! -f "$TARGET_BIN" ]; then
  echo "Target binary not found at $TARGET_BIN. Build it first with: cargo build -p chatw-app --features desktop --release"
  exit 1
fi

DEB_ROOT="$ROOT_DIR/target/deb_package"
rm -rf "$DEB_ROOT"
mkdir -p "$DEB_ROOT/DEBIAN"
mkdir -p "$DEB_ROOT/usr/bin"
mkdir -p "$DEB_ROOT/usr/share/applications"
mkdir -p "$DEB_ROOT/usr/share/icons/hicolor/scalable/apps"
mkdir -p "$DEB_ROOT/usr/share/doc/chatw"

# 1. Binary
cp "$TARGET_BIN" "$DEB_ROOT/usr/bin/chatw"
chmod 755 "$DEB_ROOT/usr/bin/chatw"

# 2. Desktop file
cat << 'EOF' > "$DEB_ROOT/usr/share/applications/chatw.desktop"
[Desktop Entry]
Name=chatw
Comment=Ứng dụng nhắn tin bảo mật mã hóa đầu cuối E2EE (Rust + Dioxus)
Exec=/usr/bin/chatw
Icon=chatw
Terminal=false
Type=Application
Categories=Network;InstantMessaging;Chat;
Keywords=chat;e2ee;privacy;rust;dioxus;
StartupWMClass=chatw
EOF
chmod 644 "$DEB_ROOT/usr/share/applications/chatw.desktop"

# 3. Icon
cp "$SCRIPT_DIR/public/logo.svg" "$DEB_ROOT/usr/share/icons/hicolor/scalable/apps/chatw.svg"
chmod 644 "$DEB_ROOT/usr/share/icons/hicolor/scalable/apps/chatw.svg"

# 4. Doc / Copyright
cat << 'EOF' > "$DEB_ROOT/usr/share/doc/chatw/copyright"
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: chatw
Source: https://github.com/kiyogo/chart

Files: *
Copyright: 2026 chatw contributors
License: MIT
EOF

# 5. Debian control
INST_SIZE=$(du -sk "$DEB_ROOT/usr" | cut -f1)
cat << EOF > "$DEB_ROOT/DEBIAN/control"
Package: chatw
Version: 0.1.0
Section: net
Priority: optional
Architecture: amd64
Installed-Size: $INST_SIZE
Maintainer: chatw team <https://github.com/kiyogo/chart>
Depends: libgtk-3-0 (>= 3.24.0) | libgtk-3-0t64, libwebkit2gtk-4.1-0 (>= 2.40.0) | libwebkitgtk-6.0-4, libssl3 (>= 3.0.0)
Description: Ứng dụng nhắn tin bảo mật mã hóa đầu cuối E2EE
 chatw là ứng dụng nhắn tin cá nhân bảo mật cao, mã hóa đầu cuối (E2EE)
 chuẩn ECDH P-256 và AES-256-GCM, xây dựng bằng Rust và Dioxus 0.7.
EOF

dpkg-deb --build "$DEB_ROOT" "$ROOT_DIR/target/chatw_0.1.0_amd64.deb"
echo "Successfully built Debian package: target/chatw_0.1.0_amd64.deb"
ls -lh "$ROOT_DIR/target/chatw_0.1.0_amd64.deb"


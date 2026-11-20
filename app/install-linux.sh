#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TARGET_BIN="$ROOT_DIR/target/release/chatw-app"

if [ ! -f "$TARGET_BIN" ]; then
  echo "Building ChatW desktop release binary..."
  (cd "$ROOT_DIR" && cargo build -p chatw-app --features desktop --release)
fi

INSTALL_DIR="$HOME/.local/share/chatw"
BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"

mkdir -p "$INSTALL_DIR" "$BIN_DIR" "$APP_DIR" "$ICON_DIR"

# 1. Install desktop binary
cp "$TARGET_BIN" "$INSTALL_DIR/chatw-desktop"
chmod 755 "$INSTALL_DIR/chatw-desktop"

# 2. Install crisp vector SVG icon (blue rounded square + white ChatW bubble mark)
cat << 'EOF' > "$ICON_DIR/chatw.svg"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
  <rect x="2" y="2" width="60" height="60" rx="16" fill="#2563EB"/>
  <path d="M18 17h28a7 7 0 0 1 7 7v15a7 7 0 0 1-7 7H32.5l-9.2 6.6c-1 .72-2.4 0-2.4-1.24V46H18a7 7 0 0 1-7-7V24a7 7 0 0 1 7-7z" fill="rgba(255,255,255,0.18)" stroke="#FFFFFF" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M21 28l3.6 9.2L32 29.5l7.4 7.7L43 28" fill="none" stroke="#FFFFFF" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
EOF
chmod 644 "$ICON_DIR/chatw.svg"

# 3. Install launcher wrapper (ensures local libs & local backend are ready)
cat << EOF > "$BIN_DIR/chatw"
#!/usr/bin/env bash
set -euo pipefail

export LD_LIBRARY_PATH="\$HOME/.local/lib:\$HOME/.local/usr/lib/x86_64-linux-gnu:\${LD_LIBRARY_PATH:-}"
APP_BIN="$INSTALL_DIR/chatw-desktop"
REPO_DIR="$ROOT_DIR"
API_URL="\${CHATW_API_URL:-http://127.0.0.1:8085}"

if [ "\$API_URL" = "http://127.0.0.1:8085" ] || [ "\$API_URL" = "http://localhost:8085" ]; then
  if ! curl -fsS --max-time 1 "http://127.0.0.1:8085/" >/dev/null 2>&1; then
    SERVER_BIN=""
    if [ -x "\$REPO_DIR/target/release/chatw" ]; then
      SERVER_BIN="\$REPO_DIR/target/release/chatw"
    elif [ -x "\$REPO_DIR/target/debug/chatw" ]; then
      SERVER_BIN="\$REPO_DIR/target/debug/chatw"
    fi
    if [ -n "\$SERVER_BIN" ]; then
      (cd "\$REPO_DIR" && nohup "\$SERVER_BIN" > "$INSTALL_DIR/server.log" 2>&1 &)
      for _ in \$(seq 1 30); do
        if curl -fsS --max-time 1 "http://127.0.0.1:8085/" >/dev/null 2>&1; then
          break
        fi
        sleep 0.15
      done
    fi
  fi
fi

exec "\$APP_BIN" "\$@"
EOF
chmod 755 "$BIN_DIR/chatw"

# 4. Install .desktop entry for Ubuntu / GNOME / KDE app menu & dock
cat << EOF > "$APP_DIR/chatw.desktop"
[Desktop Entry]
Name=ChatW
GenericName=Encrypted Messenger
Comment=Ứng dụng nhắn tin bảo mật mã hóa đầu cuối E2EE (Rust + Dioxus)
Exec=$BIN_DIR/chatw
Icon=$ICON_DIR/chatw.svg
Terminal=false
Type=Application
Categories=Network;InstantMessaging;Chat;
Keywords=chat;chatw;e2ee;privacy;messenger;rust;dioxus;
StartupNotify=true
StartupWMClass=chatw-app
EOF
chmod 644 "$APP_DIR/chatw.desktop"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" >/dev/null 2>&1 || true
fi

echo "Installed ChatW desktop app:"
echo "  Launcher : $BIN_DIR/chatw"
echo "  Binary   : $INSTALL_DIR/chatw-desktop"
echo "  Icon     : $ICON_DIR/chatw.svg"
echo "  Desktop  : $APP_DIR/chatw.desktop"

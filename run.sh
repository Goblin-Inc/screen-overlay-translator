#!/usr/bin/env bash
set -e
DIR="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")" && pwd)"

# Prevent WebKitGTK Wayland Error 71 on Hyprland / Sway / Niri / KDE Wayland
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export WEBKIT_DISABLE_COMPOSITING_MODE=1
export GDK_BACKEND="${GDK_BACKEND:-x11}"

if [ -x "$DIR/src-tauri/target/release/screen-overlay-translator" ]; then
    BIN="$DIR/src-tauri/target/release/screen-overlay-translator"
elif [ -x "$DIR/bin/screen-overlay-translator" ]; then
    BIN="$DIR/bin/screen-overlay-translator"
else
    echo "[*] Бинарник не найден, запускаю сборку Cargo (release)..."
    cargo build --release --manifest-path "$DIR/src-tauri/Cargo.toml"
    mkdir -p "$DIR/bin"
    cp "$DIR/src-tauri/target/release/screen-overlay-translator" "$DIR/bin/screen-overlay-translator"
    chmod +x "$DIR/bin/screen-overlay-translator"
    BIN="$DIR/bin/screen-overlay-translator"
fi

exec "$BIN" "$@"

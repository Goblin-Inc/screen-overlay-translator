#!/usr/bin/env bash
set -e
DIR="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")" && pwd)"

if command -v pacman >/dev/null 2>&1; then
    echo "[*] Установка зависимостей для Arch Linux (X11 + Wayland/Hyprland)..."
    sudo pacman -S --needed tesseract tesseract-data-eng tesseract-data-rus maim grim wl-clipboard imagemagick webkit2gtk-4.1 rust
elif command -v apt >/dev/null 2>&1; then
    echo "[*] Установка зависимостей для Ubuntu / Debian..."
    sudo apt update && sudo apt install -y tesseract-ocr tesseract-ocr-eng tesseract-ocr-rus maim grim wl-clipboard imagemagick libwebkit2gtk-4.1-dev cargo rustc
fi

if [ ! -f "$DIR/bin/screen-overlay-translator" ] && [ ! -f "$DIR/src-tauri/target/release/screen-overlay-translator" ]; then
    echo "[*] Сборка бинарника screen-overlay-translator через Cargo..."
    cargo build --release --manifest-path "$DIR/src-tauri/Cargo.toml"
fi

if [ -f "$DIR/src-tauri/target/release/screen-overlay-translator" ]; then
    mkdir -p "$DIR/bin"
    cp "$DIR/src-tauri/target/release/screen-overlay-translator" "$DIR/bin/screen-overlay-translator"
fi

if [ -f "$DIR/bin/screen-overlay-translator" ]; then
    chmod +x "$DIR/bin/screen-overlay-translator"
fi
chmod +x "$DIR/run.sh"

echo "✅ Готово! Запускайте: ./run.sh"

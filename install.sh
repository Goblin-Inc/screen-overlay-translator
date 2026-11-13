#!/usr/bin/env bash
set -e
DIR="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")" && pwd)"
if command -v pacman >/dev/null 2>&1; then
    sudo pacman -S --needed tesseract tesseract-data-eng tesseract-data-rus maim webkit2gtk-4.1
elif command -v apt >/dev/null 2>&1; then
    sudo apt update && sudo apt install -y tesseract-ocr tesseract-ocr-eng tesseract-ocr-rus maim libwebkit2gtk-4.1-0
fi
chmod +x "$DIR/bin/screen-overlay-translator"
echo "Готово! Запускайте: ./bin/screen-overlay-translator"

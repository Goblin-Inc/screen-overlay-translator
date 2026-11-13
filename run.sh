#!/usr/bin/env bash
set -e
DIR="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")" && pwd)"
chmod +x "$DIR/bin/screen-overlay-translator"
exec "$DIR/bin/screen-overlay-translator" "$@"

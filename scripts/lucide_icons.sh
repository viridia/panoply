#!/usr/bin/env bash
# Fetches Lucide icons by name and writes each as a 48 px white-on-transparent PNG, as the editor's
# other icons are, into assets/editor/icons/lucide/, with Lucide's licence beside them.
#
#   scripts/lucide_icons.sh lock box ruler
#
# Needs curl and rsvg-convert (macOS: `brew install librsvg`; Ubuntu: `apt install librsvg2-bin`).
set -euo pipefail

VERSION="0.460.0"
BASE="https://unpkg.com/lucide-static@${VERSION}"
OUT="$(cd "$(dirname "$0")/.." && pwd)/assets/editor/icons/lucide"

if [ "$#" -eq 0 ]; then
    echo "usage: $0 <icon name>..." >&2
    exit 2
fi

mkdir -p "$OUT"
curl -fsSL "${BASE}/LICENSE" -o "${OUT}/LICENSE"
for name in "$@"; do
    svg="$(curl -fsSL "${BASE}/icons/${name}.svg")" || {
        echo "no Lucide icon named ${name} in ${VERSION}" >&2
        exit 1
    }
    printf '%s' "${svg//currentColor/#ffffff}" | rsvg-convert -w 48 -h 48 -o "${OUT}/${name}.png"
    echo "${OUT}/${name}.png"
done

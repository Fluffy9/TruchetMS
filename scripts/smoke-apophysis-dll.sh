#!/usr/bin/env bash
# Load the built DLLs the way Apophysis does (LoadLibrary + GetProcAddress),
# under Wine in Docker, and drive them through a short session.
#
#   scripts/build-apophysis-dll.sh && scripts/smoke-apophysis-dll.sh
#
# Wine 10 (Debian trixie) is needed: the DLLs import bcryptprimitives.dll, which
# older Wine releases do not provide. Windows 10 or newer has it.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
image="${DEBIAN_IMAGE:-debian:trixie-slim}"

for arch in x86 x64; do
  [ -f "$root/dist/apophysis/$arch/truchet_ms.dll" ] || {
    echo "missing dist/apophysis/$arch/truchet_ms.dll; run scripts/build-apophysis-dll.sh" >&2
    exit 1
  }
done

docker run --rm -v "$root":/src:ro -e WINEDEBUG="${WINEDEBUG:--all}" -e WINEPREFIX=/tmp/wine "$image" bash -euc '
  dpkg --add-architecture i386
  apt-get update -qq
  DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends \
    gcc-mingw-w64-x86-64-posix gcc-mingw-w64-i686-posix wine wine64 wine32:i386 >/dev/null
  src=/src/crates/truchet-apophysis/smoke/smoke.c
  mkdir -p /tmp/smoke && cd /tmp/smoke
  x86_64-w64-mingw32-gcc-posix -O2 "$src" -o smoke64.exe
  i686-w64-mingw32-gcc-posix   -O2 "$src" -o smoke32.exe
  cp /src/dist/apophysis/x64/truchet_ms.dll truchet_ms64.dll
  cp /src/dist/apophysis/x86/truchet_ms.dll truchet_ms32.dll
  wine smoke64.exe truchet_ms64.dll
  wine smoke32.exe truchet_ms32.dll
'

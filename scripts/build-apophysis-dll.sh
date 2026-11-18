#!/usr/bin/env bash
# Cross-compile the Apophysis 7X plugin (truchet_ms.dll) for 32- and 64-bit
# Windows from Linux or macOS, using MinGW inside Docker.
#
#   scripts/build-apophysis-dll.sh
#
# Output:
#   dist/apophysis/x86/truchet_ms.dll   for 32-bit Apophysis
#   dist/apophysis/x64/truchet_ms.dll   for 64-bit Apophysis
#
# On Windows, skip this: `cargo build --release -p truchet-apophysis` with the
# MSVC toolchain (add `--target i686-pc-windows-msvc` for 32-bit) does the same.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
image="${RUST_IMAGE:-rust:1.86-bookworm}"

docker run --rm \
  -v "$root":/src -w /src \
  -e CARGO_TARGET_DIR=/tmp/target \
  -e HOST_UID="$(id -u)" -e HOST_GID="$(id -g)" \
  -e CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc-posix \
  -e CARGO_TARGET_I686_PC_WINDOWS_GNU_LINKER=i686-w64-mingw32-gcc-posix \
  "$image" bash -euc '
    apt-get update -qq
    apt-get install -y -qq --no-install-recommends mingw-w64 >/dev/null
    rustup target add x86_64-pc-windows-gnu i686-pc-windows-gnu

    for triple in x86_64-pc-windows-gnu:x64 i686-pc-windows-gnu:x86; do
      target="${triple%%:*}"; arch="${triple##*:}"
      objdump="${target%%-pc-*}-w64-mingw32-objdump"
      [ "$arch" = x86 ] && objdump=i686-w64-mingw32-objdump

      cargo build --release -p truchet-apophysis --target "$target"
      dll="/tmp/target/$target/release/truchet_ms.dll"

      # Every export the Apophysis loader looks up must be in the DLL.
      exports="$("$objdump" -p "$dll")"
      for fn in PluginVarGetName PluginVarGetNrVariables PluginVarGetVariableNameAt \
                PluginVarCreate PluginVarDestroy PluginVarInit PluginVarInit3D \
                PluginVarInitDC PluginVarPrepare PluginVarCalc PluginVarGetVariable \
                PluginVarSetVariable PluginVarResetVariable PluginVarGetArchitecture; do
        if ! grep -qw "$fn" <<<"$exports"; then
          echo "error: $dll does not export $fn" >&2; exit 1
        fi
      done

      # Only Windows system DLLs: nothing to ship next to the plugin.
      echo "== $arch imports"
      grep "DLL Name" <<<"$exports" | sort -u
      if grep -iE "DLL Name: (libgcc|libwinpthread|libstdc)" <<<"$exports"; then
        echo "error: $dll depends on a MinGW runtime DLL" >&2; exit 1
      fi

      mkdir -p "dist/apophysis/$arch"
      cp "$dll" "dist/apophysis/$arch/truchet_ms.dll"
    done

    chown -R "$HOST_UID:$HOST_GID" dist
  '

echo
ls -l "$root"/dist/apophysis/*/truchet_ms.dll

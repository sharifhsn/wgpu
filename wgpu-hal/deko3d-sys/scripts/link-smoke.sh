#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
DEVKITPRO=${DEVKITPRO:-${DEKO3D_SYS_DEVKITPRO:-/opt/devkitpro}}
DEVKITA64=${DEVKITA64:-$DEVKITPRO/devkitA64}
CXX=${CXX:-$DEVKITA64/bin/aarch64-none-elf-g++}

if [ ! -x "$CXX" ]; then
  echo "skipped deko3d-sys link smoke: missing Horizon C++ toolchain at $CXX"
  exit 0
fi
if [ ! -f "$DEVKITPRO/libnx/include/deko3d.h" ]; then
  echo "skipped deko3d-sys link smoke: missing deko3d header under $DEVKITPRO/libnx/include"
  exit 0
fi
DEKO3D_LIB_DIR="$DEVKITPRO/libnx/lib"
if [ -f "$DEVKITPRO/portlibs/switch/lib/libdeko3d.a" ]; then
  DEKO3D_LIB_DIR="$DEVKITPRO/portlibs/switch/lib"
fi
if [ ! -f "$DEVKITPRO/libnx/lib/libnx.a" ] || [ ! -f "$DEKO3D_LIB_DIR/libdeko3d.a" ]; then
  echo "skipped deko3d-sys link smoke: missing libnx or libdeko3d static library under $DEVKITPRO"
  exit 0
fi

OUT=${TMPDIR:-/tmp}/deko3d-sys-link-smoke.elf
"$CXX" -std=c++14 -O0 -fPIE -pie -I "$DEVKITPRO/libnx/include" \
  "$ROOT_DIR/abi/deko3d_sys_link_smoke.cpp" \
  -L "$DEKO3D_LIB_DIR" -L "$DEVKITPRO/libnx/lib" \
  -specs="$DEVKITPRO/libnx/switch.specs" -ldeko3d -lnx -o "$OUT"
rm -f "$OUT"
echo "deko3d-sys link smoke passed against $DEVKITPRO"

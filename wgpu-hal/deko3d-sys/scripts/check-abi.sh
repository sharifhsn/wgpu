#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
DEVKITPRO=${DEVKITPRO:-${DEKO3D_SYS_DEVKITPRO:-/opt/devkitpro}}
DEVKITA64=${DEVKITA64:-$DEVKITPRO/devkitA64}
CXX=${CXX:-$DEVKITA64/bin/aarch64-none-elf-g++}

skip_if_missing() {
  if [ ! -e "$1" ]; then
    echo "skipped deko3d-sys B1 ABI check: missing $1"
    exit 0
  fi
}

skip_if_missing "$CXX"
skip_if_missing "$DEVKITPRO/libnx/include/deko3d.h"
skip_if_missing "$DEVKITPRO/libnx/include/switch.h"

"$CXX" \
  -std=c++14 \
  -fsyntax-only \
  -I "$DEVKITPRO/libnx/include" \
  "$ROOT_DIR/abi/deko3d_sys_b1_abi_check.cpp"

echo "deko3d-sys B1 ABI check passed against $DEVKITPRO"

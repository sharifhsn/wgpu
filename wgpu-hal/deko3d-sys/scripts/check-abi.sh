#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
DEVKITPRO=${DEVKITPRO:-${DEKO3D_SYS_DEVKITPRO:-/tmp/devkitpro-switch1/opt/devkitpro}}
DEVKITA64=${DEVKITA64:-$DEVKITPRO/devkitA64}
CXX=${CXX:-$DEVKITA64/bin/aarch64-none-elf-g++}

require_file() {
  if [ ! -e "$1" ]; then
    echo "missing required file: $1" >&2
    exit 1
  fi
}

require_file "$CXX"
require_file "$DEVKITPRO/libnx/include/deko3d.h"
require_file "$DEVKITPRO/libnx/include/switch.h"

"$CXX" \
  -std=c++14 \
  -fsyntax-only \
  -I "$DEVKITPRO/libnx/include" \
  "$ROOT_DIR/abi/deko3d_sys_abi_check.cpp"

echo "deko3d-sys ABI check passed against $DEVKITPRO"

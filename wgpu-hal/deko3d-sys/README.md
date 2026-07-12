# deko3d-sys

`deko3d-sys` is the backend-owned raw ABI layer for the experimental Horizon Deko3D backend.
It contains handwritten declarations only for the public Deko3D and libnx surface needed by the
planned v29 backend. The HAL owns all higher-level resource, synchronization, and presentation
semantics.

The parent `wgpu-hal` `deko3d` feature selects the release `deko3d` library. Direct consumers
must select exactly one of `debug-deko3d` or `release-deko3d` on Horizon.

## ABI check

Run the header check after changing this crate's handwritten declarations:

```sh
DEVKITPRO=/opt/devkitpro ./scripts/check-abi.sh
```

The B1 check uses public devkitPro headers and verifies every extern exposed by the B1 Rust ABI
surface. It intentionally excludes harness, applet, RomFS, input, allocator, and deferred API
symbols.

## Link smoke

Run the optional target link smoke when the Horizon toolchain and static libraries are installed:

```sh
DEVKITPRO=/opt/devkitpro ./scripts/link-smoke.sh
```

The script exits successfully with an explicit `skipped` reason when that target toolchain or either
static library is unavailable.

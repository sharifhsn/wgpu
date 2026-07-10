# deko3d-sys

Minimal raw FFI smoke bindings for the public Nintendo Switch homebrew deko3d
and libnx headers.

This crate is intentionally small for Milestone 1. It binds the handle types,
maker structs, default helpers, and symbols needed to prove that Rust can name
deko3d/libnx types and emit link metadata for `libdeko3dd.a` plus `libnx.a`.
Full generated bindings are deferred until the Rust `deko_basic` parity proof
shows which parts of the C API are actually needed.

For the Switch target, the build script searches for devkitPro in this order:

1. `DEKO3D_SYS_DEVKITPRO`
2. `DEVKITPRO`
3. `/opt/devkitpro`
4. `/tmp/devkitpro-switch1/opt/devkitpro`

The default feature links the debug deko3d library, `deko3dd`. Use
`--no-default-features --features release-deko3d` to emit release-library link
metadata.

## ABI/header check

Run this after changing the handwritten FFI surface:

```sh
DEVKITPRO=/tmp/devkitpro-switch1/opt/devkitpro ./scripts/check-abi.sh
```

The check compiles `abi/deko3d_sys_abi_check.cpp` with public devkitA64 C++14 in
`-fsyntax-only` mode. It asserts the enum values, struct sizes, field offsets,
opaque object alignment/sizes, default state bit patterns, and function
signatures currently used by the Rust `deko_basic` port. It is intentionally
narrow; broad generated bindings remain deferred until the next backend slice
needs more deko3d surface area.

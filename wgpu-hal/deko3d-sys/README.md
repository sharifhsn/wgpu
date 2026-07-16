# deko3d-sys

Raw, `no_std` Rust bindings for the public C ABI of
[Deko3D](https://github.com/devkitPro/deko3d), the low-level Nintendo Switch homebrew graphics API.

This crate provides ABI declarations and Cargo link integration only. It does not implement GPU
resource ownership, synchronization, command validation, or safe lifetimes. Those semantics belong
in a higher-level consumer such as the Deko3D backend in `wgpu-hal`.

This is an independent community binding and is not an official devkitPro project.

## Using the crate

Select exactly one Deko3D library on Horizon builds:

```toml
[dependencies]
deko3d-sys = {
    version = "0.1",
    default-features = false,
    features = ["release-deko3d"],
}
```

Then use the raw API through the conventional underscore crate name:

```no_run
use deko3d_sys::{DkDeviceMaker, DkResult};

let maker = DkDeviceMaker::defaults();
let _success = DkResult::DkResult_Success;
```

The crate remains buildable without either link feature on non-Horizon hosts. This supports unit
tests, documentation builds, ABI tooling, and downstream cross-platform dependency graphs without
requiring devkitPro on the host.

## Features

- `release-deko3d` links `libdeko3d.a`.
- `debug-deko3d` links `libdeko3dd.a`, Deko3D's validation-enabled debug library.

Enabling both, or neither, is rejected when targeting Horizon. The `wgpu-hal` Deko3D backend selects
`release-deko3d` explicitly.

## Toolchain discovery

On Horizon, `build.rs` looks for a devkitPro prefix in this order:

1. `DEKO3D_SYS_DEVKITPRO`
2. `DEVKITPRO`
3. `/opt/devkitpro`

The selected prefix must contain the public headers and libraries under `libnx/include` and
`libnx/lib`.

## Safety

Almost every operation is unsafe. Callers must obey Deko3D's requirements for object lifetimes,
alignment, mapped memory, command-buffer state, queue synchronization, descriptor validity, and
thread access. Raw handles may be null, dangling, or invalid, and the Rust type system cannot verify
the relationships between devices, queues, memory blocks, images, shaders, and swapchains.

The small Rust helper functions in this crate mirror public inline helpers from `deko3d.h`; they do
not add validation. Prefer a higher-level backend unless direct ABI access is specifically required.

## ABI maintenance

The binding surface is curated rather than regenerated during every Cargo build. This keeps crates.io
and cross-compilation builds deterministic and avoids requiring libclang for ordinary consumers.

After changing a declaration, validate it against the installed public headers:

```sh
DEVKITPRO=/opt/devkitpro ./scripts/check-abi.sh
```

The check verifies the exposed enum values, constants, structure sizes and alignments, field offsets,
bitfield encodings, default helpers, and external function signatures.

When the Horizon C++ toolchain is installed, also run the target link smoke:

```sh
DEVKITPRO=/opt/devkitpro ./scripts/link-smoke.sh
```

For the complete host-side publication gate:

```sh
./scripts/check-package.sh
```

## Versioning

Until version `1.0`, additions are expected as the wgpu backend covers more Deko3D functionality.
Breaking Rust API or verified ABI changes increment the minor version. The public Deko3D header
remains the source of truth.

## License

Licensed under the Zlib License in the packaged `LICENSE` file, matching Deko3D's permissive
license.

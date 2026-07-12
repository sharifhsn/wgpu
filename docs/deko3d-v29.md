# Deko3D on wgpu v29

`wgpu-deko3d-29` is the active implementation line. The sibling v30 worktree is reference-only; do not cherry-pick from it or maintain commit parity.

## Phase A

Phase A establishes the native v29 integration boundary:

- `Backend::Deko3d` and `Backends::DEKO3D` identify the target and parse `WGPU_BACKEND=deko3d`.
- Deko3D stays outside `Backends::all()`, defaults, support tiers, and `Backend::ALL`; callers must select `Backends::DEKO3D` explicitly.
- The release `deko3d` feature flows from `wgpu` to `wgpu-core` to `wgpu-hal`.
- `wgpu-hal/deko3d-sys` owns the relative devkitPro/Deko3D/libnx link configuration for Horizon builds.

`wgpu-core::Instance` registers the concrete Deko3D HAL and `wgpu::Instance::enabled_backend_features`
advertises it only under the `deko3d` cfg alias, which requires both `target_os = "horizon"` and
the release `deko3d` feature. This keeps ordinary builds and default backend selection unchanged.

## Native v29 milestones

1. B1a complete: `wgpu-hal/deko3d-sys` owns the raw ABI declarations and header-layout probe. It has no HAL runtime claim.
2. B1b complete: a native v29 `wgpu-hal::deko3d::Api` owns its HAL types and satisfies the full trait surface. It remains HAL-only, and `Instance::create_surface` fails closed until the explicit Switch policy is ready.
3. B1c complete: validate the buffer, DKSH shader, pipeline, command, and explicit default-Switch surface paths needed for a public triangle, then wire the functional API into `wgpu-core::Instance` and `wgpu::Instance::enabled_backend_features`.
4. B2 partial: the opaque path accepts one mip-level, single-sample `Rgba8Unorm` or
   `Rgba8UnormSrgb` sampled texture, a filtering sampler, one static texture/sampler bind
   group with an optional uniform (or a separate uniform bind group), `Float32x2`/`x3`/`x4`
   and `Uint32` vertex attributes,
   cull/front-face state, and no-blend color output. `Depth32Float` supports clear/load,
   depth compare, and writes. Dynamic viewport and scissor state are recorded. Every other
   texture format, blend mode, depth/stencil mode, topology, and binding form remains rejected.

For captured WGSL, install one [`Deko3dWgslArtifactProvider`] on the `Device` before pipeline
creation. The provider receives the exact WGSL bytes and SHA-256 digest plus the requested
vertex, fragment, or compute entry point, and returns a single-program DKSH artifact. It is
set-once, shared by `Device` clones, and invoked outside wgpu's provider lock. WGSL is only a
manifest key on Deko3D: the stage-specific DKSH is selected when a pipeline is created.
This does not add a general runtime WGSL compiler or broaden the single-program DKSH contract.

## DKSH validation fixture

Forced-host unit tests validate the single-program DKSH container without a Switch toolchain. The
checked-in `wgpu-hal/src/deko3d/test-data/deko-basic-color_fsh.dksh` fixture is the 512-byte
fragment shader from the sibling public `deko-basic-rs` proof, generated with its documented
public devkitPro/UAM toolchain. Its SHA-256 is
`eea40963451820c1a6b7ea6834ac579989b37f5badab3740f38ba397d35df28d`.

The validator accepts only one program until public program-selection metadata exists. Tests cover
the fixture and independent mutations of every checked header, program-table, stage, entrypoint,
constant-buffer-range, and truncation class. The forced-host checks do not validate Deko3D FFI
linking. The ABI and link-smoke scripts report an explicit skip when the Horizon toolchain or its
static libraries are unavailable.

## Default Switch surface seam

The cfg-gated default surface leases libnx's single process-global window and never fabricates raw
window/display handles. A second live default surface is rejected and drop releases the lease.
Textures carry monotonic surface and configuration identities; configure, submit, present, and
discard reject stale or mismatched ownership. Horizon uses Deko3D fences and waits
conservatively before reporting completion. Forced-host coverage verifies lease, identity, and
wrong-target behavior only; no Horizon window, queue, or fence exists on the host.

## Explicit triangle proof and external target gate

`examples/standalone/deko3d_explicit_triangle` is a deliberately small source proof. It selects
`Backends::DEKO3D`, creates only `Deko3dDefaultSurface`, configures `Rgba8Unorm`, maps a vertex
buffer, consumes one checked-in offline DKSH vertex/fragment pair, creates a zero-bind-group
triangle-list pipeline, then submits and presents one frame. The fixtures are
`triangle_vsh.dksh` (`f38d9572d01b39a2a535a03c7d62bff9b23346d65a6a4dea6277e4a22aadea80`) and
`color_fsh.dksh` (`eea40963451820c1a6b7ea6834ac579989b37f5badab3740f38ba397d35df28d`).

The remaining gate is an external Horizon build and run with the devkitPro/libnx/Deko3D target
toolchain and real hardware. Host checks compile the selection plumbing and test fail-closed
behavior, but do not claim that target build or presentation validation.

The explicit triangle was built on 2026-07-12 against the public devkitPro prefix at
`/tmp/devkitpro-switch1/opt/devkitpro`; its NRO SHA-256 was
`87e5d05b107b2e012a0b0f91dd281337703a1b64f06cd39d857f79f3090887a0`.

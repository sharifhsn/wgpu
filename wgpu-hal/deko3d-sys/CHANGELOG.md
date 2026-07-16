# Changelog

All notable changes to `deko3d-sys` are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Before version 1.0,
minor releases may contain breaking changes to the Rust API when required to track Deko3D's public
C ABI.

## [Unreleased]

### Added

- Complete crates.io package metadata and deterministic package contents.
- Crate-level usage, safety, toolchain, ABI-maintenance, and versioning documentation.
- A Zlib license file and a repeatable package-readiness check.
- Public-header-verified advanced command, query, multisample, dynamic-state, storage-buffer, and
  compute ABI declarations required by the wgpu backend.

### Changed

- The ABI gate now compiles both the narrow B1 surface and the extended backend surface.

## [0.1.0] - 2026-07-16

### Added

- Initial `no_std` Deko3D and libnx raw ABI surface used by the wgpu Deko3D backend.
- Debug and release Deko3D link-selection features.
- Header ABI verification and Horizon target link-smoke tooling.

#!/usr/bin/env bash
set -euo pipefail

CRATE_DIR=$(cd "$(dirname "$0")/.." && pwd)
DEVKITPRO=${DEVKITPRO:-/opt/devkitpro}

cargo fmt --manifest-path "$CRATE_DIR/Cargo.toml" --check --package deko3d-sys
cargo test --manifest-path "$CRATE_DIR/Cargo.toml" --package deko3d-sys
cargo check \
    --manifest-path "$CRATE_DIR/Cargo.toml" \
    --package deko3d-sys \
    --no-default-features \
    --features debug-deko3d
cargo check \
    --manifest-path "$CRATE_DIR/Cargo.toml" \
    --package deko3d-sys \
    --no-default-features \
    --features release-deko3d
cargo clippy --manifest-path "$CRATE_DIR/Cargo.toml" --package deko3d-sys -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc \
    --manifest-path "$CRATE_DIR/Cargo.toml" \
    --package deko3d-sys \
    --no-deps
DEVKITPRO="$DEVKITPRO" "$CRATE_DIR/scripts/check-abi.sh"
DEVKITPRO="$DEVKITPRO" "$CRATE_DIR/scripts/link-smoke.sh"
cargo package \
    --manifest-path "$CRATE_DIR/Cargo.toml" \
    --package deko3d-sys \
    --allow-dirty

#!/bin/sh
# Opt-in release build for the CPU this compiler is running on.
# Profile release: thin LTO, one codegen unit, target-cpu=native.
set -eu
cd "$(dirname "$0")/.."
unset CARGO_ENCODED_RUSTFLAGS
export RUSTFLAGS="-C target-cpu=native"
echo "$(rustc --version)"
echo "runtime: profile release, thin LTO, codegen-units 1, target-cpu=native"
cargo build --locked --release -p horizon-game
echo "built binary: horizon-client"

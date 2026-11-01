#!/bin/sh
# Build in dependency order and stage the exact binaries pinned by build.rs.
set -eu
cd "$(dirname "$0")/.."
profile=debug
for arg in "$@"; do
    if [ "$arg" = --release ]; then profile=release; fi
done
runner=${SPORE_BUILD_RUNNER:-cross}
target=riscv64imac-unknown-none-elf
mkdir -p "build/$profile"
for contract in spore_extension_lua spore_extension_rust cluster cluster_proxy cluster_agent spore mutant_equipment mutant_probe; do
    "$runner" build --locked --target "$target" -p "$contract" "$@"
    cp "target/$target/$profile/$contract" "build/$profile/$contract"
done
cp contracts/spore_extension_lua/lua/libckblua.so "build/$profile/libckblua.so"

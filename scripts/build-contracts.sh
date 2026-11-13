#!/bin/sh
# Build and stage this standalone optional-CoBuild (legacy Lua) candidate.
set -eu
cd "$(dirname "$0")/.."
profile=debug
for arg in "$@"; do
    if [ "$arg" = --release ]; then profile=release; fi
done
runner=${SPORE_BUILD_RUNNER:-cross}
target=riscv64imac-unknown-none-elf
mkdir -p "build/$profile"
: "${SPORE_DEPLOYMENT_MANIFEST:?set a reviewed code Type ID plan (VM tests use deployment/type-ids.fixture.toml)}"
python3 scripts/type-id-plan.py "$SPORE_DEPLOYMENT_MANIFEST" "build/$profile/code-type-ids.json"

for contract in spore_extension_lua cluster cluster_proxy cluster_agent spore; do
    "$runner" build --locked --target "$target" -p "$contract" "$@"
    cp "target/$target/$profile/$contract" "build/$profile/$contract"
done

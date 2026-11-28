#!/bin/sh
# Build a complete set in isolation before publishing the plan and its ELFs.
set -eu
cd "$(dirname "$0")/.."

profile=debug
target_dir_pending=false
for arg do
    shift
    if "$target_dir_pending"; then
        CARGO_TARGET_DIR=$arg
        export CARGO_TARGET_DIR
        target_dir_pending=false
        continue
    fi
    case "$arg" in
        --target-dir) target_dir_pending=true ;;
        --target-dir=*) export CARGO_TARGET_DIR="${arg#*=}" ;;
        --release) profile=release; set -- "$@" "$arg" ;;
        *) set -- "$@" "$arg" ;;
    esac
done
if "$target_dir_pending"; then
    echo "--target-dir requires a directory" >&2
    exit 1
fi
runner=${SPORE_BUILD_RUNNER:-cross}
target=riscv64imac-unknown-none-elf
: "${SPORE_DEPLOYMENT_MANIFEST:?set a reviewed code Type ID plan (VM tests use deployment/type-ids.fixture.toml)}"

mkdir -p build
stage=$(mktemp -d "build/.$profile.XXXXXXXX")
cargo_stage=
cleanup() {
    if [ -n "$cargo_stage" ]; then rm -rf "$cargo_stage"; fi
    rm -rf "$stage"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
mkdir "$stage/artifacts"
cp "$SPORE_DEPLOYMENT_MANIFEST" "$stage/type-ids.toml"
export SPORE_DEPLOYMENT_MANIFEST="$stage/type-ids.toml"
optional_contracts=$(python3 scripts/type-id-plan.py "$SPORE_DEPLOYMENT_MANIFEST" "$stage/artifacts/code-type-ids.json")

# Resolve Cargo's effective target root before isolating this build's outputs.
metadata=$(cargo metadata --locked --no-deps --format-version 1)
target_root=$(printf '%s\n' "$metadata" | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
mkdir -p "$target_root"
cargo_stage=$(mktemp -d "$target_root/.spore-contracts.XXXXXXXX")
export CARGO_TARGET_DIR="$cargo_stage"
for contract in spore_extension_rust cluster $optional_contracts spore mutant_equipment mutant_probe; do
    if [ "$contract" = spore ]; then
        "$runner" build --locked --target "$target" -p "$contract" "$@" \
            --message-format=json-render-diagnostics > "$stage/spore-build.json"
        python3 scripts/cargo-build-network.py < "$stage/spore-build.json" > "$stage/artifacts/network"
    else
        "$runner" build --locked --target "$target" -p "$contract" "$@"
    fi
    cp "$cargo_stage/$target/$profile/$contract" "$stage/artifacts/$contract"
done
python3 scripts/code-hashes.py "$stage/artifacts" >/dev/null

python3 - "$stage/artifacts" "build/$profile" <<'PY'
import ctypes
import fcntl
import os
from pathlib import Path
import sys

artifacts, output = map(Path, sys.argv[1:])
lock = os.open(output.parent, os.O_RDONLY | os.O_DIRECTORY)
try:
    fcntl.flock(lock, fcntl.LOCK_EX)
    if output.is_symlink() or (output.exists() and not output.is_dir()):
        sys.exit(f"Expected a build profile directory at {output}")
    if not output.exists():
        artifacts.rename(output)
    else:
        # Exchange complete directories in one syscall. Until it succeeds,
        # the existing profile remains in place, even if the process dies.
        libc = ctypes.CDLL(None, use_errno=True)
        if sys.platform.startswith("linux") and hasattr(libc, "renameat2"):
            at_fdcwd = -100
            result = libc.renameat2(
                at_fdcwd, os.fsencode(artifacts), at_fdcwd, os.fsencode(output), 2
            )
        elif sys.platform == "darwin" and hasattr(libc, "renamex_np"):
            result = libc.renamex_np(os.fsencode(artifacts), os.fsencode(output), 2)
        else:
            sys.exit("Replacing a build profile requires atomic directory exchange")
        if result:
            error = ctypes.get_errno()
            raise OSError(error, os.strerror(error), output)
finally:
    os.close(lock)
PY

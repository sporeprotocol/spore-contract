#!/bin/sh
# Adapt legacy ckb-std C compiler flags for a local RISC-V-capable Clang.
set -eu
for arg do
    shift
    case "$arg" in
        -nostartfiles|-Wno-dangling-pointer|-Wno-nonnull-compare|--target=*) ;;
        *) set -- "$@" "$arg" ;;
    esac
done
exec "${SPORE_CLANG:-clang}" --target=riscv64 -march=rv64imac \
    -gdwarf-4 -Wno-unused-command-line-argument "$@"

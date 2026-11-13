# Isolated Lua performance baseline

The current Spore and cluster contracts support native Rust/RISC-V Mutants only.
Nothing in this directory is compiled by the default workspace, build script or
test suite. Historical Lua contract source and deployment records remain in
their original locations.

The comparison uses the fixed intermediate commit
[`af622aca750f16c7b777c0c3126b21ad9e68dc34`](https://github.com/a19q3/spore-contract/commit/af622aca750f16c7b777c0c3126b21ad9e68dc34),
which ran Lua through returning `spawn/wait` and enforced subsequent Spore checks.
It is not the old deployed `exec` path that could skip validation. This baseline
lets us compare the same recipe and applicable validation on both paths without
putting a Lua compatibility branch into the new contracts.

`fixture.rs` is the exact one-off benchmark used for that commit. It measures
creation, mint, transfer and burn, plus 1/2/4 rules and a mixed pair. Each
transaction is verified five times with identical cycles. `measurements.json`
contains the captured values; `binaries-*.json` record file sizes and SHA-256
digests. `historical-tests.rs` archives the previous Lua-only test cases and is
not a module in the current test suite.

## Reproduce the baseline separately

Run from a checkout of the current branch, with the pinned Rust toolchain and
RISC-V-capable Clang installed. The local adapter was tested with Clang 22.1.8.

```sh
native_repo="$PWD"
benchmark_root="$(mktemp -d)"
git clone https://github.com/a19q3/spore-contract.git "$benchmark_root/repo"
git -C "$benchmark_root/repo" checkout --detach af622aca750f16c7b777c0c3126b21ad9e68dc34
cp "$native_repo/benchmarks/legacy-lua/fixture.rs" \
  "$benchmark_root/repo/tests/src/tests/performance_fixture.rs"
printf '\ninclude!("performance_fixture.rs");\n' >> \
  "$benchmark_root/repo/tests/src/tests/mutant_rust.rs"
cd "$benchmark_root/repo"
rustup target add riscv64imac-unknown-none-elf
SPORE_BUILD_RUNNER=cargo \
CC_riscv64imac_unknown_none_elf="$native_repo/scripts/clang-riscv.sh" \
./scripts/build-contracts.sh --release
CAPSULE_TEST_ENV=release cargo test --locked --manifest-path tests/Cargo.toml \
  pr75_performance_report -- --nocapture
```

For mainnet, rebuild with `--release --features release_export` and run the same
test command. Keep the baseline checkout and its staged binaries separate from
the current checkout. A different compiler/toolchain may change measured cycles.

The current native-only benchmark and the comparison results are documented in
the [performance report](../../examples/mutant-rust/PERFORMANCE.md).

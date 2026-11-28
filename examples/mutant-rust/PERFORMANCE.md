> Historical data-bound measurements below do not validate the new Type ID-bound artifacts. See [current deployment and validation boundaries](../../docs/TYPE_BINDING.md).

# Native Mutant performance

The current contracts execute native Rust/RISC-V Mutants only. For this small
recipe, a single native rule uses **82.9% fewer mint-transaction VM cycles** than
the equivalent Lua rule in the isolated comparison. Four rules use **86.9% fewer**.
These are script-verifier cycle counts, not wall-clock timings or a throughput
claim for arbitrary game rules.

## Comparison boundary

Native results below were measured from the current native-only implementation.
Lua results are preserved from the fixed intermediate commit
[`af622aca750f16c7b777c0c3126b21ad9e68dc34`](https://github.com/a19q3/spore-contract/commit/af622aca750f16c7b777c0c3126b21ad9e68dc34).
That commit used returning `spawn/wait` for Lua and enforced the remaining Spore
checks. Both sides therefore execute the same recipe and applicable Spore/co-build
validation. The comparison does not use the historical deployed `exec` behavior
that could skip checks. The current contracts cannot execute the Lua baseline.

## Release/testnet results

| Operation, one rule | Current native cycles | Lua baseline cycles | Reduction |
| --- | ---: | ---: | ---: |
| Create Mutant cell | 163,012 | 1,278,768 | 87.3% |
| Mint Spore | 258,039 | 1,505,694 | 82.9% |
| Transfer Spore | 261,975 | 1,514,761 | 82.7% |
| Burn Spore | 230,263 | 1,474,057 | 84.4% |

The recipe validates `[left, right, result]` at mint. Both examples allow Create,
Transfer and Burn without application-specific checks. Those measurements cover
startup and the applicable lifecycle/Spore validation, not additional game logic.
Mutant creation is measured separately from Spore operations.

| Rules on one Spore | Current native mint cycles | Lua baseline mint cycles | Reduction |
| --- | ---: | ---: | ---: |
| 1 | 258,039 | 1,505,694 | 82.9% |
| 2 | 418,324 | 2,915,988 | 85.7% |
| 4 | 753,937 | 5,761,408 | 86.9% |

The current no-Mutant baseline uses **99,430 mint cycles**,
**87,193 transfer cycles** and **70,092 burn cycles**.
These totals include the fixture's input lock and all applicable Spore checks;
they are not isolated child-VM instruction counts.

## Release/mainnet results

| Rules | Current native mint cycles | Lua baseline mint cycles |
| --- | ---: | ---: |
| 1 | 258,111 | 1,505,574 |
| 2 | 418,396 | 2,915,868 |
| 4 | 754,009 | 5,761,288 |

The mainnet build uses `--release --features release_export`. Both configurations
were rebuilt and measured separately; each measured transaction was verified
five times with identical cycles. Full create/transfer/burn values for both
configurations are retained in the JSON results linked below.

## Code and dependency sizes

| Component | Current native path | Lua baseline |
| --- | ---: | ---: |
| Rule per Mutant cell | 9,736-byte ELF | 594-byte source |
| Shared lifecycle/interpreter binary | 26,528 bytes | 32,424 bytes |
| Additional shared Lua library | None | 267,432 bytes |
| CellDeps in the single-rule mint fixture | 3 | 5 |
| Serialized single-rule mint transaction | 790 bytes | 864 bytes |

The ELF is larger than this Lua source. Its benefit here is reduced execution
cost and removal of the Lua runtime dependencies. Shared binaries are deployed
once and referenced through CellDeps; their bytes are not copied into every
mint transaction. File sizes exclude cell metadata and common Spore/lock code.

The current release Spore binary is 73,008 bytes in the testnet configuration,
compared with 75,088 bytes in the intermediate dual-path baseline.

## Method and limits

- Contract tools: Rust 1.92.0, Clang 22.1.8, and the checked-in release profile
  (`opt-level = 's'`, LTO, one codegen unit, overflow checks and stripped symbols).
- Verifier: `ckb-testtool 0.15.1`, `ckb-script 0.200.0`, `ckb-vm 0.24.13`, Meepo
  enabled. Each call runs the actual transaction script verifier.
- Inputs: `[3,4,7]`, no cluster or optional Mutant payment, valid co-build witness
  and `ALWAYS_SUCCESS` input locks. Rust uses `mutant_equipment`; the Lua baseline
  uses `equipment-merge.lua`. Both fixtures also reject `[3,4,8]` and a missing
  co-build witness.
- The 500-million-cycle diagnostic ceiling is not a proposed network limit. All
  measured transactions use fewer than 6 million cycles.
- These are script-verification fixtures, not full-node admission or live-chain
  tests. Production signature verification, cluster/multipart workloads, host
  timing and network throughput are outside these measurements.
- Removing Lua does not establish that historical chain usage was zero. Existing
  data-hash-bound assets retain their old binaries and original rules.

## Reproduce the current native measurements

From the repository root, build one configuration at a time:

```sh
rustup target add riscv64imac-unknown-none-elf
SPORE_BUILD_RUNNER=cargo \
CC_riscv64imac_unknown_none_elf="$PWD/scripts/clang-riscv.sh" \
SPORE_DEPLOYMENT_MANIFEST=deployment/type-ids.fixture.toml ./scripts/build-contracts.sh --release
CAPSULE_TEST_ENV=release cargo test --locked --manifest-path tests/Cargo.toml \
  native_mutant_performance_report -- --ignored --nocapture
```

For mainnet, rebuild with `--release --features release_export`, then run the
same benchmark command. The performance test is explicitly ignored during the
normal regression suite. It loads no Lua interpreter or library. Different
compiler versions or toolchains may change code hashes and cycle counts.

- [Native benchmark source](../../tests/src/tests/mutant_performance.rs)
- [Native cycle results](../../benchmarks/native-only/measurements.json)
- [Native testnet binary sizes and hashes](../../benchmarks/native-only/binaries-testnet.json)
- [Native mainnet binary sizes and hashes](../../benchmarks/native-only/binaries-mainnet.json)
- [Isolated Lua baseline and reproduction instructions](../../benchmarks/legacy-lua/README.md)

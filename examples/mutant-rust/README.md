# Native Rust Mutants

Spore now executes Rust-authored RISC-V ELF code directly from a Mutant cell's
data using CKB-VM V2 `spawn` and `wait`. No Lua bridge, companion type group,
receipt output or per-rule deployed interpreter is required for this path.

This is a **new contract version**, identified by new code hashes. Deploy the
new `spore_extension_rust`, `cluster`, and `spore` binaries and use `hash_type =
data2` for new Spore and Rust Mutant scripts. Frozen hashes, deployment records,
and the Molecule asset schemas are unchanged. Existing assets continue to use
their original Spore binaries and rules; this does not upgrade them or make
immutable content mutable. Build outputs are not claimed to match old binaries.

## Author a rule

`contracts/mutant_equipment` is a complete `no_std` Rust executable built for
`riscv64imac-unknown-none-elf`. It depends on `spore-mutant` for the versioned
invocation ABI and uses ordinary CKB cell-loading syscalls to inspect the
transaction. The entry point collects `ckb_std::env::argv()` as byte slices and
calls `spore_mutant::Invocation::parse`.

Return zero to accept an operation and a nonzero `i8` to reject it. Handle
`Create`, `Mint`, `Transfer` and `Burn` explicitly. A rule is run once when its
code cell is created, and on every mint, transfer and burn of a Spore referring
to it. Creation has no target Spore; use it for rule initialization checks.

The example checks three content bytes `[left, right, result]` at mint: both
levels must be positive, addition must fit in a byte, and the sum must equal
result. `[3,4,7]` succeeds; `[3,4,8]` and `[255,1,0]` fail. It permits transfer
and burn. This certifies a recipe in a new immutable asset; it does not consume
equipment NFTs, conserve supply, or implement a stateful NFT protocol.

## Build and use

Use the pinned Rust toolchain and the Cross configuration:

```sh
rustup target add riscv64imac-unknown-none-elf
cargo install cross --version 0.2.5 --locked
./scripts/build-contracts.sh --release
CAPSULE_TEST_ENV=release cargo test --locked --manifest-path tests/Cargo.toml
cargo test --locked -p spore-mutant --lib
```

Build order matters: `spore_extension_rust` and the legacy Lua interpreter must
exist before cluster, and cluster/agent before Spore. `capsule.toml` records this
order, as does the build script (which passes the target explicitly). The native
example and test fixture are built last. Do not build the whole contracts workspace in parallel: build scripts pin prerequisite binaries'
data hashes. Generated hash modules live in each build's `OUT_DIR`; prerequisite
binaries and frozen lists are declared build inputs, so configuration switches
recompute the appropriate pins. With a host RISC-V C compiler configured,
`SPORE_BUILD_RUNNER=cargo` selects direct Cargo builds instead of Cross.
Development ELFs strip symbols/DWARF to remain within the native code cell size limit; assertions and CKB debug output remain enabled.

1. Deploy the new native Mutant lifecycle and Spore code cells (and new cluster
   code if using native Mutants in clusters). This PR does not deploy them.
2. Create a Mutant cell with type `code_hash = spore_extension_rust data hash`,
   `hash_type = data2`, and `args = mutant_id [32 bytes] || optional mint payment
   [8-byte little-endian shannons]`. The ID is CKB-personalized Blake2b-256 of the
   transaction's first serialized input followed by the output index as a
   little-endian `u64`, as in existing Mutants. Store the built rule ELF in data.
   Include the lifecycle contract as a code dependency. Creation validates the
   ID/argument length and ELF header/size, then actually executes its Create
   entry point; malformed code or failed execution rejects creation.
3. Mint using `application/x.equipment;mutant[]=<64 lowercase hex ID>`, the
   ordinary Spore type-ID rules, input authorization and valid co-build action.
   Include the immutable Mutant cell in `CellDep`; Spore executes that exact
   cell's data. No separate rule code cell or Lua dependency is needed.
4. Include the same Mutant dependency on transfer/burn. Code, ID, and payment
   remain immutable; the Mutant cell itself can transfer with unchanged data,
   but cannot be destroyed or duplicated. Input locks authorize all spends.

Clusters declare the same Mutant ID in `ClusterDataV2.mutant_id`; creation finds
that ID in a recognized dependency, including cells with the optional payment
suffix. The cluster ID is a separate identity. Spore still verifies cluster
existence, ownership and application of the cluster's required Mutant.

## ABI v1

Every native invocation has exactly five NUL-terminated arguments:

```text
spore-mutant-v1 <create|mint|transfer|burn> <mutant_index> <input_index|-> <output_index|->
```

| Operation | Mutant index source | Input index | Output index |
| --- | --- | --- | --- |
| create | transaction Output | `-` | `-` |
| mint | resolved CellDep | `-` | Spore Output |
| transfer | resolved CellDep | Spore Input | Spore Output |
| burn | resolved CellDep | Spore Input | `-` |

Indices are full decimal `usize` values, with no single-byte encoding or `u8`
truncation. The parser rejects unknown ABI versions, extra/missing arguments,
wrong operation shapes, noncanonical/overflowing indices and negative indices.
The child inherits the invoking script's group context: `load_script()` is the
parent Spore (or lifecycle script), not the Mutant cell's type. Use the supplied
transaction indices for cell inspection, not assumptions about group indices.
The invocation parameters are validation context, not a new authorization grant.

## Execution and compatibility

The parent matches the immutable Mutant ID only under pinned lifecycle code
hashes and exact hash types. Rust cells require data2; recognized Lua cells
require data1 and keep their original decimal interpreter ABI. Lua remains
available for compatibility; new native rules do not depend on it.

Each rule runs in a fresh child VM. Spore waits for its exit and rejects any
nonzero result or spawn/wait failure. It runs **every** declared Mutant before
continuing multipart, cluster and co-build validation. The prior successful
`exec` replacement/early-exit behavior is removed in this new Spore version,
including for legacy Lua rules. Mint payments accumulate per payee across the Mutants of each Spore script
group, with checked arithmetic for thresholds and capacity sums. The existing
per-group model is retained; no cross-Spore payment accounting is added.

The new path requires Meepo/CKB-VM V2. It uses the final asynchronous spawn/wait
ABI, not the obsolete memory-limit/content-return prototype exposed by old
`ckb-std` releases. The small `spore-mutant` library pins `ckb-std 0.16.4` without
its optional runtime/type features for these syscalls; existing contract SDK
and schema dependencies remain separate. The VM integration harness uses
`ckb-testtool 0.15.1` / `ckb-script 0.200.0`, with Meepo enabled.

See the [VM syscalls specification](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0050-vm-syscalls-3/0050-vm-syscalls-3.md)
and [pinned Rust syscall implementation](https://github.com/nervosnetwork/ckb-std/blob/v0.16.4/src/syscalls/native.rs).

Code cells are bounded to 256 KiB. The example bounds target data to 4096 bytes
and uses checked byte arithmetic. Children share the transaction cycle budget;
an infinite loop exhausts it. Child memory is isolated by CKB-VM. No host code
execution, writable asset state or child-supplied transaction mutation is added.
Validation does not prove every arbitrary rule is correct: rule authors must
check the inputs and operations their application requires.

## Verification

The VM integration tests cover native creation and immutable lifecycle, recipe
acceptance/rejection, deterministic replay, mint/transfer/burn rejection, mixed
Rust/Lua and multiple rules, full-width dependency/output indices, code/type/ID
binding, payment accumulation/overflow, cycle exhaustion, and continued Spore
multipart, cluster, co-build and input-lock checks. `mutant_probe` is a test-only
executable and has no deployment configuration. `equipment-merge.lua` is retained
as a legacy Lua recipe reference.

No live deployment or production wallet/signature-lock integration is included.
There is no performance claim based on the old companion PoC measurements;
that architecture and its cycle table no longer describe this implementation.

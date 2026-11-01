# Rust-authored Mutant rules through a companion type script

This is an opt-in, native RISC-V **companion example**, not a new execution
backend inside the Mutant contract. Lua remains required. No existing contract,
serialized protocol structure, frozen artifact, or deployment record changes.

## Architecture assessment

Source baseline: `9a6010e43ffd09ac9e6fe7ee389cc915e3e4f999`.

- [Spore RFC](../../docs/RFC.md) requires immutable content and identity. A rule
  can validate a new asset, but cannot rewrite an existing Spore's content.
- [Mutant RFC](../../docs/MUTANT.md) describes Lua cells and an older proposed
  `exec` interface. It is not the current ABI: in
  [verify_extension](../../contracts/spore/src/entry.rs), Spore resolves a
  Mutant ID only against `MUTANT_CODE_HASHES`, checks mint payments, and calls
  `exec_cell` with operation, dependency index, and Spore input/output indices.
  [spore/build.rs](../../contracts/spore/build.rs) constructs this allowlist from
  frozen hashes and the locally built Lua Mutant. Arbitrary type scripts are
  not already supported as Mutants.
- [Lua entry](../../contracts/spore_extension_lua/src/entry.rs) dynamically loads
  the pinned `libckblua.so`, resolves `lua_create_instance` and `lua_run_code`,
  and interprets source from the Mutant dependency's data. Its actual Lua modes
  are **0 = create Mutant, 1 = mint Spore, 2 = transfer Spore, 3 = melt Spore**.
  The handoff's operation arguments are 0/1/2, respectively. Indices are encoded
  by Spore as single bytes plus ASCII `0`, then parsed as decimal by Lua's host;
  use indices 0–9 for this example. This PR does not repair that inherited ABI.
- `ckb-std 0.14.3` [high_level.rs](https://docs.rs/crate/ckb-std/0.14.3/source/src/high_level.rs)
  returns `Result<Infallible, SysError>` from `exec_cell`: success never resumes
  the caller. In `ckb-script 0.111.0` (the test suite's locked verifier),
  [exec.rs](https://docs.rs/crate/ckb-script/0.111.0/source/src/syscalls/exec.rs)
  resets the VM, preserving consumed cycles. This is a program replacement,
  not a nested call, subprocess, or isolated dynamic-library invocation.
- The locked verifier's
  [verify.rs](https://docs.rs/crate/ckb-script/0.111.0/source/src/verify.rs)
  selects VM V1 for `Data1`; exec is available, spawn is registered only for V2.
  `ckb-std`'s `ckb2023`-gated spawn API cannot simply be used on this Data1 path.
  The bundled Lua library has no exec/spawn binding (its exported API strings
  include cell-loading functions). Newer Lua source or newer CKB syscalls do not
  retroactively change that binary. The CKB VM and verifier provide the actual
  syscall/security boundary; compiling Rust for RISC-V alone grants no invocation
  mechanism. The locked VM uses a 4 MiB memory limit and cycle metering.

**Conclusion:** native Rust validation is feasible in a separate CKB script
group. A backend added to the current contract would change its code hash; Spore
would also need a new build/hash to recognize it. Instead, the existing Lua API
can require an output type script, causing CKB to execute a separately deployed
Rust validator. That needs only a new companion code hash and a new opt-in Lua
Mutant cell. Existing Spore and Lua binaries/hashes remain unchanged with the
same compiler and build configuration. Other languages can use this composition
if they compile to a supported CKB executable; this is not a generic backend ABI.

Relevant upstream history: [#10](https://github.com/sporeprotocol/spore-contract/pull/10)
introduced Lua Mutants; [#62](https://github.com/sporeprotocol/spore-contract/pull/62)
fixed exec loading after [#25](https://github.com/sporeprotocol/spore-contract/issues/25);
[#66](https://github.com/sporeprotocol/spore-contract/pull/66) adjusted handoff opcodes;
[#31](https://github.com/sporeprotocol/spore-contract/issues/31) tracks missing
Mutant lifecycle/complex-script tests, and
[#67](https://github.com/sporeprotocol/spore-contract/pull/67) addresses lifecycle
tests. These are historical references, not claims about their current status.

## Example and transaction layout

The recipe content is exactly three bytes `[left_level, right_level, result]`.
Both levels must be positive and their sum must fit in a byte and equal result.
For example `[3, 4, 7]` succeeds; `[3, 4, 8]` and `[255, 1, 0]` fail.
This certifies a recipe in a **new** immutable Spore. It does not consume two
equipment NFTs, conserve equipment supply, or authorize a game-state update.

1. Build with `capsule build --release`. Deploy `build/release/mutant_equipment`
   as a code cell. Compute its CKB-personalized Blake2b data hash (not a standard
   unpersonalized Blake2b digest).
2. Replace `@CODE_HASH@` in [require-rust.lua](require-rust.lua) with that
   64-character lowercase hex data hash. Store the result in an ordinary Lua
   Mutant cell, created using the existing Mutant contract and type-ID rules.
   Mode 0 permits this creation without a receipt.
3. Mint a Spore using `application/x.spore-equipment;mutant[]=<mutant ID>` and
   content `0x030407`. Include the normal Spore/Lua code deps, Lua library and
   Mutant cell, plus the native companion code cell. Supply the ordinary mint
   co-build action and normal input lock authorization.
4. Add an empty-data receipt output with type script:

   ```text
   code_hash = mutant_equipment binary data hash
   hash_type = data1
   args      = full 32-byte type-script hash of the new Spore output
   ```

The Lua bridge requires this exact output script, including hash type and target.
An input, dependency, unrelated receipt, or another backend does not satisfy it.
CKB must validate all type and input-lock groups for the transaction to succeed;
the receipt contract returns a nonzero code on failure. Receipt locks are chosen
by the transaction author. Receipts carry no state or authority and can be spent
without recreation; any recreated output must validate a target again.

[equipment-merge.lua](equipment-merge.lua) implements the same recipe entirely
in Lua. The integration tests render the bridge from the actual binary hash and
construct both transactions; no manually supplied deployment hash is needed.

## Security and compatibility boundaries

- No host code execution, nested execution, new syscalls, native pointer ABI,
  recursive dispatch, or user-supplied executable payload. The companion is an
  ordinary RISC-V CKB type script, selected by an immutable code data hash.
- The native search is bounded to 256 output indices; target data is capped at
  4096 bytes, parsed with the existing Molecule reader, and the payload is exactly
  three bytes. Overflow rejects. CKB's transaction cycle budget bounds all script
  groups together. The bridge scans at most 256 outputs. This is not a per-rule
  cycle allowance or new memory-isolation guarantee.
- The companion's target argument is a script hash, not an output index. The
  bridge binds it to the actual Spore currently being minted. The companion alone
  is a data validator; it does not establish that an arbitrary target is a Spore
  or grant ownership. Use the bridge together with the Spore type script.
- This example adds a mint restriction only. Transfers and melts retain existing
  Lua/Spore behavior and do not need new receipts. Spore content and identity stay
  immutable; existing locks authorize input spends. Production signature-lock
  integration is not implemented by the example's test fixtures.
- **Inherited limitation:** Spore calls `verify_extension` before mint cluster,
  multipart and co-build checks; successful exec cannot return to those checks.
  Likewise only the first successfully executed Mutant runs, and post-exec
  co-build checks on transfer/melt are not reached. This PR neither introduces
  nor repairs that behavior. Do not rely on those skipped checks or claim this
  example establishes cluster/co-build/multiple-Mutant security. Use one Mutant,
  no cluster and non-multipart content for the example. A production backend
  redesign must address these control-flow issues separately.

No SDK integration, deployment automation, new backend selector, shared-library
framework, source-to-RISC-V compiler service, Morph, Myelin, asset consumption,
or protocol migration is included. Possible follow-ups are an SDK transaction
builder and independently reviewed companion examples in other languages.

## Validation and measured costs

Validated with Rust 1.74.0, target `riscv64imac-unknown-none-elf`, Capsule
0.10.2 (5e49142), GCC 12.2.0 from the configured
`nervos/ckb-riscv-gnu-toolchain:focal-20230214` image, and the committed lockfiles
(`ckb-testtool 0.10.0`, `ckb-script 0.111.0`, `ckb-vm 0.24.6`). No emulator mocks
or host-native substitutes are used by these tests.

| Command | Result |
| --- | --- |
| `capsule build --release` | Pass, all six contracts |
| `capsule test --release` | 64 passed, 0 failed, 1 existing ignored test |
| `capsule build` | Pass, all six contracts |
| `capsule test` | 64 passed, 0 failed, 1 existing ignored test |
| Upstream baseline release build/test at the commit above | 49 passed, 0 failed, 1 ignored |
| `rustfmt --edition 2021 --check` on the new Rust files; `git diff --check` | Pass |

The ignored test is the pre-existing networked deployment-transaction generator.
No network deployment, live-node verification, mainnet feature build, or
production signature-lock test was performed.

The environment initially had no Rust/CKB tools. Cross 0.2.5 fell back to Cargo
on the host and did not honor the configured default target; the initial build
failed on the host target, then on the missing cross compiler. The successful
Capsule runs used a local Cross wrapper adding
`--target riscv64imac-unknown-none-elf`, with the configured image's cross
compiler on `PATH`. These environment workarounds are not repository changes.
An initial test attempt before building had missing-binary failures; all final
runs above used successfully built RISC-V artifacts.

The 15 added integration tests cover both recipe implementations, real Lua
Mutant creation, deterministic repeated execution, overflow/zero/malformed
payloads, exact backend/target binding, receipt presence/uniqueness/data,
code-hash substitution, receipt disposal/recreation/replay, invalid ELF rejection,
malformed Molecule, missing target, bad arguments,
oversized target data, transfer/melt compatibility, immutable content, rejecting
input locks, insufficient cycles, and a Lua infinite loop exhausting the budget.

Successful execution output from the full suite (representative single runs):

```text
# release
equipment [3,4,7] lua: 1367776 total transaction cycles
equipment [3,4,7] rust-companion: 1627556 total transaction cycles
test tests::mutant_rust::equipment_lua_and_rust_execution_and_cycles ... ok
test result: ok. 64 passed; 0 failed; 1 ignored
# debug
equipment [3,4,7] lua: 2986133 total transaction cycles
equipment [3,4,7] rust-companion: 3328038 total transaction cycles
test result: ok. 64 passed; 0 failed; 1 ignored
```

These are **whole-transaction** costs, including Spore, Lua, locks and, in the
Rust case, bridge/receipt validation and its extra code dep. They are equivalent
recipe workloads, not isolated language/interpreter benchmarks. Test fixtures
generate different transaction IDs between runs; exact totals vary slightly.
Within each transaction the test repeats verification and requires equal cycles.
The Rust companion path costs more in this small example; no general language
performance claim follows.

The stripped release companion is **11,984 bytes**, an ordinary statically linked
RISC-V ELF. The unchanged Lua library is 267,432 bytes and the unchanged Lua
Mutant contract is 31,496 bytes. The Lua-only recipe source is 594 bytes; the
rendered bridge is 1047 bytes. Native use adds one deployed code cell/dependency,
one opted-in Lua Mutant, and one empty-data receipt output per target Spore.
It does not remove Lua deployment/storage costs. Debug binaries retain symbols
and are test artifacts, not recommended deployment artifacts.

Memory figures are source/static-layout observations, **not measured peak usage**:
the native validator uses a 4096-byte target buffer and ckb-std's default
4 KiB fixed + 516 KiB dynamic heap reservation in its separate VM group. The
existing Lua host requests a 270 KiB loader context and a 500 KiB Lua arena.
No peak-memory profiler or per-rule cycle accounting is added.

Rebuilding the untouched upstream worktree with the same toolchain produced
byte-identical `spore`, `cluster`, `cluster_proxy`, `cluster_agent`,
`spore_extension_lua`, and `libckblua.so` release artifacts. Thus their data hashes
do not change in this build. Frozen deployments and serialized schemas are
untouched. Reproducibility with other toolchains is not implied.

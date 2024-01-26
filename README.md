# <p align="center"> Spore Protocol</p>
<p align="center">
  A protocol for valuing on-chain contents, build on top of <a href="https://github.com/nervosnetwork/ckb">CKB</a>. Check <a href="https://docs.spore.pro">Spore Docs</a> for a quick start.
</p>

Developers are recommended to use the [Spore SDK](https://github.com/sporeprotocol/spore-sdk) for building applications with Spore, instead of directly interacting with the contracts. If you're interested in testing, extending the contracts, or deploying them on a local blockchain, feel free to explore the [Development](#⚙️-development) section for guidance.

## About

Spore is an on-chain protocol to power digital asset ownership, distribution, and value capture. For more, check out https://spore.pro

This repo contains the [Spore RFC](./docs/RFC.md), [Spore Mutant RFC](./docs/MUTANT.md), [Spore Cluster Proxy/Agent RFC](./docs/RFC_PROXY_AGENT.md), protocol types [schema definition](./lib/types/schemas/spore.mol) and [implementation](./contracts/) of Spore contracts written in Rust.


## ⚙️ Development
Before development, basic dependencies are required:

- [Rust](https://www.rust-lang.org/tools/install)
- [Cross](https://github.com/cross-rs/cross)
- [Capsule](https://github.com/nervosnetwork/capsule)

To build contracts, please run one of below commands:

```bash
$ capsule build # build in debug mode
$ capsule build --release # build in release mode for testnet
$ capsule build --release -- --features release_export # build in release mode for mainnet
```

To check native test cases, which are placed in [tests](./tests/), please run:

```bash
$ capsule test # test in debug mode
$ capsule test --release # test in release mode
```

### Writing extra contracts

If your development requires to be built on Spore, steps below are recommended:

1. Writing a new Type contract
2. Modify existed contracts

For method 1, the flow is:

1. Run `capsule new-contract YOUR_CONTRACT_NAME` to initialize a new contract project
2. Modify `Cargo.toml` of your contract, and introduce `spore-types`,`spore-utils`, `spore-constant`
3. Implementing your contract rules in `entry.rs`
4. Writing new tests in `tests/src/tests.rs`. See existed test cases for how-to

## Deployed Code Hashes
The versioning philosophy is **"Using code_hash as version"** for Spore Protocol, which means the different code hash matches the different version.

Make sure you are using the proper version you want, because there's no such an "upgrade/downgrade" method but we suggest to use "destroy/reconstruct" method instead, which requires no modification of any fields in Spore cell.

Our `forzen` versions of contract, which are our prior versions, can be found in [directory](https://github.com/sporeprotocol/spore-contract/tree/master/deployment/frozen) `./deployment/frozen`. To describe more clearly, the `frozen` information contains each avaliable `code_hash` generated from Spore contracts. Historical version refers to [here](https://github.com/sporeprotocol/spore-contract/blob/master/docs/VERSIONS.md).

In addition, using Mutant contract requires the binary of Lua library. Information are recorded [here](https://github.com/sporeprotocol/spore-contract/tree/master/contracts/spore_extension_lua/lua). For simplicity, it's already deployed in the Pudge Testnet:

#### Spore Lua Lib
- tx_hash: `0x8fb7170a58d631250dabd0f323a833f4ad2cfdd0189f45497e62beb8409e7a0c`
- index: `0`
- data_hash: `0xed08faee8c29b7a7c29bd9d495b4b93cc207bd70ca93f7b356f39c677e7ab0fc`

## Deployment

We provided a simple bash script to operate deployment through `ckb-cli` toolchain, details refer to [here](https://github.com/sporeprotocol/spore-contract/tree/master/deployment)

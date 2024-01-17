# <p align="center">🧬 Spore Protocol</p>
<p align="center">
  A protocol for valuing on-chain contents, build on top of <a href="https://github.com/nervosnetwork/ckb">CKB</a>. Check <a href="https://docs.spore.pro">Spore Docs</a> for a quick start.
</p>


Spore developers are supposed to use [Spore SDK](https://github.com/sporeprotocol/spore-sdk) instead of this project directly. But if you want to test/extend the contract or deploy on a local chain, you can follow the [Development](#⚙️-development) part.

## About

Spore is an on-chain protocol to power digital asset ownership, distribution, and value capture. For more, check out https://spore.pro

This repo contains the [Spore RFC](./RFC.md), protocol types [schema definition](./lib/types/schemas/spore.mol) and [implementation](./contracts/) of Spore Type contract written in Rust.


## ⚙️ Development
To start developing this contract, you'll need:

- [Rust](https://www.rust-lang.org/tools/install)
- [Cross](https://github.com/cross-rs/cross)
- [Capsule](https://github.com/nervosnetwork/capsule)

To build contracts, run:

``` sh
capsule build
```

Test cases are located in [tests](./tests/). To run test cases:

``` sh
capsule test
```

### Writing extra contracts

If you want to extend spore contracts, you can achieve it by:

1. Writing a new Type contract
2. Modify existed contracts.

For method 1, the flow is:

1. Run `capsule new-contract YOUR_CONTRACT_NAME` to init a new contract
2. Modify `Cargo.toml` of your contract, introduce `spore-types`,`spore-utils`, `spore-constant`
3. Implementing your contract rules in `entry.rs`
4. Writing new tests in `tests/src/tests.rs`. See existed test cases for how-to.


## Deployed Code Hashes
The versioning philosophy is **"Using code_hash as version"** while developing with Spore Protocol.

A different code hash means a different version of Spore Protocol.

Make sure you are using the proper version you want, because there's no such "upgrade/downgrade" method to use, the only way to achieve this in a similiar result is to destroy and reconstruct a new Spore with same fields. 

Our `forzen` versions of contract, which means our prior versions, can be found in [directory](https://github.com/sporeprotocol/spore-contract/tree/master/deployment/frozen) `./deployment/frozen`. And to be more clear, the `frozen` information contains each avaliable `code_hash` generated from each Spore contracts we deployed before, with their corresponding commit hash as an indicator field.

`./deployment/migration` stores the deployment detail for each Spore contracts. Here's a list of newest version of `code_hash`, which are aslo recorded in [migration](https://github.com/sporeprotocol/spore-contract/tree/feat/complete-test-cases/deployment/migration) directory:

### Spore
Pudge Testnet:
- data_hash: `0xfd2dc714c4d4cb81e8621e5c124465a048d06551b467f58eaa64041dd322cf81`

### Cluster
Pudge Testnet:
- data_hash: `0x372b7c11d7b688e02d9c2b7604fbdf0dc898a0f6741854ea6c65d41f8ef4a64e`

### Cluster Proxy
Pudge Testnet:
- data_hash: `0xfc1fbe95e7fb5be520f1adb2bdbd1529422613b02254ff01fd0f30604861ae36`

### Cluster Agent
- data_hash: `0xa170fc93235213e90214e4273bb283e7979bf6477f70b4f2319d3777ec36235c`

### Mutant (Lua Extension)
Pudge Testnet:
- data_hash: `0x94a9b875911ace20f1f0d063a26495d14e4b04e32fd218261bb747f34e71ae47`

In addition, using Mutant contract requires the Lua library file. Information are recorded [here](https://github.com/sporeprotocol/spore-contract/tree/master/contracts/spore_extension_lua/lua), and we've already deployed in the Pudge Testnet:

### Spore Lua Lib
- tx_hash: `0x8fb7170a58d631250dabd0f323a833f4ad2cfdd0189f45497e62beb8409e7a0c`
- index: `0`
- data_hash: `0xed08faee8c29b7a7c29bd9d495b4b93cc207bd70ca93f7b356f39c677e7ab0fc`

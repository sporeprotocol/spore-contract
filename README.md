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

Make sure you are using the proper version you want, because there's no such "upgrade/downgrade" method to use, the only way to achieve this in a similiar result is destroy and reconstruct a new Spore with same fields. 

Here are lists of Spore protocol contract cells code_hashes (Top item is the latest, bottom item is the oldest):

### Spore
Pudge Testnet:
- data_hash: `0x56f5dbbafccf025c2fde98fda20498dc98245a0a28fce2db190cd24cc3636c6d`
- data_hash: `0xbbad126377d45f90a8ee120da988a2d7332c78ba8fd679aab478a19d6c133494`

### Cluster
Pudge Testnet:
- data_hash: `0x15f835c4ca0b861df38f10d4e95c51ba9cee3c89f178b21e2e28baa67ebd8b42`
- data_hash: `0x598d793defef36e2eeba54a9b45130e4ca92822e1d193671f490950c3b856080`

### Mutant (Lua Extension)
Pudge Testnet:
- data_hash: `0xb4d3f207831e2774d310a87571fb0095f5b4af4fa176d8bfaae0191a4d6989c8`

### Spore Lua Lib
- tx_hash: `0x8fb7170a58d631250dabd0f323a833f4ad2cfdd0189f45497e62beb8409e7a0c`
- index: `0`
- data_hash: `0xed08faee8c29b7a7c29bd9d495b4b93cc207bd70ca93f7b356f39c677e7ab0fc`

### Cluster Proxy
Pudge Testnet:
- data_hash: `0x428457c447f0200e302c3b64f0ee0c165b759e9d3b98118c55710bf2f294a7c2`

### Cluster Agent
- data_hash: `0x1c6296a5a0aa3cdb50c9f9e6c713c28c2e1dff5c826d84d4dbe5d35cc307bb6f`
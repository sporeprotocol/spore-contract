<br/>
<p align="center">
  🧬 Spore Protocol
</p>
<p align="center">
  A protocol for valuing on-chain contents, build on top of <a href="https://github.com/nervosnetwork/ckb">CKB</a>.
</p>


## About

Spore is an on-chain protocol to power digital asset ownership, distribution, and value capture. For more, check out https://spore.pro

This repo contains the [Spore RFC](./RFC.md), protocol types [schema definition](./lib/types/schemas/spore.mol) and [implementation](./contracts/) of Spore Type contract written in Rust.


##  Quick Start

Spore developers are supposed to use [Spore SDK](https://github.com/sporeprotocol/spore-sdk) instead of this project directly. But if you want to test/extend the contract or deploy on a local chain, you can follow the [Development](#⚙️-development) part.

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
4. Writing new tests in `tests/src/tests.rf`. See existed test cases for how-to.


## Deployed Code Hashes
The versioning philosophy is **"Using code_hash as version"** while developing with Spore Protocol.

A different code hash means a different version of Spore Protocol.

Make sure you are using the proper version you want, because there's no such "upgrade/downgrade" method to use, the only way to achieve this in a similiar result is destroy and reconstruct a new Spore with same fields. 

Here are lists of Spore protocol contract cells code_hashes (Top item is the latest, bottom item is the oldest):

### Spore
Pudge Testnet:
- `0xbbad126377d45f90a8ee120da988a2d7332c78ba8fd679aab478a19d6c133494`

### Cluster
Pudge Testnet:
- `0x598d793defef36e2eeba54a9b45130e4ca92822e1d193671f490950c3b856080`
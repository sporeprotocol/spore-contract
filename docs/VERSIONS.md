# Contract Versions

## Spore v3 / 0.3.0-alpha.1 (unreleased)

The proposed next contract generation is **Spore v3**, with source package
version **`0.3.0-alpha.1`**. This is an unreleased candidate, not a deployment or
an official Git tag. It requires both changes below before a release is built:

| Change | Review | Responsibility |
| --- | --- | --- |
| Native-only Rust/RISC-V Mutants | [#75](https://github.com/sporeprotocol/spore-contract/pull/75) | Execute every declared rule with returning `spawn/wait`; reject Lua on the new path |
| Optional CoBuild intent | [#76](https://github.com/sporeprotocol/spore-contract/pull/76) | Validate cells without requiring Actions; leave optional intent to tooling and each Lock's own rules |

The PRs remain separate for review. Neither intermediate branch is the complete
v3 release: #75 alone still requires core CoBuild Actions, while #76 alone uses
the upstream legacy executor. The combined candidate is **native-only with no
mandatory core CoBuild Action**. Build and test that combined tree before
recording deployment hashes. An integration branch is maintained at
[`codex/spore-v3-integration`](https://github.com/a19q3/spore-contract/tree/codex/spore-v3-integration).

### Why v3 rather than v2?

The [upstream SDK configuration](https://github.com/sporeprotocol/spore-sdk/blob/beta/packages/core/src/config/predefined.ts)
already labels the `0.2.2-beta.1` mainnet and `0.2.2-beta.2` testnet contract
hashes listed below as `v2`. Reusing that generation name would confuse clients.
The existing repository tags run from `0.1.0` through `0.2.2-beta.2`; the candidate
continues that source-version series at `0.3.0-alpha.1`.

These version labels describe different things:

| Label | Meaning |
| --- | --- |
| Spore v3 | Proposed protocol/contract generation |
| `0.3.0-alpha.1` | Unreleased source package version for current contracts and libraries; historical Lua package stays unchanged |
| `hash_type: type` | Standard code Type ID reference; Meepo activation provides VM2 for native execution |
| `spore-mutant-v1` | Native invocation ABI, unchanged by this release label |
| `ClusterDataV2` | Existing cell schema name; no schema rename or reinterpretation is implied |

The candidate now uses Type ID-bound code with deployment-supplied governance Locks. See [type binding and deployment plans](./TYPE_BINDING.md). Earlier data2 evidence remains attached to its recorded revision.

The semantic version does not select a contract on chain. Clients must still
bind the exact deployed code hash, hash type and dependency. Existing cells
bound by `data`/`data1` hashes keep their original binaries, Lua dependencies
where applicable, and validation rules. New contracts do not upgrade or convert
those cells. Historical sources, deployment records and frozen hash lists stay
unchanged. Do not infer that every historical deployment uses a data hash;
clients must resolve the script binding they actually selected.

No v3 deployment, release tag or published SDK configuration is created by
these PRs. Deployment records must be added only after the combined candidate
has been reviewed, built and deployed.

## Historical deployments

## tag 0.2.1 (testnet)
**⚠️ Deprecated !!! This early beta release has been phased out. For optimal ecosystem compatibility, please upgrade to version 0.2.2-beta.2 or later.**

**spore**:
| | |
|-|-|
|tx_hash|0x06995b9fc19461a2bf9933e57b69af47a20bf0a5bc6c0ffcb85567a2c733f0a1|
|index|0|
|code_hash|0x5e063b4c0e7abeaa6a428df3b693521a3050934cf3b0ae97a800d1bc31449398|

**cluster**:
| | |
|-|-|
|tx_hash|0xfbceb70b2e683ef3a97865bb88e082e3e5366ee195a9c826e3c07d1026792fcd|
|index|0|
|code_hash|0x7366a61534fa7c7e6225ecc0d828ea3b5366adec2b58206f2ee84995fe030075|

**cluster_agent**:
| | |
|-|-|
|tx_hash|0x53fdb9366637434ff685d0aca5e2a68a859b6fcaa4b608a7ecca0713fed0f5b7|
|index|0|
|code_hash|0xc986099b41d79ca1b2a56ce5874bcda8175440a17298ea5e2bbc3897736b8c21|

**cluster_proxy**:
| | |
|-|-|
|tx_hash|0x0231ea581bbc38965e10a2659da326ae840c038a9d0d6849f458b51d94870104|
|index|0|
|code_hash|0xbe8b9ce3d05a32c4bb26fe71cd5fc1407ce91e3a8b9e8719be2ab072cef1454b|

## tag 0.2.2-beta.1 (mainnet)

**spore**:
| | |
|-|-|
|tx_hash|0x96b198fb5ddbd1eed57ed667068f1f1e55d07907b4c0dbd38675a69ea1b69824|
|index|0|
|code_hash|0x4a4dce1df3dffff7f8b2cd7dff7303df3b6150c9788cb75dcf6747247132b9f5|

**cluster**:
| | |
|-|-|
|tx_hash|0xe464b7fb9311c5e2820e61c99afc615d6b98bdefbe318c34868c010cbd0dc938|
|index|0|
|code_hash|0x7366a61534fa7c7e6225ecc0d828ea3b5366adec2b58206f2ee84995fe030075|

## tag 0.2.2-beta.2 (testnet)

**spore**:
| | |
|-|-|
|tx_hash|0x5e8d2a517d50fd4bb4d01737a7952a1f1d35c8afc77240695bb569cd7d9d5a1f|
|index|0|
|code_hash|0x685a60219309029d01310311dba953d67029170ca4848a4ff638e57002130a0d|

**cluster**:
| | |
|-|-|
|tx_hash|0xcebb174d6e300e26074aea2f5dbd7f694bb4fe3de52b6dfe205e54f90164510a|
|index|0|
|code_hash|0x0bbe768b519d8ea7b96d58f1182eb7e6ef96c541fbd9526975077ee09f049058|

**cluster_agent**:
| | |
|-|-|
|tx_hash|0x52210232292d10c51b48e72a2cea60d8f0a08c2680a97a8ee7ca0a39379f0036|
|index|0|
|code_hash|0x923e997654b2697ee3f77052cb884e98f28799a4270fd412c3edb8f3987ca622|

**cluster_proxy**:
| | |
|-|-|
|tx_hash|0xc5a41d58155b11ecd87a5a49fdcb6e83bd6684d3b72b2f3686f081945461c156|
|index|0|
|code_hash|0x4349889bda064adab8f49f7dd8810d217917f7df28e9b2a1df0b74442399670a|

**spore_extension_lua**:
| | |
|-|-|
|tx_hash|0x9b2098e5b6f575b2fd34ffd0212bc1c96e1f9e86fcdb146511849c174dfe0d02|
|index|0|
|code_hash|0x5ff1a403458b436ea4b2ceb72f1fa70a6507968493315b646f5302661cb68e57|

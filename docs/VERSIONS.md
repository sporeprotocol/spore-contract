# Historical Versions

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

### Current Spore and Cluster deployment

The same 0.2.2-beta.2 binaries were redeployed on CKB Testnet (Pudge) on
2026-11-25 with unchanged data hashes. Both cells were committed in block
23176511 at 2026-11-25 13:39:27 Asia/Shanghai (UTC+8) and verified live.

Both cells have no Type ID and use the official CKB genesis Burn lock with
`code_hash` `0x0000000000000000000000000000000000000000000000000000000000000000`,
`hash_type` `data`, and `args` `0x62e907b15cbf27d5425399ebf6f0fb50ebb88f18`.
Clients use the data hash as `code_hash`, with script `hash_type` `data1` and
cell dependency `dep_type` `code`.

**spore** ([migration record](../deployment/migration/testnet/spore/2026-11-25-0.2.2-beta.2.json))

| | |
|-|-|
|tx_hash|0xe71e3d378ab589685cafb9ea7da6bd42c9b0b770016d32b727a1e4118ac8d680|
|index|0|
|code_hash|0x685a60219309029d01310311dba953d67029170ca4848a4ff638e57002130a0d|
|occupied capacity|75853 CKB|

**cluster** ([migration record](../deployment/migration/testnet/cluster/2026-11-25-0.2.2-beta.2.json))

| | |
|-|-|
|tx_hash|0xe71e3d378ab589685cafb9ea7da6bd42c9b0b770016d32b727a1e4118ac8d680|
|index|1|
|code_hash|0x0bbe768b519d8ea7b96d58f1182eb7e6ef96c541fbd9526975077ee09f049058|
|occupied capacity|46589 CKB|

### Other Testnet contracts

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

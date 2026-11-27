# Legacy deployment

Use this helper to deploy the exact binaries from a published legacy release.
For V3, follow the [Type ID deployment procedure](../docs/TYPE_BINDING.md#explicit-deployment-plan).

| Network | Legacy source commit | Contracts |
| --- | --- | --- |
| Testnet | `9e8663b3ef6e277a58acca9f4f347e5dbf8d5c06` (`0.2.2-beta.2`) | `spore`, `cluster`, `cluster_agent`, `cluster_proxy`, `spore_extension_lua` |
| Mainnet | `3c796b6c02e5d3857bbaa42476adcb23e1c88722` (`0.2.2-beta.1`) | `spore`, `cluster` |

Use a Git checkout of this helper that includes the published testnet release
commit. Check out the network's legacy commit separately with no tracked
changes. Stage the corresponding release binary at `build/release/<contract>`
in that legacy checkout. Use the recorded code Cell bytes or reproduce them
with `capsule build --release` for testnet and
`capsule build --release -- --features release_export` for mainnet. A rebuild
must match the data hash in the published migration record byte for byte.

Set the intended output Lock in `toml/<network>/<contract>.toml` and supply the
funding address for that network. With Git, Python 3.11+ and `ckb-cli` installed,
run the helper from the root of the deployment checkout.

```bash
SPORE_LEGACY_CHECKOUT=/path/to/legacy-checkout \
  ./deployment/deploy.sh <contract> <rpc-url> <funding-address> [testnet|mainnet]
```

The helper defaults to testnet. It checks the legacy source revision, validates
the recipe, and compares the executable's CKB data hash with the published
record before invoking `ckb-cli`. The signing step uses a private copy of the
verified recipe and executable. A failed submission reports the path to the
retained signed transaction for recovery.

# Deployment

usage:
```bash
$ capsule build --release # build for testnet
$ capsule build --release -- --features release_export # build for mainnet
$ cd deployment
$ ./deploy.sh <contract-name> <ckb-url> <ckb-address>
```

for example:
```bash
$ ./deploy.sh spore https://testnet.ckbapp.dev/ ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq28phxutezqvjgfv5q38gn5kwek4m9km3cmajeqs
```

before deployment, please make sure your `<ckb-address>` matches `args` setting in deployment toml [files](https://github.com/sporeprotocol/spore-contract/tree/master/deployment/toml).

taking [cluster_agent](https://github.com/sporeprotocol/spore-contract/blob/master/deployment/toml/cluster_agent.toml) for example:
```toml
[[cells]]
name = "cluster_agent"
enable_type_id = true
location = { file = "../build/release/cluster_agent" }

[lock]
code_hash = "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8"
args = <your-wallet-lock-args>
hash_type = "type"
```

notice: `frozen` versions are always containing all of deployed contracts except the latest one

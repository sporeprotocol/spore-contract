#!/usr/bin/env bash
set -euo pipefail

fail() {
    printf '%s\n' "$*" >&2
    exit 1
}

if [[ $# -lt 3 || $# -gt 4 ]]; then
    fail "Usage: SPORE_LEGACY_CHECKOUT=/path/to/legacy ./deploy.sh CONTRACT RPC_URL ADDRESS [testnet|mainnet]"
fi

deployment_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
contract=$1
rpc_url=$2
address=$3
network=${4:-testnet}

# This commit contains the published testnet and mainnet deployment records.
testnet_commit=9e8663b3ef6e277a58acca9f4f347e5dbf8d5c06
case "$network/$contract" in
    testnet/spore|testnet/cluster|testnet/cluster_agent|testnet/cluster_proxy|testnet/spore_extension_lua)
        source_commit=$testnet_commit
        record=2024-03-16-0.2.2-beta.2.json
        ;;
    mainnet/cluster|mainnet/spore)
        source_commit=3c796b6c02e5d3857bbaa42476adcb23e1c88722
        if [[ $contract == cluster ]]; then
            record=2024-01-23-0.2.2-beta.1.json
        else
            record=2024-01-25-0.2.2-beta.1.json
        fi
        ;;
    *) fail "No approved legacy artifact for $network/$contract" ;;
esac

python3 "$deployment_dir/verify-chain.py" "$network" "$rpc_url"
legacy_checkout=$(cd "${SPORE_LEGACY_CHECKOUT:?set SPORE_LEGACY_CHECKOUT to the pinned legacy checkout}" && pwd -P)
if [[ $(git -C "$legacy_checkout" rev-parse --show-toplevel) != "$legacy_checkout" ||
      $(git -C "$legacy_checkout" rev-parse --verify HEAD) != "$source_commit" ]] ||
    ! git -C "$legacy_checkout" diff --quiet HEAD --; then
    fail "Legacy source for $network/$contract must be an unmodified checkout of $source_commit"
fi

stage=$(mktemp -d)
stage=$(cd "$stage" && pwd -P)
trap 'rm -rf -- "$stage"' EXIT

git --no-replace-objects -C "$deployment_dir" show \
    "$testnet_commit:deployment/migration/$network/$contract/$record" > "$stage/record.json"

python3 - "$stage" "$contract" "$deployment_dir/toml/$network/$contract.toml" \
    "$legacy_checkout/build/release/$contract" <<'PY'
import hashlib
import json
from pathlib import Path
import sys
import tomllib

stage, contract, recipe_path, binary_path = sys.argv[1:]
stage = Path(stage)
relative_binary = f"../build/release/{contract}"
try:
    recipe = Path(recipe_path).read_bytes()
    config = tomllib.loads(recipe.decode("utf-8"))
    cells = config["cells"]
    if (
        len(cells) != 1
        or cells[0]["name"] != contract
        or cells[0]["location"] != {"file": relative_binary}
        or config.get("dep_groups")
    ):
        raise ValueError("recipe must deploy only the approved legacy binary")

    record = json.loads((stage / "record.json").read_bytes())
    expected_hash, = (
        cell["data_hash"] for cell in record["cell_recipes"] if cell["name"] == contract
    )
    binary = Path(binary_path).read_bytes()
    digest = hashlib.blake2b(binary, digest_size=32, person=b"ckb-default-hash").hexdigest()
    if not binary.startswith(b"\x7fELF") or f"0x{digest}" != expected_hash:
        raise ValueError(f"{binary_path}: expected legacy ELF with data hash {expected_hash}")

    # Give ckb-cli the exact recipe and bytes checked above, even if the
    # operator rebuilds the legacy checkout while signing is in progress.
    (stage / "deployment").mkdir()
    (stage / "deployment/recipe.toml").write_bytes(recipe)
    staged_binary = stage / "build/release" / contract
    staged_binary.parent.mkdir(parents=True)
    staged_binary.write_bytes(binary)
except (OSError, ValueError, KeyError, TypeError) as error:
    raise SystemExit(f"Legacy deployment verification failed: {error}") from error
PY

migration_dir="$deployment_dir/migration/$network/$contract"
info_file="$stage/transaction.json"
cd "$stage/deployment"
ckb-cli --url "$rpc_url" deploy gen-txs --from-address "$address" --fee-rate 1000 \
    --deployment-config "$PWD/recipe.toml" --info-file "$info_file" \
    --migration-dir "$migration_dir" --sign-now

# A failed submission can still require recovery from the signed transaction.
trap - EXIT
if ! python3 "$deployment_dir/verify-chain.py" "$network" "$rpc_url"; then
    fail "Destination chain check failed. Signed transaction retained at $info_file"
fi
if ! ckb-cli --url "$rpc_url" deploy apply-txs --info-file "$info_file" --migration-dir "$migration_dir"; then
    fail "Deployment failed. Signed transaction retained at $info_file"
fi
rm -rf -- "$stage"
echo "Deployment finished"

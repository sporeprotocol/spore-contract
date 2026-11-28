"""Verify the destination network before legacy deployment."""

import json
import subprocess
import sys


CHAINS = {
    "mainnet": (
        "ckb",
        "0x92b197aa1fba0f63633922c61c92375c9c074a93e85963554f5499fe1450d0e5",
    ),
    "testnet": (
        "ckb_testnet",
        "0x10639e0895502b5688a6be8cf69460d76541bfa4821629d86d62ba0aae3f9606",
    ),
}


def rpc(url, method, params):
    query = {"jsonrpc": "2.0", "id": 1, "method": method, "params": params}
    try:
        response = subprocess.check_output(
            ["curl", "--disable", "--silent", "--fail", "--max-time", "10",
             "--max-filesize", "65536", "--proto", "=http,https", "--header",
             "Content-Type: application/json", "--data-binary", json.dumps(query),
             "--write-out", "\n%{http_code}", "--url", url],
        )
    except subprocess.CalledProcessError as error:
        raise ValueError(f"{method} request failed") from error
    body, status = response.rsplit(b"\n", 1)
    if status != b"200":
        raise ValueError(f"unexpected HTTP status from {method}")
    result = json.loads(body)
    if result.get("jsonrpc") != "2.0" or result.get("id") != 1 or "error" in result:
        raise ValueError(f"invalid {method} response")
    return result["result"]


network, url = sys.argv[1:]
try:
    chain, genesis = CHAINS[network]
    if (
        rpc(url, "get_blockchain_info", [])["chain"] != chain
        or rpc(url, "get_block_hash", ["0x0"]) != genesis
    ):
        raise ValueError(f"RPC does not identify the expected {network} chain")
except (OSError, ValueError, AttributeError, TypeError, KeyError) as error:
    raise SystemExit(f"Legacy deployment network verification failed: {error}") from error

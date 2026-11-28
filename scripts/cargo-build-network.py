"""Print the network selected by Cargo for the staged Spore executable."""
import json
import sys

for line in sys.stdin:
    if not line.startswith("{"):
        continue
    message = json.loads(line)
    if message.get("reason") == "compiler-artifact":
        target = message["target"]
        if target["name"] == "spore" and target["kind"] == ["bin"]:
            print("mainnet" if "release_export" in message["features"] else "testnet")
            break
else:
    sys.exit("Cargo did not report the Spore executable")

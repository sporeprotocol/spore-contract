"""Use a local JSON-RPC stub to exercise the pre-signing network gate."""

from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os
from pathlib import Path
import subprocess
import tempfile
from threading import Thread
import unittest


DEPLOY = Path(__file__).with_name("deploy.sh")
TESTNET_GENESIS = "0x10639e0895502b5688a6be8cf69460d76541bfa4821629d86d62ba0aae3f9606"


class DeploymentNetworkTest(unittest.TestCase):
    def test_network_gate(self):
        cases = [
            ("ckb", TESTNET_GENESIS, False),
            ("ckb_dev", TESTNET_GENESIS, False),
            ("ckb_testnet", "0x" + "00" * 32, False),
            ("ckb_testnet", TESTNET_GENESIS, True),
        ]
        for chain, genesis, accepted in cases:
            with self.subTest(chain=chain, genesis=genesis):
                self.check_network_gate(chain, genesis, accepted)

    def check_network_gate(self, chain, genesis, accepted):
        class RpcStub(BaseHTTPRequestHandler):
            def do_POST(self):
                request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                if request["method"] == "get_blockchain_info":
                    result = {"chain": chain}
                else:
                    assert request["method"] == "get_block_hash"
                    assert request["params"] == ["0x0"]
                    result = genesis
                self.send_response(200)
                self.end_headers()
                self.wfile.write(json.dumps({
                    "jsonrpc": "2.0", "id": request["id"], "result": result,
                }).encode())

            def log_message(self, *args):
                pass

        with tempfile.TemporaryDirectory() as tmp, HTTPServer(("127.0.0.1", 0), RpcStub) as server:
            directory = Path(tmp)
            signer = directory / "ckb-cli"
            called = directory / "signer-called"
            signer.write_text('#!/bin/sh\ntouch "$SIGNER_CALLED"\nexit 1\n')
            signer.chmod(0o700)
            env = {**os.environ, "PATH": f"{directory}{os.pathsep}{os.environ['PATH']}",
                   "SIGNER_CALLED": str(called)}
            env.pop("SPORE_LEGACY_CHECKOUT", None)
            thread = Thread(target=server.serve_forever)
            thread.start()
            try:
                run = subprocess.run([
                    DEPLOY, "cluster", f"http://127.0.0.1:{server.server_port}/", "fixture",
                ], env=env, capture_output=True, text=True, timeout=15)
            finally:
                server.shutdown()
                thread.join()
            self.assertNotEqual(run.returncode, 0)
            self.assertFalse(called.exists())
            if accepted:
                self.assertIn("set SPORE_LEGACY_CHECKOUT", run.stderr)
            else:
                self.assertIn("RPC does not identify the expected testnet chain", run.stderr)


if __name__ == "__main__":
    unittest.main()

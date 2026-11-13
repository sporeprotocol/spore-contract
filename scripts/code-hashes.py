#!/usr/bin/env python3
"""Print CKB-personalized Blake2b-256 hashes for staged contract binaries."""
import hashlib
from pathlib import Path
import sys

root = Path(sys.argv[1] if len(sys.argv) > 1 else "build")
paths = sorted(path for path in root.rglob("*") if path.is_file())
if not paths:
    raise SystemExit(f"no contract binaries found in {root}")
for path in paths:
    digest = hashlib.blake2b(
        path.read_bytes(), digest_size=32, person=b"ckb-default-hash"
    ).hexdigest()
    print(f"{path}: 0x{digest}")

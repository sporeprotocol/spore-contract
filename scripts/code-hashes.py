#!/usr/bin/env python3
"""Print CKB-personalized hashes for complete sets of staged contract ELFs."""
import hashlib
import json
from pathlib import Path
import sys

MANIFEST = "code-type-ids.json"
REQUIRED_ROLES = {"spore", "cluster", "mutant"}
OPTIONAL_ROLES = {"cluster_agent", "cluster_proxy"}


def contract_paths(profile):
    try:
        plan = json.loads((profile / MANIFEST).read_text())
        roles = set(plan["type_ids"])
        if plan["format"] != 1 or not REQUIRED_ROLES <= roles <= REQUIRED_ROLES | OPTIONAL_ROLES:
            raise ValueError("invalid code Type ID plan")
    except (OSError, KeyError, TypeError, ValueError) as error:
        raise SystemExit(f"{profile}: {error}") from error

    names = {"spore", "cluster", "spore_extension_rust", "mutant_equipment", "mutant_probe"}
    names.update(roles & OPTIONAL_ROLES)
    return [profile / name for name in sorted(names)]


def code_hash(path):
    try:
        data = path.read_bytes()
    except OSError as error:
        raise SystemExit(f"{path}: {error}") from error
    if not data.startswith(b"\x7fELF"):
        raise SystemExit(f"{path}: expected contract ELF")
    digest = hashlib.blake2b(data, digest_size=32, person=b"ckb-default-hash").hexdigest()
    return f"{path}: 0x{digest}"


root = Path(sys.argv[1] if len(sys.argv) > 1 else "build")
profiles = [root] if (root / MANIFEST).is_file() else [
    root / name for name in ("debug", "release") if (root / name).is_dir()
]
if not profiles:
    raise SystemExit(f"no staged contract profiles found in {root}")

hashes = [code_hash(path) for profile in profiles for path in contract_paths(profile)]
print("\n".join(hashes))

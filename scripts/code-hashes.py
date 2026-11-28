#!/usr/bin/env python3
"""Print CKB-personalized hashes for complete sets of staged contract ELFs."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import sys

from type_id_plan import validate

MANIFEST = "code-type-ids.json"
PROFILE_NAMES = ("debug", "release")


def contract_paths(profile):
    try:
        plan = json.loads((profile / MANIFEST).read_text())
        validate(plan)
    except (OSError, KeyError, TypeError, ValueError) as error:
        raise SystemExit(f"{profile}: {error}") from error

    names = {"spore", "cluster", "spore_extension_rust", "mutant_equipment", "mutant_probe"}
    names.update(plan["type_ids"].keys() & {"cluster_agent", "cluster_proxy"})
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


def collect_hashes(root):
    # Pin one generation from profile discovery through the last ELF read.
    resolved = root.resolve()
    build_dir = resolved.parent if resolved.name in PROFILE_NAMES else resolved
    try:
        lock = os.open(build_dir, os.O_RDONLY | os.O_DIRECTORY)
    except (FileNotFoundError, NotADirectoryError):
        raise SystemExit(f"no staged contract profiles found in {root}") from None
    try:
        fcntl.flock(lock, fcntl.LOCK_SH)
        profiles = [root] if (root / MANIFEST).is_file() else [
            root / name for name in PROFILE_NAMES if (root / name).is_dir()
        ]
        if not profiles:
            raise SystemExit(f"no staged contract profiles found in {root}")
        return [code_hash(path) for profile in profiles for path in contract_paths(profile)]
    finally:
        os.close(lock)


root = Path(sys.argv[1] if len(sys.argv) > 1 else "build")
print("\n".join(collect_hashes(root)))

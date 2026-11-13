#!/usr/bin/env python3
"""Stage the explicit code Type IDs used by a VM build, not public presets."""
import json
import pathlib
import re
import sys
import tomllib

plan = tomllib.loads(pathlib.Path(sys.argv[1]).read_text())
roles = {"spore", "cluster", "cluster_proxy", "cluster_agent", "mutant"}
if (
    set(plan) != {"format", "profile", "type_ids"}
    or plan.get("format") != 1
    or not isinstance(plan.get("profile"), str)
    or not plan["profile"]
    or set(plan.get("type_ids", {})) != roles
):
    raise SystemExit("invalid code Type ID plan")
values = list(plan["type_ids"].values())
if any(not isinstance(v, str) or not re.fullmatch(r"(?:0x)?[0-9a-fA-F]{64}", v) for v in values):
    raise SystemExit("code Type IDs must be 32-byte hex values")
ids = [bytes.fromhex(value.removeprefix("0x")) for value in values]
if len(set(ids)) != len(ids):
    raise SystemExit("code Type IDs must be distinct")
pathlib.Path(sys.argv[2]).write_text(json.dumps(plan, sort_keys=True, indent=2) + "\n")

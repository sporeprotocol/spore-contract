#!/usr/bin/env python3
"""Stage the explicit code Type IDs used by a VM build, not public presets."""
import json
import pathlib
from runpy import run_path
import sys
import tomllib

validate = run_path(str(pathlib.Path(__file__).resolve().with_name("type_id_plan.py")))["validate"]

plan = tomllib.loads(pathlib.Path(sys.argv[1]).read_text())
try:
    validate(plan)
except ValueError as error:
    raise SystemExit(str(error)) from error
pathlib.Path(sys.argv[2]).write_text(json.dumps(plan, sort_keys=True, indent=2) + "\n")
print(" ".join(role for role in ("cluster_proxy", "cluster_agent") if role in plan["type_ids"]))

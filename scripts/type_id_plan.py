"""Validate Type ID plans before staging and before hashing their artifacts."""

import re

REQUIRED_ROLES = {"spore", "cluster", "mutant", "cluster_proxy", "cluster_agent"}
OPTIONAL_ROLES = set()


def validate(plan):
    if (
        not isinstance(plan, dict)
        or set(plan) != {"format", "profile", "type_ids"}
        or type(plan["format"]) is not int
        or plan["format"] != 1
        or not isinstance(plan["profile"], str)
        or not plan["profile"]
        or not isinstance(plan["type_ids"], dict)
        or not REQUIRED_ROLES <= set(plan["type_ids"]) <= REQUIRED_ROLES | OPTIONAL_ROLES
    ):
        raise ValueError("invalid code Type ID plan")
    values = plan["type_ids"].values()
    if any(not isinstance(v, str) or not re.fullmatch(r"(?:0x)?[0-9a-fA-F]{64}", v) for v in values):
        raise ValueError("code Type IDs must be 32-byte hex values")
    ids = {bytes.fromhex(value.removeprefix("0x")) for value in values}
    if len(ids) != len(values):
        raise ValueError("code Type IDs must be distinct")

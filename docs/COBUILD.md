# Optional CoBuild transaction intent

This is the intent-decoupling part of **Spore v3 / `0.3.0-alpha.1`
(unreleased)**. The final candidate also requires the native-only Rust executor
from #75. This PR stays based on upstream master for a focused CoBuild review;
its standalone Lua tests describe that intermediate tree, not v3 runtime support.
See the [shared release plan](./VERSIONS.md#spore-v3--030-alpha1-unreleased).

## Consensus boundary

This source version removes mandatory CoBuild Action extraction from the four
core type scripts. A missing, malformed, duplicate, or misleading Action is not
a reason for these scripts to reject an otherwise valid transaction. They do
not authenticate or certify an attached Action, even when one is present.

The eleven affected operations are Spore mint/transfer/burn, Cluster
mint/transfer, Proxy mint/transfer/burn, and Agent mint/transfer/burn. Cluster
burn remains forbidden. The cell schemas are unchanged.

| Validator | Retained responsibility |
| --- | --- |
| Spore | Type ID on mint, single-cell group rules, immutable transfer data, MIME/content checks, immortality, Cluster authority, declared Mutants and payments |
| Cluster | Type ID on mint, single-cell group rules, nonempty name, immutable transfer data, declared Mutant dependency, forbidden burn |
| Proxy | Type ID on mint, single-cell group rules, Cluster reference and owner authority, immutable Cluster ID |
| Agent | Single-cell group rules, recognized Proxy reference, matching Cluster ID, Proxy authority/payment and conflict rules, immutable Proxy hash |
| Input Lock | The Lock's own authorization, signature, and witness rules |
| Mutant | The declared extension's own validation rules and witness requirements |

The existing Mutant executor/control flow is unchanged in this patch. Returning
from Mutant execution and native Rust support are handled separately in
[PR #75](https://github.com/sporeprotocol/spore-contract/pull/75). In particular,
this patch does not claim to repair upstream's `exec_cell` continuation behavior.

## Explicit cell structure

Some structure constraints used to be implicit in comparisons with Action
`Byte32` fields. They are now checked directly:

- Spore and Cluster script arguments contain at least the 32-byte ID prefix.
- Proxy arguments contain a 32-byte ID, optionally followed by the existing
  8-byte payment field. Proxy data contains exactly one 32-byte Cluster ID.
- Agent arguments contain exactly the 32-byte Cluster ID. Agent data contains
  exactly the 32-byte Proxy type-script hash, and the referenced Proxy's Cluster
  ID is also checked for that size.

The Agent test fixtures now use the mintable 32-byte argument format instead of
an inapplicable Proxy-style payment suffix. New error variants use unused enum
values; existing error numbers are preserved.

## Optional tooling

The Molecule Action schemas remain in `spore-types`. Legacy on-chain parsing
and address-comparison helpers remain available to custom consumers through an
explicit feature:

```toml
spore-utils = { path = "../lib/utils", features = ["cobuild"] }
```

This enables `co_build_types`, `extract_spore_action`, and
`check_spore_address`. Default core contract dependencies do not enable the
feature or link the CoBuild parser. The repository's transaction-construction
tests opt in and continue producing existing CoBuild messages. No SDK or wallet
implementation is changed by this patch.

Before displaying or authorizing an intent, wallets and transaction builders
must independently derive the operation from resolved inputs and outputs, then
compare the Action's kind, asset identity, data hash, and complete `from`/`to`
Lock scripts as applicable. Reject mismatches or show the actual transaction
without trusting the Action. A valid signature alone does not prove that a
signed Action truthfully describes its cells.

Do not strip or rewrite witnesses globally. A CoBuild-aware Lock, custom
Mutant, or old-version type-script group in the same transaction may still
require them. Finalize any optional Message before signing according to the
chosen Lock's rules. The standard sighash Lock regression test confirms that
changing an appended Message after signing invalidates that signature.

## Deployment and migration

This is a consensus change for the combined v3 candidate, not an update to existing
assets. Deploy new binaries under new code hashes, rebuild dependent contracts
in dependency order, and explicitly select the resulting version in clients.
Existing frozen deployments and code-hash records are left unchanged. Cells
bound to an old code hash still require the old rules; editing this repository
does not migrate them or remove those requirements. A transaction mixing old
and new versions must satisfy each group's rules.

## Regression coverage

`tests/src/tests/cobuild_decoupling.rs` exercises all eleven operations without
Actions and with opaque witnesses under an AlwaysSuccess fixture Lock. It also
covers immutable transfer data, forbidden Cluster burn, invalid identity and
duplicate outputs, explicit Proxy/Agent shapes, missing Cluster/Proxy references,
Cluster-owner authorization, Proxy payments, and rejecting Lua Mutants.

Existing Action-bearing tests remain. A separate test uses the actual bundled
`secp256k1_blake160_sighash_all` binary and SDK signing, with and without a
CoBuild Message: unsigned and corrupt signatures fail; signed transactions
pass; recipient or Message tampering after signing fails. These checks cover
that Lock's behavior, not every custom CoBuild-aware Lock or wallet.

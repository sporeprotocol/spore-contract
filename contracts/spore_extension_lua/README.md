# Historical Lua Mutant interpreter

This source and the bundled Lua library are retained for historical reference.
The package is excluded from the current workspace and Capsule contract list;
the default build does not build or stage either binary. New Spore and cluster
contracts do not recognize Lua Mutants.

Existing assets that reference an older Spore binary by data hash continue to
use that binary and its original dependency requirements. Editing this repository
does not upgrade those assets or change their deployed code cells. Deployment
records and frozen hashes are preserved.

The isolated [Lua benchmark baseline](../../benchmarks/legacy-lua/README.md)
uses a fixed earlier commit. It is not a compatibility feature in the new
contracts. For new rules, use the [Rust Mutant guide](../../examples/mutant-rust/README.md).

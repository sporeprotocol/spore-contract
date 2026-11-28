//! VM integration fixture. The Mutant lock args select a failure mode.
#![no_std]
#![no_main]

use ckb_std::{ckb_constants::Source, default_alloc, high_level::load_cell_lock};
use spore_mutant::{Invocation, Operation};

ckb_std::entry!(program_entry);
default_alloc!();

fn program_entry() -> i8 {
    let args: alloc::vec::Vec<_> = ckb_std::env::argv().iter().map(|a| a.to_bytes()).collect();
    let Ok(call) = Invocation::parse(&args) else {
        return 100;
    };
    let source = if call.operation == Operation::Create {
        Source::Output
    } else {
        Source::CellDep
    };
    let Ok(lock) = load_cell_lock(call.mutant_index, source) else {
        return 101;
    };
    let args = lock.args().raw_data();
    let op = match call.operation {
        Operation::Create => b"create".as_slice(),
        Operation::Mint => b"mint",
        Operation::Transfer => b"transfer",
        Operation::Burn => b"burn",
    };
    if args.as_ref() == op {
        return -42;
    }
    if call.operation != Operation::Create && args.as_ref() == b"loop" {
        loop {
            core::hint::spin_loop();
        }
    }
    0
}

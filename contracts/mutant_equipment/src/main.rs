//! A native Rust Mutant: certify a three-byte equipment merge recipe at mint.
#![no_std]
#![no_main]

use ckb_std::{
    ckb_constants::Source, ckb_types::prelude::*, default_alloc, syscalls::load_cell_data,
};
use spore_mutant::{Invocation, Operation};
use spore_types::generated::spore::SporeDataReader;

ckb_std::entry!(program_entry);
default_alloc!();

fn validate() -> Result<(), i8> {
    let args: alloc::vec::Vec<_> = ckb_std::env::argv()
        .iter()
        .map(|arg| arg.to_bytes())
        .collect();
    let invocation = Invocation::parse(&args).map_err(|_| 100i8)?;
    match invocation.operation {
        Operation::Create | Operation::Transfer | Operation::Burn => Ok(()),
        Operation::Mint => {
            let mut buffer = [0u8; 4096];
            let size = load_cell_data(
                &mut buffer,
                0,
                invocation.output_index.ok_or(100i8)?,
                Source::Output,
            )
            .map_err(|_| 101i8)?;
            let spore =
                SporeDataReader::from_compatible_slice(&buffer[..size]).map_err(|_| 102i8)?;
            let content = spore.content().raw_data();
            if content.len() != 3
                || content[0] == 0
                || content[1] == 0
                || content[0].checked_add(content[1]) != Some(content[2])
            {
                return Err(103);
            }
            Ok(())
        }
    }
}

fn program_entry() -> i8 {
    match validate() {
        Ok(()) => 0,
        Err(code) => code,
    }
}

//! Immutable native Mutant code cells, with creation-time execution in CKB-VM V2.
#![no_std]
#![no_main]

use ckb_std::{
    ckb_constants::Source::{GroupInput, GroupOutput, Output},
    default_alloc,
    error::SysError,
    high_level::{load_cell_data_hash, load_cell_type, load_script, QueryIter},
    syscalls::load_cell_data,
};
use spore_errors::error::Error;
use spore_mutant::{Invocation, Operation};
use spore_utils::{find_position_by_type, verify_type_id};

ckb_std::entry!(program_entry);
default_alloc!();

fn validate() -> Result<(), Error> {
    let script = load_script()?;
    if script.hash_type() != spore_mutant::TYPE_CODE_HASH_TYPE.into() {
        return Err(Error::NativeMutantRequiresVm2);
    }
    if !matches!(script.args().raw_data().len(), 32 | 40) {
        return Err(Error::InvalidExtensionArg);
    }
    let inputs = QueryIter::new(load_cell_type, GroupInput).count();
    let outputs = QueryIter::new(load_cell_type, GroupOutput).count();
    match (inputs, outputs) {
        (0, 1) => {
            let index = find_position_by_type(&script, Output).ok_or(Error::IndexOutOfBound)?;
            verify_type_id(index).ok_or(Error::InvalidExtensionID)?;
            // Bound code size and reject non-RISC-V/64-bit/little-endian ELF files.
            // Spawn subsequently validates and executes the actual ELF.
            let mut header = [0u8; 64];
            let size = match load_cell_data(&mut header, 0, index, Output) {
                Ok(n) | Err(SysError::LengthNotEnough(n)) => n,
                Err(e) => return Err(e.into()),
            };
            if !(64..=256 * 1024).contains(&size)
                || &header[..7] != b"\x7fELF\x02\x01\x01"
                || header[16..20] != [2, 0, 243, 0]
            {
                return Err(Error::InvalidNativeMutantCode);
            }
            Invocation {
                operation: Operation::Create,
                mutant_index: index,
                input_index: None,
                output_index: None,
            }
            .execute()
            .map_err(|_| Error::NativeMutantRejected)?;
            Ok(())
        }
        (1, 1) => {
            if load_cell_data_hash(0, GroupInput)? != load_cell_data_hash(0, GroupOutput)? {
                return Err(Error::ModifyExtensionPermanentField);
            }
            Ok(())
        }
        _ => Err(Error::InvalidExtensionOperation),
    }
}

fn program_entry() -> i8 {
    match validate() {
        Ok(()) => 0,
        Err(e) => e as i8,
    }
}

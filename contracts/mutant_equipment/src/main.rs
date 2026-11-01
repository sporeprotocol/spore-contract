//! Opt-in recipe validator, executed by CKB as a separate type-script group.
#![no_std]
#![no_main]

use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::*,
    default_alloc,
    error::SysError,
    high_level::{load_cell_type_hash, load_script},
    syscalls::load_cell_data,
};
use spore_types::generated::spore::SporeDataReader;

ckb_std::entry!(program_entry);
default_alloc!();

#[repr(i8)]
enum Error {
    Arguments = 5,
    Receipt = 6,
    Target = 7,
    SporeData = 8,
    Recipe = 9,
    Syscall = 10,
}

impl From<SysError> for Error {
    fn from(_: SysError) -> Self {
        Self::Syscall
    }
}

fn validate() -> Result<(), Error> {
    let script = load_script()?;
    let target = script.args().raw_data();
    if target.len() != 32 {
        return Err(Error::Arguments);
    }

    // Receipts have no stored state and may be discarded. Every output, even a
    // recreated receipt, must validate the target again. One output per target.
    match load_cell_type_hash(0, Source::GroupOutput) {
        Err(SysError::IndexOutOfBound) => return Ok(()),
        Err(err) => return Err(err.into()),
        Ok(_) => {}
    }
    if load_cell_type_hash(1, Source::GroupOutput) != Err(SysError::IndexOutOfBound)
        || load_cell_data(&mut [], 0, 0, Source::GroupOutput) != Ok(0)
    {
        return Err(Error::Receipt);
    }

    // Bound both the search and data allocation. No dynamic loading, exec,
    // subprocesses, recursion, or witness-controlled executable code.
    for index in 0..256 {
        match load_cell_type_hash(index, Source::Output) {
            Ok(Some(hash)) if hash.as_slice() == target.as_ref() => {
                let mut buffer = [0u8; 4096];
                let size = load_cell_data(&mut buffer, 0, index, Source::Output)
                    .map_err(|_| Error::SporeData)?;
                let spore = SporeDataReader::from_compatible_slice(&buffer[..size])
                    .map_err(|_| Error::SporeData)?;
                let content = spore.content().raw_data();
                // A recipe certificate: two positive byte levels and their sum.
                // It does not claim that any equipment inputs were consumed.
                if content.len() != 3
                    || content[0] == 0
                    || content[1] == 0
                    || content[0].checked_add(content[1]) != Some(content[2])
                {
                    return Err(Error::Recipe);
                }
                return Ok(());
            }
            Ok(_) => {}
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => return Err(err.into()),
        }
    }
    Err(Error::Target)
}

fn program_entry() -> i8 {
    match validate() {
        Ok(()) => 0,
        Err(err) => err as i8,
    }
}

// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::vec::Vec;

// Import CKB syscalls and structures
// https://docs.rs/ckb-std/
use ckb_std::{
    ckb_types::prelude::*,
    ckb_constants::Source,
    error::SysError,
    high_level::{load_cell_data, load_cell_type, load_cell_type_hash, load_script_hash},
};

use crate::error::Error;

use spore_types::generated::spore_types::ClusterData;
use spore_utils::verify_type_id;

fn process_input(
    index: usize,
    input_source: Source,
    group_cell_in_outputs: &mut Vec<usize>,
    output_source: Source,
) -> Result<(), Error> {
    let group_id = load_cell_type(index, input_source)?
        .unwrap_or_default()
        .args();

    for i in 0..group_cell_in_outputs.len() {
        let output_index = group_cell_in_outputs.get(i).unwrap();
        let output_group_id = load_cell_type(*output_index, output_source)?
            .unwrap_or_default()
            .args();

        if group_id.as_slice()[..] == output_group_id.as_slice()[..] {

            let group_data = load_group_data(index, input_source)?;
            let output_group_data = load_group_data(i, output_source)?;

            if group_data.name().as_slice()[..] != output_group_data.name().as_slice()[..] {
                return Err(Error::ModifyPermanentField);
            }

            group_cell_in_outputs.remove(i);
            return Ok(())
        }
    }

    // can not destroy a group cell now
    Err(Error::InvalidOperation)
}

fn load_group_data(index: usize, source: Source) -> Result<ClusterData, Error> {
    let raw_data = load_cell_data(index, source)?;
    let group_data = ClusterData::from_slice(raw_data.as_slice()).map_err(|_| Error::InvalidGroupData)?;
    Ok(group_data)
}

fn process_creation(index: usize, source: Source) -> Result<(), Error> {
    let group_data = load_group_data(index, source)?;

    if group_data.name().is_empty() {
        return Err(Error::EmptyName);
    }

    if !verify_type_id(index, source) {
        return Err(Error::InvalidGroupID);
    }

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let group_type = load_script_hash()?;

    let mut group_cell_in_outputs: Vec<usize> = Vec::new();

    for i in 0.. {
        let script_hash = match load_cell_type_hash(i, Source::GroupOutput) {
            Ok(script_hash) => script_hash,
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => return Err(err.into()),
        };

        if script_hash.unwrap_or_default() != group_type {
            continue;
        }

        group_cell_in_outputs.push(i);
    }

    // go through inputs

    for i in 0.. {
        let script_hash = match load_cell_type_hash(i, Source::GroupInput) {
            Ok(script_hash) => script_hash,
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => return Err(err.into()),
        };

        if script_hash.unwrap_or_default() != group_type {
            continue;
        }

        // information update
        process_input(i, Source::GroupInput, &mut group_cell_in_outputs, Source::GroupOutput)?;
    }

    if !group_cell_in_outputs.is_empty() {
        // creation
        for index in group_cell_in_outputs {
            process_creation(index, Source::GroupOutput)?;
        }

    }

    Ok(())
}




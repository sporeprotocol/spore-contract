// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::vec::Vec;

// Import CKB syscalls and structures
// https://docs.rs/ckb-std/
use ckb_std::{ckb_types::prelude::*, ckb_constants::Source, error::SysError, high_level::{load_cell_data, load_cell_type, load_cell_type_hash, load_script_hash}, debug};
use ckb_std::ckb_types::core::ScriptHashType;
use ckb_std::high_level::QueryIter;

use crate::error::Error;

use spore_types::generated::spore_types::ClusterData;
use spore_utils::{type_hash_filter_builder, verify_type_id};

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
            debug!("Line35!!!");
            let output_group_data = load_group_data(i, output_source)?;
            debug!("Line37!!!");

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
    let group_data = ClusterData::from_slice(raw_data.as_slice()).map_err(|_| Error::InvalidClusterData)?;
    Ok(group_data)
}

fn process_creation(index: usize, source: Source) -> Result<(), Error> {
    let group_data = load_group_data(index, source)?;
    debug!("Line60!!!");

    if group_data.name().is_empty() {
        return Err(Error::EmptyName);
    }

    if !verify_type_id(index, source) {
        return Err(Error::InvalidClusterID);
    }

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let cluster_hash = load_script_hash()?;

    let filter_for_cluster_type = type_hash_filter_builder(cluster_hash, ScriptHashType::Data1);

    let mut group_cell_in_outputs =
        QueryIter::new(load_cell_type, Source::GroupOutput)
        .enumerate()
        .filter(|(_,script_hash)| filter_for_cluster_type(script_hash))
        .map(|(pos,_)| pos).collect();

    // go through inputs

    QueryIter::new(load_cell_type, Source::GroupInput)
        .enumerate()
        .filter(|(_, script)|filter_for_cluster_type(script))
        .map(|(index, _)| index)
        .try_for_each(|index|
            process_input(index, Source::GroupInput, &mut group_cell_in_outputs, Source::GroupOutput)
        )?;

    if !group_cell_in_outputs.is_empty() {
        // creation
        for index in group_cell_in_outputs {
            process_creation(index, Source::GroupOutput)?;
        }
    }

    Ok(())
}




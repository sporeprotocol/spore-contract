// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::vec::Vec;
// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import CKB syscalls and structures
// https://docs.rs/ckb-std/
use ckb_std::ckb_types::core::ScriptHashType;
use ckb_std::high_level::{load_cell_type_hash, QueryIter};
use ckb_std::{ckb_constants::Source, ckb_types::prelude::*, debug, high_level::{load_cell_data, load_cell_type, load_script_hash}};

use spore_types::generated::spore_types::ClusterData;
use spore_utils::{type_hash_filter_builder, verify_type_id};

use crate::error::Error;

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
            return Ok(());
        }
    }

    // can not destroy a group cell now
    Err(Error::InvalidOperation)
}

fn load_group_data(index: usize, source: Source) -> Result<ClusterData, Error> {
    let raw_data = load_cell_data(index, source)?;
    let cluster_data =
        ClusterData::from_slice(raw_data.as_slice()).map_err(|_| Error::InvalidClusterData)?;
    Ok(cluster_data)
}

fn process_creation(index: usize, source: Source) -> Result<(), Error> {
    let group_data = load_group_data(index, source)?;

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

    let hash_filter = type_hash_filter_builder(cluster_hash);

    let mut group_cell_in_outputs = QueryIter::new(load_cell_type_hash, Source::GroupOutput)
        .enumerate()
        .filter(|(_, script_hash)| hash_filter(script_hash) )
        .map(|(pos, _)| pos)
        .collect();

    // go through inputs, looking for cell matched with code hash

    QueryIter::new(load_cell_type_hash, Source::Input)
        .enumerate()
        .filter(|(_, script_hash)| hash_filter(script_hash) )
        .map(|(index, _)| index)
        .try_for_each(|index|
            // process every cluster input
            process_input(index, Source::Input, &mut group_cell_in_outputs, Source::GroupOutput)
        )?;

    if !group_cell_in_outputs.is_empty() {
        // creation
        for index in group_cell_in_outputs {
            // process matched cluster creation cells in output
            process_creation(index, Source::Output)?
        }
    }

    Ok(())
}

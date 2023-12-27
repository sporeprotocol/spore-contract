// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::vec::Vec;
// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

use ckb_std::ckb_constants::Source::{CellDep, GroupInput, GroupOutput, Output};
use ckb_std::ckb_types::packed::Script;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::*,
    high_level::{load_cell_data, load_cell_type},
};
// Import CKB syscalls and structures
// https://docs.rs/ckb-std/
use ckb_std::high_level::{load_script, QueryIter};
use spore_constant::CodeHash::SPORE_EXTENSION_LUA;
use spore_errors::error::Error;
use spore_types::generated::spore_types::ClusterData;
use spore_utils::{find_position_by_type, find_position_by_type_arg, verify_type_id};

#[allow(unused)]
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
            let group_data = load_cluster_data(index, input_source)?;
            let output_group_data = load_cluster_data(i, output_source)?;

            if group_data.name().as_slice()[..] != output_group_data.name().as_slice()[..] {
                return Err(Error::ModifyClusterPermanentField);
            }

            group_cell_in_outputs.remove(i);
            return Ok(());
        }
    }

    // can not destroy a group cell now
    Err(Error::InvalidClusterOperation)
}

fn load_cluster_data(index: usize, source: Source) -> Result<ClusterData, Error> {
    let raw_data = load_cell_data(index, source)?;
    let cluster_data = ClusterData::from_compatible_slice(raw_data.as_slice())
        .map_err(|_| Error::InvalidClusterData)?;
    Ok(cluster_data)
}

fn process_creation(index: usize) -> Result<(), Error> {
    let cluster_data = load_cluster_data(index, Output)?;
    if cluster_data.name().is_empty() {
        return Err(Error::EmptyName);
    }
    if !verify_type_id(index, Output) {
        return Err(Error::InvalidClusterID);
    }

    // Verify if mutant is set
    if cluster_data.mutant_id().is_some() {
        let script = load_script().unwrap_or_default();
        let filter_fn: fn(&[u8; 32]) -> bool = |x| -> bool { SPORE_EXTENSION_LUA.contains(x) };
        let args: Vec<u8> = script.args().unpack();
        find_position_by_type_arg(args.as_slice(), CellDep, Some(filter_fn))
            .ok_or(Error::MutantNotInDeps)?;
    }

    Ok(())
}

fn process_transfer() -> Result<(), Error> {
    // check no field was modified
    let input_cluster_data = load_cluster_data(0, GroupInput)?;
    let output_cluster_data = load_cluster_data(0, GroupOutput)?;

    if input_cluster_data.as_slice()[..] != output_cluster_data.as_slice()[..] {
        return Err(Error::ModifyClusterPermanentField);
    }

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let cluster_in_output: Vec<Script> = QueryIter::new(load_cell_type, GroupOutput)
        .map(|script| script.unwrap_or_default())
        .collect();

    if cluster_in_output.len() > 1 {
        // Conflict Creation
        return Err(Error::InvalidClusterOperation);
    }

    let cluster_in_input: Vec<Script> = QueryIter::new(load_cell_type, GroupInput)
        .map(|script| script.unwrap_or_default())
        .collect();

    if cluster_in_input.len() > 1 {
        // Multi-spend
        return Err(Error::InvalidClusterOperation);
    }

    match (cluster_in_input.len(), cluster_in_output.len()) {
        (0, 1) => {
            // find it's index in Source::Output

            let output_index =
                find_position_by_type(cluster_in_output[0].as_slice(), Output).unwrap_or_default(); // Once we entered here, it can't be empty, and use 0 as a fallback position
            return process_creation(output_index);
        }
        // can not destroy a cluster cell
        (1, 0) => {
            return Err(Error::InvalidClusterOperation);
        }
        (1, 1) => {
            return process_transfer();
        }
        _ => unreachable!(),
    }
}

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::vec::Vec;
use spore_types::generated::action;
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
use spore_errors::error::Error;
use spore_types::generated::spore::ClusterDataV2 as ClusterData;
use spore_utils::{
    check_spore_address, extract_spore_action, find_position_by_type, find_position_by_type_args,
    load_self_id, verify_type_id, blake2b_256
};

use crate::hash::SPORE_EXTENSION_LUA;

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
    let Some(cluster_id) = verify_type_id(index) else {
        return Err(Error::InvalidClusterID);
    };

    // Verify if mutant is set
    if cluster_data.mutant_id().is_some() {
        let script = load_script().unwrap_or_default();
        let filter_fn: fn(&[u8; 32]) -> bool = |x| -> bool { SPORE_EXTENSION_LUA.contains(x) };
        let args: Vec<u8> = script.args().unpack();
        find_position_by_type_args(&args, CellDep, Some(filter_fn))
            .ok_or(Error::MutantNotInDeps)?;
    }

    // check co-build action @lyk
    let action::SporeActionUnion::MintCluster(mint) = extract_spore_action()?.to_enum() else {
        return Err(Error::SporeActionMismatch);
    };
    if cluster_id != mint.cluster_id().as_slice()
        || blake2b_256(cluster_data.as_slice()) != mint.data_hash().as_slice()
    {
        return Err(Error::SporeActionFieldMismatch);
    }
    check_spore_address(GroupOutput, mint.to())?;

    Ok(())
}

fn process_transfer() -> Result<(), Error> {
    // check no field was modified
    let input_cluster_data = load_cluster_data(0, GroupInput)?;
    let output_cluster_data = load_cluster_data(0, GroupOutput)?;

    if input_cluster_data.as_slice()[..] != output_cluster_data.as_slice()[..] {
        return Err(Error::ModifyClusterPermanentField);
    }

    // check co-build action @lyk
    let action::SporeActionUnion::TransferCluster(transfer) = extract_spore_action()?.to_enum()
    else {
        return Err(Error::SporeActionMismatch);
    };
    if transfer.cluster_id().as_slice() != &load_self_id()? {
        return Err(Error::SporeActionFieldMismatch);
    }
    check_spore_address(GroupInput, transfer.from())?;
    check_spore_address(GroupOutput, transfer.to())?;

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
                find_position_by_type(&cluster_in_output[0], Output).unwrap_or_default(); // Once we entered here, it can't be empty, and use 0 as a fallback position
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

use alloc::vec::Vec;
use core::result::Result;

// Import from `core` instead of from `std` since we are in no-std mode
use ckb_std::ckb_constants::Source::{CellDep, GroupInput, GroupOutput, Input, Output};
use ckb_std::ckb_types::packed::Script;
use ckb_std::high_level::{
    load_cell_data, load_cell_lock_hash, load_cell_type, load_script, QueryIter,
};

use spore_errors::error::Error;
use spore_utils::{
    find_position_by_lock_hash, find_position_by_type, find_position_by_type_args_matching,
    verify_type_id, CLUSTER_PROXY_ID_LEN, CLUSTER_PROXY_ID_WITH_PAYMENT_LEN,
};

fn is_valid_cluster_cell(script: &Script) -> bool {
    spore_utils::matches_code_identity(
        script,
        &crate::hash::CLUSTER_CODE_HASHES,
        &crate::hash::CLUSTER_TYPE_HASHES,
    )
}

// Cluster IDs were previously constrained by the Action's Byte32 field.
fn load_cluster_id(source: ckb_std::ckb_constants::Source) -> Result<Vec<u8>, Error> {
    let data = load_cell_data(0, source)?;
    if data.len() != 32 {
        return Err(Error::InvalidProxyData);
    }
    Ok(data)
}

fn process_creation(index: usize) -> Result<(), Error> {
    let cluster_id = load_cluster_id(GroupOutput)?;
    // check cluster in Deps
    let cell_dep_index =
        find_position_by_type_args_matching(&cluster_id, CellDep, is_valid_cluster_cell)
            .ok_or(Error::ClusterCellNotInDep)?;

    // verify Proxy ID
    verify_type_id(index).ok_or(Error::InvalidProxyID)?;

    // Condition 1: Check if cluster exist in Inputs & Outputs
    let cluster_cell_in_input =
        find_position_by_type_args_matching(&cluster_id, Input, is_valid_cluster_cell).is_some();
    let cluster_cell_in_output =
        find_position_by_type_args_matching(&cluster_id, Output, is_valid_cluster_cell).is_some();

    if !cluster_cell_in_input || !cluster_cell_in_output {
        // Condition 2: Check if Lock Proxy exist in Inputs & Outputs
        let cluster_lock_hash = load_cell_lock_hash(cell_dep_index, CellDep)?;
        find_position_by_lock_hash(&cluster_lock_hash, Output)
            .ok_or(Error::ClusterOwnershipVerifyFailed)?;
        find_position_by_lock_hash(&cluster_lock_hash, Input)
            .ok_or(Error::ClusterOwnershipVerifyFailed)?;
    }

    Ok(())
}

fn process_transfer() -> Result<(), Error> {
    let input_data = load_cluster_id(GroupInput)?;
    let output_data = load_cluster_id(GroupOutput)?;

    if input_data != output_data {
        return Err(Error::ImmutableProxyFieldModification);
    }

    Ok(())
}

fn process_destruction() -> Result<(), Error> {
    load_cluster_id(GroupInput)?;
    Ok(())
}

pub fn main() -> Result<(), Error> {
    let args = load_script()?.args().raw_data();
    if !matches!(
        args.len(),
        CLUSTER_PROXY_ID_LEN | CLUSTER_PROXY_ID_WITH_PAYMENT_LEN
    ) {
        return Err(Error::InvalidProxyArgs);
    }
    let proxy_in_output: Vec<Script> = QueryIter::new(load_cell_type, GroupOutput)
        .map(|script| script.unwrap_or_default())
        .collect();

    if proxy_in_output.len() > 1 {
        // Conflict Creation/Multiplier
        return Err(Error::InvalidProxyOperation);
    }

    let proxy_in_input: Vec<Script> = QueryIter::new(load_cell_type, GroupInput)
        .map(|script| script.unwrap_or_default())
        .collect();

    if proxy_in_input.len() > 1 {
        // Multi-spend
        return Err(Error::InvalidProxyOperation);
    }

    return match (proxy_in_input.len(), proxy_in_output.len()) {
        (0, 1) => {
            // Creation
            let output_index =
                find_position_by_type(&proxy_in_output[0], Output).unwrap_or_default(); // Once we entered here, it can't be empty, and use 0 as a fallback position
            return process_creation(output_index);
        }
        (1, 0) => process_destruction(),
        (1, 1) => process_transfer(),
        _ => unreachable!(),
    };
}

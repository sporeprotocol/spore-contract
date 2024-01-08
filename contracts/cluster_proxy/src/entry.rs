use alloc::vec::Vec;
// Import from `core` instead of from `std` since we are in no-std mode
use ckb_std::ckb_constants::Source::{CellDep, GroupInput, GroupOutput, Input, Output};
use ckb_std::ckb_types::packed::Script;
use ckb_std::high_level::{load_cell_data, load_cell_lock_hash, load_cell_type, QueryIter};
use core::result::Result;
use spore_errors::error::Error;
use spore_utils::{
    find_position_by_lock_hash, find_position_by_type, find_position_by_type_args, verify_type_id,
};

fn is_valid_cluster_cell(script_hash: &[u8; 32]) -> bool {
    crate::hash::CLUSTER_CODE_HASHES.contains(script_hash)
}

fn process_creation(index: usize) -> Result<(), Error> {
    let target_cluster_id = load_cell_data(0, GroupOutput)?;
    // check cluster in Deps
    let cell_dep_index =
        find_position_by_type_args(&target_cluster_id, CellDep, Some(is_valid_cluster_cell))
            .ok_or(Error::ClusterCellNotInDep)?;

    // verify Proxy ID
    if !verify_type_id(index, Output) {
        return Err(Error::InvalidProxyID);
    }

    // Condition 1: Check if cluster exist in Inputs & Outputs
    return if find_position_by_type_args(&target_cluster_id, Input, Some(is_valid_cluster_cell))
        .is_some()
        && find_position_by_type_args(&target_cluster_id, Output, Some(is_valid_cluster_cell))
            .is_some()
    {
        Ok(())
    } else {
        // Condition 2: Check if Lock Proxy exist in Inputs & Outputs
        let cluster_lock_hash = load_cell_lock_hash(cell_dep_index, CellDep)?;
        find_position_by_lock_hash(&cluster_lock_hash, Output)
            .ok_or(Error::ClusterOwnershipVerifyFailed)?;
        find_position_by_lock_hash(&cluster_lock_hash, Input)
            .ok_or(Error::ClusterOwnershipVerifyFailed)?;
        Ok(())
    };
}

fn process_transfer() -> Result<(), Error> {
    let input_data = load_cell_data(0, GroupInput)?;
    let output_data = load_cell_data(0, GroupOutput)?;

    if input_data != output_data {
        return Err(Error::ImmutableProxyFieldModification);
    }

    Ok(())
}

pub fn main() -> Result<(), Error> {
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
        (1, 0) => Ok(()), // There's no limitation to destroy a proxy except lock
        (1, 1) => process_transfer(),
        _ => unreachable!(),
    };
}
